use gl_render::GLVertex;
use gl_render::atlas::Atlas;
use gl_render::translate::image::ImageTranslate;
use theme::{ThemeText, RescaleRules};

use cgmath::{EuclideanSpace, ElementWise, Point2, Vector2};
use cgmath_geometry::{BoundBox, DimsBox, OffsetBox, Segment, GeoBox};

use gullery::colors::Rgba;
use gullery::glsl::Nu8;

use glyphydog::{ShapedBuffer, ShapedGlyph, Face, FaceSize, DPI, LoadFlags, RenderMode};
use dct::hints::Align;

use unicode_segmentation::UnicodeSegmentation;

use itertools::Itertools;
use std::{cmp, vec};
use std::cmp::Ordering;
use std::ops::Range;
use std::cell::{Ref, RefCell};


pub(in gl_render) struct TextTranslate<'a> {
    glyph_draw: GlyphDraw<'a>,

    rect: BoundBox<Point2<i32>>,
    glyph_slice_index: usize,
    glyph_slice: Ref<'a, [RenderGlyph]>,
    highlight_range: Range<usize>,
    cursor_pos: Option<usize>,
    string_len: usize,

    font_ascender: i32,
    font_descender: i32,

    highlight_vertex_iter: Option<ImageTranslate>,
    glyph_vertex_iter: Option<ImageTranslate>,
    cursor_vertex_iter: Option<ImageTranslate>
}

#[derive(Debug, Clone, Copy)]
pub struct RenderGlyph {
    pos: Point2<i32>,
    highlight_rect: BoundBox<Point2<i32>>,
    str_index: usize,
    glyph_index: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct EditString {
    pub render_string: RenderString,
    pub draw_cursor: bool,
    cursor_pos: usize,
    highlight_range: Range<usize>,
    cursor_target_x_px: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct RenderString {
    string: String,
    cell: RefCell<Option<RenderStringCell>>
}

#[derive(Debug, Clone)]
struct RenderStringCell {
    shaped_glyphs: Vec<RenderGlyph>,
    text_style: ThemeText,
    dpi: DPI,
    draw_rect: BoundBox<Point2<i32>>,
}

struct GlyphDraw<'a> {
    face: &'a mut Face<()>,
    atlas: &'a mut Atlas,
    text_style: ThemeText,
    dpi: DPI
}

struct GlyphIter {
    glyph_items: vec::IntoIter<GlyphItem>,
    v_advance: i32,
    line_start_x: i32,
    run_start_x: i32,
    cursor: Vector2<i32>,
    x_justify: Align,
    active_run: Run,
    on_hard_break: bool,
    whitespace_overflower: OverflowAdd,

    font_ascender: i32,
    font_descender: i32,

    tab_advance: i32,
    bounds_width: i32,
}

/// An item in the list that dictates glyph layout.
///
/// List rules:
/// 1. List must begin with a `Line`.
/// 2. A `Run` must directly follow a `Line`, although `Line`s don't necessarily have to come
///    before a `Run`.
/// 3. `Word`s must be followed by a sequence of `Glyph`s dictated by the `Word`'s `glyph_count`.
///     3a. `Glyph`s can only exist following a `Word`.
/// 4. There may be no consecutive `Whitespace`s.
/// 5. A `Run` is only ended by a `Line` or a `Tab`. Until the next `Run` begins, there may be only
///    `Tab`s and `Line`s.
/// 6. All `advance` values must match the advances obtained in the respecting sections.
#[derive(Debug, PartialEq, Eq)]
enum GlyphItem {
    /// A single shaped glyph. Location is relative to word start.
    Glyph(ShapedGlyph),
    /// A sequence of renderable glyphs.
    Word {
        glyph_count: u32,
        advance: i32,
    },
    /// Non-tabulating whitespace.
    Whitespace {
        glyph_count: u32,
        advance: i32
    },
    WhitespaceGlyph(ShapedGlyph),
    /// Dictates where a new line starts. Contains the horizontal advance of the line,
    /// not including any trailing whitespace.
    Line {
        /// A line's advance, not including trailing whitespace
        advance: i32,
        hard_break: bool
    },
    /// An alternating sequence of words and whitespace that can be spaced and justified as a whole.
    /// The distinction between a `Run` and a `Line` only really matters when doing full
    /// justification, as a `Run` may be ended by a `Tab`.
    Run(Run),
    /// A character that advances the cursor to the next tab stop in the line.
    Tab {
        str_index: usize
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Run {
    /// The advance in the run provided by renderable glyphs.
    glyph_advance: i32,
    /// The amount of whitespace in the run. Does not contain tabs.
    whitespace_advance: i32,
    /// The whitespace at the very end of the line, with no glyphs following.
    trailing_whitespace: i32,
    /// If this run ends the line.
    ends_line: bool
}

impl<'a> TextTranslate<'a> {
    pub fn new_rs<'b, F>(
        rect: BoundBox<Point2<i32>>,
        text_style: ThemeText,
        face: &'a mut Face<()>,
        dpi: DPI,
        atlas: &'a mut Atlas,
        shape_text: F,
        render_string: &'a RenderString
    ) -> TextTranslate<'a>
        where F: FnOnce(&str, &mut Face<()>) -> &'b ShapedBuffer
    {
        Self::new_raw(rect, text_style, face, dpi, atlas, shape_text, render_string, 0..0, None)
    }

    pub fn new_es<'b, F>(
        rect: BoundBox<Point2<i32>>,
        text_style: ThemeText,
        face: &'a mut Face<()>,
        dpi: DPI,
        atlas: &'a mut Atlas,
        shape_text: F,
        edit_string: &'a EditString
    ) -> TextTranslate<'a>
        where F: FnOnce(&str, &mut Face<()>) -> &'b ShapedBuffer
    {
        Self::new_raw(
            rect, text_style, face, dpi, atlas,
            shape_text, &edit_string.render_string,
            edit_string.highlight_range.clone(),
            match edit_string.draw_cursor && edit_string.highlight_range.len() == 0 {
                true => Some(edit_string.cursor_pos),
                false => None
            }
        )
    }

