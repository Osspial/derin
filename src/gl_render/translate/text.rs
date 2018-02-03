use gl_render::GLVertex;
use gl_render::atlas::Atlas;
use gl_render::translate::image::ImageTranslate;
use theme::{ThemeText, RescaleRules};

use cgmath::{EuclideanSpace, ElementWise, Point2, Vector2};
use cgmath_geometry::{BoundBox, DimsBox, OffsetBox, GeoBox};

use gl_raii::colors::Rgba;
use gl_raii::glsl::Nu8;

use glyphydog::{ShapedBuffer, ShapedGlyph, Face, FaceSize, DPI, LoadFlags, RenderMode};
use dct::hints::Align;

use itertools::Itertools;
use std::vec;


pub(in gl_render) struct TextTranslate<'a> {
    glyph_draw: GlyphDraw<'a>,

    rect: BoundBox<Point2<i32>>,
    glyph_iter: GlyphIter<vec::IntoIter<GlyphItem>>,
    vertex_iter: Option<ImageTranslate>
}

struct GlyphDraw<'a> {
    face: &'a mut Face<()>,
    atlas: &'a mut Atlas,
    text_style: ThemeText,
    dpi: DPI
}

struct GlyphIter<I: Iterator<Item=GlyphItem>> {
    glyph_items: I,
    v_advance: i32,
    line_start_x: i32,
    run_start_x: i32,
    cursor: Vector2<i32>,
    x_justify: Align,
    active_run: Run,
    on_hard_break: bool,
    whitespace_overflower: OverflowAdd,

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
        advance: i32
    },
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
    Tab,
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
    pub fn new(
        rect: BoundBox<Point2<i32>>,
        shaped_text: &'a ShapedBuffer,
        text_style: ThemeText,
        face: &'a mut Face<()>,
        dpi: DPI,
        atlas: &'a mut Atlas
    ) -> TextTranslate<'a> {
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
                macro_rules! push_whitespace {
                    () => {{
                        if whitespace_advance != 0 {
                            line_advance += whitespace_advance;
                            segment_run = segment_run.append_run(Run::tail_whitespace(whitespace_advance));
                            glyph_items.push(GlyphItem::Whitespace{ advance: whitespace_advance });
                            segment_item_count += 1;
                            #[allow(unused_assignments)]
                            {
                                whitespace_advance = 0;
                            }
                        }
                    }}
                }

                for (glyph, c) in glyphs.peeking_take_while(|&(_, c)| c.is_whitespace()) {
                    match c == '\t' {
                        false => whitespace_advance += glyph.advance.x,
                        // If the whitespace is a tab, push all the accumulated whitespace, begin a
                        // new run and mark off the old run.
                        true => {
                            push_whitespace!();

                            // Move the advance to the next tab stop.
                            line_advance = ((line_advance/tab_advance) + 1) * tab_advance;
                            // If the last thing in `glyph_items` is a tab, then we're in a sequence of `Tab`s
                            // and the `Run` was already inserted by the first tab.
                            if glyph_items.last() != Some(&GlyphItem::Tab) {
                                glyph_items.insert(run_insert_index, GlyphItem::Run(run.append_run(segment_run)));
                            }
                            glyph_items.push(GlyphItem::Tab);
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
        let line_height = (face.metrics_sized(face_size, dpi).unwrap().height / 64) as i32;

        let v_advance = match text_style.justify.y {
            Align::Stretch => (rect.height() / num_lines) as i32,
            _ => line_height
        };

        TextTranslate {
            rect,
            glyph_iter: GlyphIter {
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
                on_hard_break: false,
                tab_advance,
                bounds_width: rect.width() as i32,
            },
            glyph_draw: GlyphDraw{ face, atlas, text_style, dpi },
            vertex_iter: None
        }
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

impl<I: Iterator<Item=GlyphItem>> Iterator for GlyphIter<I> {
    type Item = ShapedGlyph;

    fn next(&mut self) -> Option<ShapedGlyph> {
        loop {
            match self.glyph_items.next()? {
                GlyphItem::Glyph(mut glyph) => {
                    glyph.pos = Point2::from_vec(self.cursor);
                    self.cursor += glyph.advance.mul_element_wise(Vector2::new(1, -1));
                    return Some(glyph);
                },
                GlyphItem::Word{..} => (),
                GlyphItem::Whitespace{advance} => {
                    let cursor_advance = match self.x_justify == Align::Stretch && !self.on_hard_break {
                        false => advance,
                        true => {
                            let advance_shift = (advance as i64) << OVERFLOW_SHIFT;
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
                    self.cursor.x += cursor_advance;
                    continue;
                },
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
                GlyphItem::Tab => {
                    self.cursor.x = (((self.cursor.x - self.line_start_x)/self.tab_advance) + 1) * self.tab_advance + self.line_start_x;
                    continue;
                }
            }
        }
    }
}

impl<'a> Iterator for TextTranslate<'a> {
    type Item = GLVertex;

    fn next(&mut self) -> Option<GLVertex> {
        loop {
            match self.vertex_iter.as_mut().map(|v| v.next()).unwrap_or(None) {
                Some(vert) => return Some(vert),
                None => {
                    let next_glyph = self.glyph_iter.next()?;
                    self.vertex_iter = Some(self.glyph_draw.glyph_atlas_image(next_glyph, self.rect));
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
    fn glyph_atlas_image(&mut self, glyph: ShapedGlyph, rect: BoundBox<Point2<i32>>) -> ImageTranslate {
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
            glyph.glyph_index,
            || {
                let glyph_res = face.load_glyph(
                    glyph.glyph_index,
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

        let glyph_pos =
            // rect top-left
            rect.min() +
            glyph.pos.to_vec() +
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
            text_style.color,
            RescaleRules::Stretch
        )
    }
}
