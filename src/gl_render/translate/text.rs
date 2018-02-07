use gl_render::GLVertex;
use gl_render::atlas::Atlas;
use gl_render::translate::image::ImageTranslate;
use theme::{ThemeText, RescaleRules};

use cgmath::{EuclideanSpace, ElementWise, Point2, Vector2};
use cgmath_geometry::{BoundBox, DimsBox, OffsetBox, Segment, GeoBox};

use gl_raii::colors::Rgba;
use gl_raii::glsl::Nu8;

use glyphydog::{ShapedBuffer, ShapedGlyph, Face, FaceSize, DPI, LoadFlags, RenderMode};
use dct::hints::Align;

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
    highlight_vertex_iter: Option<ImageTranslate>,
    glyph_vertex_iter: Option<ImageTranslate>
}

#[derive(Debug, Clone, Copy)]
pub struct RenderGlyph {
    pos: Point2<i32>,
    highlight_rect: BoundBox<Point2<i32>>,
    str_index: usize,
    glyph_index: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RenderString {
    string: String,
    pub highlight_range: Range<usize>,
    cell: RefCell<Option<RenderStringCell>>
}

#[derive(Debug, Clone)]
struct RenderStringCell {
    shaped_glyphs: Vec<RenderGlyph>,
    text_style: ThemeText,
    dpi: DPI,
    draw_rect: BoundBox<Point2<i32>>,
}

impl RenderString {
    pub fn new(string: String) -> RenderString {
        RenderString {
            string,
            highlight_range: 0..0,
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

    pub fn select_on_line(&mut self, segment: Segment<Point2<i32>>) {
        let cell = self.cell.borrow();
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
        let mut end_in_range = false;

        for (i, glyph) in shaped_glyphs.iter().enumerate() {
            let x_dist = |point: Point2<_>| dist(glyph.highlight_rect.min.x, glyph.highlight_rect.max.x, point.x);
            let y_dist = |point: Point2<_>| dist(glyph.highlight_rect.min.y, glyph.highlight_rect.max.y, point.y);
            let glyph_start_x_dist = x_dist(segment.start);
            let glyph_start_y_dist = y_dist(segment.start);
            let glyph_end_x_dist = x_dist(segment.end);
            let glyph_end_y_dist = y_dist(segment.end);

            if glyph_start_y_dist < min_start_y_dist {
                min_start_y_dist = glyph_start_y_dist;
                min_start_x_dist = glyph_start_x_dist;
                start_index = i;
            }
            if glyph_end_y_dist < min_end_y_dist {
                min_end_y_dist = glyph_end_y_dist;
                min_end_x_dist = glyph_end_x_dist;
                end_index = i;
                end_in_range = glyph.highlight_rect.center().x <= segment.end.x;
            }
            if glyph_start_x_dist < min_start_x_dist && glyph_start_y_dist <= min_start_y_dist {
                min_start_x_dist = glyph_start_x_dist;
                start_index = i;
            }
            if glyph_end_x_dist < min_end_x_dist && glyph_end_y_dist <= min_end_y_dist {
                min_end_x_dist = glyph_end_x_dist;
                end_index = i;
                end_in_range = glyph.highlight_rect.center().x <= segment.end.x;
            }
        }

        end_index += end_in_range as usize;
        self.highlight_range = cmp::min(start_index, end_index)..cmp::max(start_index, end_index);
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
    pub fn new<'b, F>(
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
        TextTranslate {
            rect,
            glyph_slice_index: 0,
            glyph_slice: render_string.reshape_glyphs(rect, shape_text, &text_style, face, dpi),
            glyph_draw: GlyphDraw{ face, atlas, text_style, dpi },
            highlight_range: render_string.highlight_range.clone(),
            highlight_vertex_iter: None,
            glyph_vertex_iter: None
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
            let next_vertex =
                self.highlight_vertex_iter.as_mut().map(|v| v.next()).unwrap_or(None)
                    .or_else(|| self.glyph_vertex_iter.as_mut().map(|v| v.next()).unwrap_or(None));
            match next_vertex {
                Some(vert) => return Some(vert),
                None => {
                    let TextTranslate {
                        ref glyph_slice,
                        ref mut glyph_slice_index,
                        ref highlight_range,
                        ref mut glyph_vertex_iter,
                        ref mut glyph_draw,
                        ref mut highlight_vertex_iter,
                        rect,
                    } = *self;
                    let next_glyph = glyph_slice.get(*glyph_slice_index)?;
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

                    *highlight_vertex_iter = match is_highlighted {
                        true => {
                            Some(ImageTranslate::new(
                                next_glyph.highlight_rect + rect.min().to_vec(),
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