    fn new_raw<'b, F>(
        rect: BoundBox<Point2<i32>>,
        text_style: ThemeText,
        face: &'a mut Face<()>,
        dpi: DPI,
        atlas: &'a mut Atlas,
        shape_text: F,
        render_string: &'a RenderString,
        highlight_range: Range<usize>,
        cursor_pos: Option<usize>,
    ) -> TextTranslate<'a>
        where F: FnOnce(&str, &mut Face<()>) -> &'b ShapedBuffer
    {
        let face_size = FaceSize::new(text_style.face_size, text_style.face_size);
        let font_metrics = face.metrics_sized(face_size, dpi).unwrap();
        let (ascender, descender) = ((font_metrics.ascender / 64) as i32, (font_metrics.descender / 64) as i32);

        TextTranslate {
            rect,
            glyph_slice_index: 0,
            glyph_slice: render_string.reshape_glyphs(rect, shape_text, &text_style, face, dpi),
            glyph_draw: GlyphDraw{ face, atlas, text_style, dpi },
            highlight_range,
            cursor_pos,
            string_len: render_string.string.len(),
            font_ascender: ascender,
            font_descender: descender,
            highlight_vertex_iter: None,
            glyph_vertex_iter: None,
            cursor_vertex_iter: None
        }
    }
}

impl GlyphIter {
    fn new(
        rect: BoundBox<Point2<i32>>,
        shaped_text: &ShapedBuffer,
        text_style: &ThemeText,
        face: &mut Face<()>,
        dpi: DPI
    ) -> GlyphIter
    {
        // TODO: CACHE HEAP ALLOC
        let mut glyph_items = Vec::new();
        let mut segment_index = 0;

        // Compute the tab advance from the tab size and space advance. Used for tab stops.
        let tab_advance = {
            let space_glyph_index = face.char_index(' ');
            let space_glyph_advance = (face.glyph_advance(
                space_glyph_index,
                FaceSize::new(text_style.face_size, text_style.face_size),
                dpi,
                LoadFlags::empty()
            ).unwrap() + (1 << 15)) >> 16;

            space_glyph_advance * text_style.tab_size as i32
        };

        // Initialize the run data and line data.
        let mut run = Run::default();
        let (mut line_advance, mut num_lines) = (0, 0);
        // The places where a `Line` or `Run` should be inserted into `glyph_items`.
        let (mut line_insert_index, mut run_insert_index) = (glyph_items.len(), glyph_items.len());

        while let Some(segment) = shaped_text.get_segment(segment_index) {
            // Create an iterator over every glyph in the segment.
            let mut glyphs = {
                let glyph_with_char = |glyph: ShapedGlyph| (glyph, segment.text[glyph.word_str_index..].chars().next().unwrap());
                segment.shaped_glyphs.iter().cloned().map(glyph_with_char).peekable()
            };
            // Contains information about the segment's advances. How it's handled depends on where the line breaks.
            let mut segment_run = Run::default();
            // The number of `Word` and `Whitespace` items in the segment.
            let mut segment_item_count = 0;

            // Loop over the glyphs, alternating between inserting renderable glyphs and
            // whitespace. First half handles renderable, second half whitespace.
            while glyphs.peek().is_some() {
                // Add sequence of renderable glyphs.

                let word_insert_index = glyph_items.len();
                let (mut glyph_count, mut word_advance) = (0, 0);

                // Continue taking glyphs until we hit whitespace.
                for (glyph, _) in glyphs.peeking_take_while(|&(_, c)| !c.is_whitespace()) {
                    glyph_count += 1;
                    word_advance += glyph.advance.x;
                    glyph_items.push(GlyphItem::Glyph(glyph));
                }
                // If there are glyphs to add, insert a `Word` and increment the advances.
                if glyph_count > 0 {
                    segment_run.trailing_whitespace = 0;

                    segment_item_count += glyph_count as usize;
                    line_advance += word_advance;
                    segment_run.glyph_advance += word_advance;
                    glyph_items.insert(word_insert_index, GlyphItem::Word{ glyph_count, advance: word_advance });
                }

                // Add sequence of whitespace characters.

                let mut whitespace_advance = 0;
                let mut whitespace_glyph_count = 0;
                let mut whitespace_insert_index = glyph_items.len();
                macro_rules! push_whitespace {
                    () => {{
                        if whitespace_advance != 0 {
                            line_advance += whitespace_advance;
                            segment_run = segment_run.append_run(Run::tail_whitespace(whitespace_advance));
                            glyph_items.insert(
                                whitespace_insert_index,
                                GlyphItem::Whitespace {
                                    glyph_count: whitespace_glyph_count,
                                    advance: whitespace_advance
                                }
                            );
                            segment_item_count += 1;
                            #[allow(unused_assignments)]
                            {
                                whitespace_advance = 0;
                                whitespace_glyph_count = 0;
                                whitespace_insert_index = glyph_items.len();
                            }
                        }
                    }}
                }

                for (glyph, c) in glyphs.peeking_take_while(|&(_, c)| c.is_whitespace()) {
                    match c == '\t' {
                        false => {
                            whitespace_glyph_count += 1;
                            segment_item_count += 1;
                            glyph_items.push(GlyphItem::WhitespaceGlyph(glyph));
                            whitespace_advance += glyph.advance.x
                        },
                        // If the whitespace is a tab, push all the accumulated whitespace, begin a
                        // new run and mark off the old run.
                        true => {
                            push_whitespace!();

                            // Move the advance to the next tab stop.
                            line_advance = ((line_advance/tab_advance) + 1) * tab_advance;
                            // If the last thing in `glyph_items` is a tab, then we're in a sequence of `Tab`s
                            // and the `Run` was already inserted by the first tab.
                            match glyph_items.last() {
                                Some(&GlyphItem::Tab{..}) => (),
                                _ => glyph_items.insert(run_insert_index, GlyphItem::Run(run.append_run(segment_run)))
                            }
                            glyph_items.push(GlyphItem::Tab{ str_index: glyph.str_index });
                            run = Run::default();
                            segment_run = Run::default();
                            run_insert_index = glyph_items.len();
                        }
                    }
                }

                push_whitespace!();
            }

            // If the segment hasn't moved the cursor beyond the line's length, append it to the run.
            if line_advance <= rect.width() as i32 {
                run = run.append_run(segment_run);
            }

            if segment.hard_break || line_advance > rect.width() as i32 {
                glyph_items.insert(run_insert_index, GlyphItem::Run(run.ends_line()));

                num_lines += 1;
                match line_advance > rect.width() as i32 {
                    // Happens if the last segment ran over the rectangle length.
                    true => {
                        line_advance -= run.trailing_whitespace + segment_run.advance();
                        glyph_items.insert(line_insert_index, GlyphItem::Line{ advance: line_advance, hard_break: segment.hard_break });

                        line_advance = segment_run.advance();
                        run = segment_run;

                        let insert_index = glyph_items.len() - segment_item_count - 1;
                        line_insert_index = insert_index;
                        run_insert_index = insert_index;
                    },
                    // Happens if we've hit a hard break and the last segment isn't overflowing the rectangle.
                    false => {
                        line_advance -= run.trailing_whitespace;
                        glyph_items.insert(line_insert_index, GlyphItem::Line{ advance: line_advance, hard_break: segment.hard_break });

                        line_advance = 0;
                        run = Run::default();

                        line_insert_index = glyph_items.len();
                        run_insert_index = glyph_items.len();
                    }
                }

            }

            segment_index += 1;
        }

        if run != Run::default() {
            glyph_items.insert(run_insert_index, GlyphItem::Run(run));
        }
        if line_advance != 0 {
            glyph_items.insert(line_insert_index, GlyphItem::Line{ advance: line_advance, hard_break: true });
            num_lines += 1;
        }

        let face_size = FaceSize::new(text_style.face_size, text_style.face_size);

        let font_metrics = face.metrics_sized(face_size, dpi).unwrap();
        let line_height = (font_metrics.height / 64) as i32;
        let (ascender, descender) = ((font_metrics.ascender / 64) as i32, (font_metrics.descender / 64) as i32);

        let v_advance = match text_style.justify.y {
            Align::Stretch => (rect.height() / num_lines) as i32,
            _ => line_height
        };

        GlyphIter {
            glyph_items: glyph_items.into_iter(),
            v_advance,
            cursor: Vector2 {
                x: 0,
                y: match text_style.justify.y {
                    Align::Center => (rect.height() as i32 - (line_height * num_lines as i32)) / 2,
                    Align::End => rect.height() as i32 - (line_height * num_lines as i32),
                    _ => 0
                }
            },
            line_start_x: 0,
            run_start_x: 0,
            x_justify: text_style.justify.x,
            active_run: Run::default(),
            whitespace_overflower: OverflowAdd::default(),

            font_ascender: ascender,
            font_descender: descender,

            on_hard_break: false,
            tab_advance,
            bounds_width: rect.width() as i32,
        }
    }

    fn highlight_rect(&self, glyph_pos: Point2<i32>, glyph_advance: i32) -> BoundBox<Point2<i32>> {
        BoundBox::new2(
            glyph_pos.x, glyph_pos.y - self.font_ascender,
            glyph_pos.x + glyph_advance, glyph_pos.y - self.font_descender
        )
    }
}

const OVERFLOW_SHIFT: i64 = 16;
const OVERFLOW_MASK: i64 = 0b1111111111111111;
#[derive(Debug, Default)]
struct OverflowAdd {
    overflow: i64
}

impl OverflowAdd {
    fn add(&mut self, x: i64) -> i64 {
        let sum = x + self.overflow;
        self.overflow = sum & OVERFLOW_MASK;
        sum
    }
}

impl Iterator for GlyphIter {
    type Item = RenderGlyph;

    fn next(&mut self) -> Option<RenderGlyph> {
        loop {
            match self.glyph_items.next()? {
                GlyphItem::Glyph(glyph) => {
                    let render_glyph = RenderGlyph {
                        pos: Point2::from_vec(self.cursor),
                        highlight_rect: self.highlight_rect(Point2::from_vec(self.cursor), glyph.advance.x),
                        str_index: glyph.str_index,
                        glyph_index: Some(glyph.glyph_index)
                    };

                    self.cursor += glyph.advance.mul_element_wise(Vector2::new(1, -1));
                    return Some(render_glyph);
                },
                GlyphItem::Word{..} => continue,
                GlyphItem::WhitespaceGlyph(glyph) => {
                    let cursor_advance = match self.x_justify == Align::Stretch && !self.on_hard_break {
                        false => glyph.advance.x,
                        true => {
                            let advance_shift = (glyph.advance.x as i64) << OVERFLOW_SHIFT;
                            let fillable_whitespace = match self.active_run.ends_line {
                                false => self.active_run.whitespace_advance as i64,
                                true => (self.bounds_width - self.run_start_x - self.active_run.glyph_advance) as i64
                            };

                            (
                                self.whitespace_overflower.add(
                                    advance_shift / self.active_run.whitespace_advance as i64 *
                                    fillable_whitespace
                                ) >> 16
                            ) as i32
                        }
                    };
                    let render_glyph = RenderGlyph {
                        pos: Point2::from_vec(self.cursor),
                        highlight_rect: self.highlight_rect(Point2::from_vec(self.cursor), cursor_advance),
                        str_index: glyph.str_index,
                        glyph_index: None
                    };
                    self.cursor.x += cursor_advance;

                    return Some(render_glyph);
                },
                GlyphItem::Whitespace{..} => continue,
                GlyphItem::Line{advance, hard_break} => {
                    self.cursor.y += self.v_advance;
                    self.cursor.x = match self.x_justify {
                        Align::Center => (self.bounds_width - advance) / 2,
                        Align::End => self.bounds_width - advance,
                        _ => 0
                    };
                    self.line_start_x = self.cursor.x;
                    self.on_hard_break = hard_break;
                },
                GlyphItem::Run(run) => {
                    self.active_run = run;
                    self.run_start_x = self.cursor.x;
                    continue;
                },
                GlyphItem::Tab{str_index} => {
                    let new_cursor_x = (((self.cursor.x - self.line_start_x)/self.tab_advance) + 1) * self.tab_advance + self.line_start_x;
                    let render_glyph = RenderGlyph {
                        pos: Point2::from_vec(self.cursor),
                        highlight_rect: self.highlight_rect(Point2::from_vec(self.cursor), new_cursor_x - self.cursor.x),
                        str_index,
                        glyph_index: None
                    };
                    self.cursor.x = new_cursor_x;

                    return Some(render_glyph);
                }
            }
        }
    }
}

impl<'a> Iterator for TextTranslate<'a> {
    type Item = GLVertex;

    fn next(&mut self) -> Option<GLVertex> {
        loop {
            fn next_in_iter(i: Option<impl Iterator<Item=GLVertex>>) -> Option<GLVertex> {i.map(|mut v| v.next()).unwrap_or(None)}
            let next_vertex =
                next_in_iter(self.highlight_vertex_iter.as_mut())
                    .or_else(|| next_in_iter(self.glyph_vertex_iter.as_mut()))
                    .or_else(|| next_in_iter(self.cursor_vertex_iter.as_mut()));
            match next_vertex {
                Some(vert) => return Some(vert),
                None => {
                    let TextTranslate {
                        ref glyph_slice,
                        ref mut glyph_slice_index,
                        ref highlight_range,
                        ref mut cursor_pos,
                        ref mut glyph_draw,
                        string_len,
                        font_ascender,
                        font_descender,
                        ref mut glyph_vertex_iter,
                        ref mut highlight_vertex_iter,
                        ref mut cursor_vertex_iter,
                        rect,
                    } = *self;
                    let next_glyph_opt = glyph_slice.get(*glyph_slice_index);

                    *cursor_vertex_iter = cursor_pos.and_then(|pos| {
                        let str_index = next_glyph_opt.map(|g| g.str_index).unwrap_or(0);
                        let highlight_rect_opt = next_glyph_opt.map(|g| g.highlight_rect + rect.min().to_vec());
                        let base_pos = if pos == str_index {
                            highlight_rect_opt.map(|r| r.min()).or(Some(
                                Point2 {
                                    x: match glyph_draw.text_style.justify.x {
                                        Align::Start |
                                        Align::Stretch => 0,
                                        Align::Center => rect.width() as i32 / 2,
                                        Align::End => rect.width()
                                    },
                                    y: match glyph_draw.text_style.justify.y {
                                        Align::Center => rect.height() as i32 / 2,
                                        Align::End => rect.height() as i32,
                                        _ => 0
                                    } - font_descender
                                } + rect.min().to_vec()
                            ))
                        } else if pos == str_index + 1 && pos == string_len {
                            highlight_rect_opt.map(|r| Point2::new(r.max().x, r.min().y))
                        } else {None};

                        base_pos.map(|pos| {
                            *cursor_pos = None;
                            ImageTranslate::new(
                                BoundBox::new(pos, pos + Vector2::new(1, font_ascender - font_descender)),
                                glyph_draw.atlas.white().cast().unwrap_or(OffsetBox::new2(0, 0, 0, 0)),
                                glyph_draw.text_style.color,
                                RescaleRules::StretchOnPixelCenter
                            )
                        })
                    });

                    let next_glyph = match (next_glyph_opt, self.cursor_vertex_iter.is_some()) {
                        (Some(next_glyph), _) => next_glyph,
                        (None, false) => return None,
                        (None, true) => continue
                    };
                    *glyph_slice_index += 1;

                    let is_highlighted = highlight_range.contains(next_glyph.str_index);
                    *glyph_vertex_iter = next_glyph.glyph_index.map(|glyph_index|
                        glyph_draw.glyph_atlas_image(
                            next_glyph.pos,
                            glyph_index,
                            is_highlighted,
                            rect
                        )
                    );


                    let starts_highlight_rect =
                        (
                            highlight_range.start == next_glyph.str_index &&
                            highlight_range.len() > 0
                        ) ||
                        (
                            is_highlighted &&
                            Some(next_glyph.pos.y) != self.glyph_slice.get(*glyph_slice_index - 2).map(|g| g.pos.y)
                        );
                    *highlight_vertex_iter = match starts_highlight_rect {
                        true => {
                            let mut dummy_last_glyph = *self.glyph_slice.last().unwrap();
                            dummy_last_glyph.pos.x += dummy_last_glyph.highlight_rect.width();
                            dummy_last_glyph.highlight_rect.min.x += dummy_last_glyph.highlight_rect.width();
                            dummy_last_glyph.highlight_rect.max.x = dummy_last_glyph.highlight_rect.min.x;
                            dummy_last_glyph.str_index += 1;

                            let highlight_rect_end = self.glyph_slice[*glyph_slice_index..]
                                .iter().cloned().chain(Some(dummy_last_glyph))
                                .take_while(|g| g.pos.y == next_glyph.pos.y)
                                .take_while(|g| g.str_index <= highlight_range.end)
                                .last().unwrap().pos.x;

                            let mut highlight_rect = next_glyph.highlight_rect;
                            highlight_rect.max.x = highlight_rect_end;
                            highlight_rect = highlight_rect + rect.min().to_vec();

                            Some(ImageTranslate::new(
                                highlight_rect,
                                glyph_draw.atlas.white().cast().unwrap_or(OffsetBox::new2(0, 0, 0, 0)),
                                glyph_draw.text_style.highlight_bg_color,
                                RescaleRules::StretchOnPixelCenter
                            ))
                        },
                        false => None
                    };

                    continue;
                }
            }
        }
    }
}

impl Run {
    #[inline]
    fn tail_whitespace(whitespace: i32) -> Run {
        Run {
            glyph_advance: 0,
            whitespace_advance: whitespace,
            trailing_whitespace: whitespace,
            ends_line: false
        }
    }

    #[inline]
    fn ends_line(mut self) -> Run {
        self.ends_line = true;
        self
    }

    #[inline]
    fn append_run(mut self, run: Run) -> Run {
        self.glyph_advance += run.glyph_advance;
        self.whitespace_advance += run.whitespace_advance;
        self.trailing_whitespace = run.trailing_whitespace;
        self
    }

    #[inline]
    fn advance(&self) -> i32 {
        self.glyph_advance + self.whitespace_advance
    }
}

impl<'a> GlyphDraw<'a> {
    fn glyph_atlas_image(&mut self, mut glyph_pos: Point2<i32>, glyph_index: u32, is_highlighted: bool, rect: BoundBox<Point2<i32>>) -> ImageTranslate {
        let GlyphDraw {
            ref mut face,
            ref mut atlas,
            ref text_style,
            dpi,
            ..
        } = *self;

        let face_size = FaceSize::new(text_style.face_size, text_style.face_size);

        let render_mode = RenderMode::Normal;
        let (atlas_rect, glyph_bearing) = atlas.glyph_rect(
            text_style.face.clone(),
            text_style.face_size,
            glyph_index,
            || {
                let glyph_res = face.load_glyph(
                    glyph_index,
                    face_size,
                    dpi,
                    LoadFlags::empty(),
                    render_mode
                ).and_then(|mut glyph_slot| Ok((
                    glyph_slot.render_glyph(render_mode)?,
                    glyph_slot.metrics()
                )));

                match glyph_res {
                    Ok((bitmap, glyph_metrics)) => {
                        assert!(bitmap.pitch >= 0);
                        let (bytes, pitch, dims) = match bitmap.pitch {
                            0 => (&[][..], 1, DimsBox::new2(0, 0)),
                            _ => (bitmap.buffer, bitmap.pitch as usize, bitmap.dims)
                        };
                        (
                            bytes.chunks(pitch)
                                .map(move |b|
                                    Nu8::slice_from_raw(&b[..dims.width() as usize])
                                        // We upload white glyphs to the atlas, which are colored by
                                        // vertex colors.
                                        .into_iter().map(|t| Rgba::new(Nu8(255), Nu8(255), Nu8(255), *t))
                                ),
                            bitmap.dims,
                            glyph_metrics.hori_bearing / 64
                        )
                    },
                    Err(_) => {
                        // TODO: LOG
                        unimplemented!()
                        // (&[], DimsBox::new2(0, 0), Vector2::new(0, 0))
                    }
                }
            }
        );

        glyph_pos +=
            // rect top-left
            rect.min().to_vec() +
            // Advance the cursor down the line. Pos is with TLO, so vertical flip
            Vector2::new(1, -1).mul_element_wise(glyph_bearing);
        let glyph_rect = BoundBox::new2(
            glyph_pos.x,
            glyph_pos.y,
            glyph_pos.x + atlas_rect.width() as i32,
            glyph_pos.y + atlas_rect.height() as i32
        );

        ImageTranslate::new(
            glyph_rect,
            atlas_rect.cast::<u16>().unwrap_or(OffsetBox::new2(0, 0, 0, 0)),
            match is_highlighted {
                false => text_style.color,
                true => text_style.highlight_text_color
            },
            RescaleRules::Stretch
        )
    }
}


impl RenderString {
    pub fn new(string: String) -> RenderString {
        RenderString {
            string,
            cell: RefCell::new(None)
        }
    }

    #[inline]
    pub fn string(&self) -> &str {
        &self.string
    }

    #[inline]
    pub fn string_mut(&mut self) -> &mut String {
        self.cell.get_mut().as_mut().map(|cell| cell.shaped_glyphs.clear());
        &mut self.string
    }

    fn reshape_glyphs<'a, F>(&self,
        rect: BoundBox<Point2<i32>>,
        shape_text: F,
        text_style: &ThemeText,
        face: &mut Face<()>,
        dpi: DPI
    ) -> Ref<[RenderGlyph]>
        where F: FnOnce(&str, &mut Face<()>) -> &'a ShapedBuffer
    {
        {
            let mut cell_opt = self.cell.borrow_mut();
            let use_cached_glyphs: bool;
            let cell = match *cell_opt {
                Some(ref mut cell) => {
                    use_cached_glyphs =
                        cell.shaped_glyphs.len() != 0 &&
                        (text_style, dpi, rect) ==
                        (&cell.text_style, cell.dpi, cell.draw_rect);

                    // Update cell contents to reflect new values
                    cell.text_style = text_style.clone();
                    cell.dpi = dpi;
                    cell.draw_rect = rect;

                    cell
                },
                None => {
                    use_cached_glyphs = false;
                    *cell_opt = Some(RenderStringCell {
                        shaped_glyphs: Vec::new(),
                        text_style: text_style.clone(),
                        dpi,
                        draw_rect: rect
                    });
                    cell_opt.as_mut().unwrap()
                }
            };
            if !use_cached_glyphs {
                let shaped_buffer = shape_text(&self.string, face);
                cell.shaped_glyphs.clear();
                cell.shaped_glyphs.extend(GlyphIter::new(rect, shaped_buffer, text_style, face, dpi));
            }
        }

        Ref::map(self.cell.borrow(), |c| &c.as_ref().unwrap().shaped_glyphs[..])
    }

    fn selection_glyph_iter<'a>(&'a mut self) -> impl 'a + Iterator<Item=RenderGlyph> + DoubleEndedIterator {
        let empty_iter = [].iter().cloned().chain(None);

        let shaped_glyphs = match *self.cell.get_mut() {
            Some(ref cell) => &cell.shaped_glyphs,
            None => return empty_iter
        };

        if let Some(last_glyph) = shaped_glyphs.last().cloned() {
            let dummy_last_glyph_pos_x = last_glyph.pos.x + last_glyph.highlight_rect.width();
            let dummy_last_glyph = RenderGlyph {
                pos: Point2::new(dummy_last_glyph_pos_x, last_glyph.pos.y),
                highlight_rect: BoundBox::new2(
                    dummy_last_glyph_pos_x, last_glyph.highlight_rect.min.y,
                    dummy_last_glyph_pos_x, last_glyph.highlight_rect.min.y + last_glyph.highlight_rect.height()
                ),
                str_index: self.string.len(),
                glyph_index: None
            };
            shaped_glyphs.iter().cloned().chain(Some(dummy_last_glyph))
        } else {
            empty_iter
        }
    }
}

impl EditString {
    pub fn new(render_string: RenderString) -> EditString {
        EditString {
            render_string,
            draw_cursor: false,
            cursor_pos: 0,
            highlight_range: 0..0,
            cursor_target_x_px: None
        }
    }

    #[inline]
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    #[inline]
    pub fn cursor_pos_mut(&mut self) -> &mut usize {
        self.cursor_target_x_px = None;
        &mut self.cursor_pos
    }

    #[inline]
    pub fn highlight_range(&self) -> Range<usize> {
        self.highlight_range.clone()
    }

    pub fn move_cursor_vertical(&mut self, dist: isize, expand_selection: bool) {
        let cursor_start_pos = self.cursor_pos;
        let EditString {
            ref mut cursor_pos,
            ref mut cursor_target_x_px,
            ref mut render_string,
            ..
        } = *self;

        macro_rules! search_for_glyph {
            ($iter:expr) => {{
                let mut glyph_iter = $iter.skip_while(move |g| g.str_index != *cursor_pos);
                if let Some(cursor_glyph) = glyph_iter.next() {
                    let cursor_pos_px = cursor_glyph.highlight_rect.min;
                    if cursor_target_x_px.is_none() {
                        *cursor_target_x_px = Some(cursor_pos_px.x);
                    }
                    let target_x_px = cursor_target_x_px.unwrap();

                    let mut min_dist_x = i32::max_value();
                    let mut cur_line_y = cursor_glyph.highlight_rect.min.y;
                    let mut line_delta = 0;

                    for glyph in glyph_iter {
                        let glyph_dist_x = (target_x_px - glyph.highlight_rect.min.x).abs();
                        if glyph.highlight_rect.min.y != cur_line_y {
                            line_delta += 1;
                            cur_line_y = glyph.highlight_rect.min.y;
                            if line_delta > dist.abs() {
                                return;
                            }

                            min_dist_x = glyph_dist_x;
                            *cursor_pos = glyph.str_index;

                            continue;
                        }
                        if line_delta == 0 {
                            continue;
                        }

                        if glyph_dist_x < min_dist_x {
                            min_dist_x = glyph_dist_x;
                            *cursor_pos = glyph.str_index;
                        }
                    }
                }
            }}
        }

        let glyph_iter = render_string.selection_glyph_iter();
        match dist.signum() {
             0 => return,
             1 => search_for_glyph!(glyph_iter),
            -1 => search_for_glyph!(glyph_iter.rev()),
            _ => unreachable!()
        }
        if expand_selection {
            self.expand_selection_to_cursor(cursor_start_pos);
        } else {
            self.highlight_range = 0..0;
        }
    }

    pub fn move_cursor_horizontal(&mut self, dist: isize, jump_to_word_boundaries: bool, expand_selection: bool) {
        let cursor_start_pos = self.cursor_pos;
        self.cursor_target_x_px = None;
        self.cursor_pos = match (self.highlight_range.len() * !expand_selection as usize, dist.signum(), jump_to_word_boundaries) {
            (_, 0, _) => return,
            (0, 1, false) =>
                self.render_string.string[self.cursor_pos..].grapheme_indices(true)
                    .skip(dist as usize).map(|(i, _)| i + self.cursor_pos)
                    .next().unwrap_or(self.render_string.string.len()),
            (0, -1, false) =>
                self.render_string.string[..self.cursor_pos].grapheme_indices(true)
                    .rev().skip(dist.abs() as usize - 1).map(|(i, _)| i)
                    .next().unwrap_or(0),
            (0, 1, true) =>
                self.render_string.string[self.cursor_pos..].unicode_words()
                .skip(dist as usize).next()
                .map(|word| word.as_ptr() as usize - self.render_string.string.as_ptr() as usize)
                .unwrap_or(self.render_string.string.len()),
            (0, -1, true) => self.render_string.string[..self.cursor_pos].unicode_words()
                .rev().skip(dist.abs() as usize - 1).next()
                .map(|word| word.as_ptr() as usize - self.render_string.string.as_ptr() as usize)
                .unwrap_or(0),
            (_, 1, _) => self.highlight_range.end,
            (_, -1, _) => self.highlight_range.start,
            _ => unreachable!()
        };
        if expand_selection {
            self.expand_selection_to_cursor(cursor_start_pos);
        } else {
            self.highlight_range = 0..0;
        }
    }

    fn expand_selection_to_cursor(&mut self, cursor_start_pos: usize) {
        if self.highlight_range.len() == 0 {
            self.highlight_range = cursor_start_pos..cursor_start_pos;
        }

        match (cursor_start_pos == self.highlight_range.start, self.cursor_pos < self.highlight_range.end) {
            (false, true) if self.cursor_pos < self.highlight_range.start => {
                self.highlight_range.end = self.highlight_range.start;
                self.highlight_range.start = self.cursor_pos;
            }
            (false, _) => self.highlight_range.end = self.cursor_pos,
            (true, true) => self.highlight_range.start = self.cursor_pos,
            (true, false) => {
                self.highlight_range.start = self.highlight_range.end;
                self.highlight_range.end = self.cursor_pos;
            }
        }
    }

    pub fn select_on_line(&mut self, segment: Segment<Point2<i32>>) {
        let cell = self.render_string.cell.borrow();
        let cell = match *cell {
            Some(ref cell) => cell,
            None => {self.highlight_range = 0..0; return}
        };
        let shaped_glyphs = &cell.shaped_glyphs;

        // let mut min_y_dist = None;
        let dist = |min: i32, max: i32, point: i32| match (min.cmp(&point), max.cmp(&point)) {
            (Ordering::Equal, _) |
            (_, Ordering::Equal) |
            (Ordering::Less, Ordering::Greater) => 0,
            (Ordering::Greater, _) => min - point,
            (_, Ordering::Less) => point - min
        };


        let (mut min_start_x_dist, mut min_start_y_dist) = (i32::max_value(), i32::max_value());
        let (mut min_end_x_dist, mut min_end_y_dist) = (i32::max_value(), i32::max_value());
        let (mut start_index, mut end_index) = (0, 0);

        for glyph in shaped_glyphs.iter() {
            let x_dist = |point: Point2<_>| dist(glyph.highlight_rect.min.x, glyph.highlight_rect.max.x, point.x);
            let y_dist = |point: Point2<_>| dist(glyph.highlight_rect.min.y, glyph.highlight_rect.max.y, point.y);
            let glyph_start_x_dist = x_dist(segment.start);
            let glyph_start_y_dist = y_dist(segment.start);
            let glyph_end_x_dist = x_dist(segment.end);
            let glyph_end_y_dist = y_dist(segment.end);
            let highlight_center = glyph.highlight_rect.center();

            if glyph_start_y_dist < min_start_y_dist {
                min_start_y_dist = glyph_start_y_dist;
                min_start_x_dist = glyph_start_x_dist;
                start_index = glyph.str_index + (highlight_center.x <= segment.start.x) as usize;
            }
            if glyph_end_y_dist < min_end_y_dist {
                min_end_y_dist = glyph_end_y_dist;
                min_end_x_dist = glyph_end_x_dist;
                end_index = glyph.str_index + (highlight_center.x <= segment.end.x) as usize;
            }
            if glyph_start_x_dist < min_start_x_dist && glyph_start_y_dist <= min_start_y_dist {
                min_start_x_dist = glyph_start_x_dist;
                start_index = glyph.str_index + (highlight_center.x <= segment.start.x) as usize;
            }
            if glyph_end_x_dist < min_end_x_dist && glyph_end_y_dist <= min_end_y_dist {
                min_end_x_dist = glyph_end_x_dist;
                end_index = glyph.str_index + (highlight_center.x <= segment.end.x) as usize;
            }
        }

        self.highlight_range = cmp::min(start_index, end_index)..cmp::max(start_index, end_index);
        self.cursor_pos = end_index;
    }

    pub fn select_all(&mut self) {
        self.highlight_range = 0..self.render_string.string.len();
        self.cursor_pos = self.highlight_range.end;
    }

    pub fn deselect_all(&mut self) {
        self.highlight_range = 0..0;
    }

    pub fn insert_char(&mut self, c: char) {
        if self.highlight_range.len() != 0 {
            self.render_string.string_mut().drain(self.highlight_range.clone());
            self.cursor_pos = self.highlight_range.start;
            self.highlight_range = 0..0;
        }
        self.render_string.string_mut().insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn insert_str(&mut self, s: &str) {
        if self.highlight_range.len() != 0 {
            self.render_string.string_mut().drain(self.highlight_range.clone());
            self.cursor_pos = self.highlight_range.start;
            self.highlight_range = 0..0;
        }
        self.render_string.string_mut().insert_str(self.cursor_pos, s);
        self.cursor_pos += s.len();
    }

    pub fn delete_chars(&mut self, dist: isize, jump_to_word_boundaries: bool) {
        let drain_range = if self.highlight_range.len() != 0 {
            self.highlight_range.clone()
        } else {
            let old_pos = self.cursor_pos;
            self.move_cursor_horizontal(dist, jump_to_word_boundaries, false);
            let new_pos = self.cursor_pos;
            cmp::min(old_pos, new_pos)..cmp::max(old_pos, new_pos)
        };
        self.render_string.string_mut().drain(drain_range.clone());
        self.highlight_range = 0..0;
        self.cursor_pos = drain_range.start;
    }
}
