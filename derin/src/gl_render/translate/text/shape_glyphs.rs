use crate::theme::{ThemeText, LineWrap};

use crate::cgmath::{EuclideanSpace, ElementWise, Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use glyphydog::{ShapedBuffer, ShapedGlyph, Face, FaceSize, DPI, LoadFlags};
use derin_common_types::layout::Align;

use itertools::Itertools;
use std::vec;
use std::any::Any;

pub fn shape_glyphs(
    rect: BoundBox<D2, i32>,
    shaped_text: &ShapedBuffer,
    text_style: &ThemeText,
    face: &mut Face<Any>,
    dpi: DPI,
    glyphs_out: &mut Vec<RenderGlyph>,
) -> ShapedGlyphsData
{
    let mut glyph_iter = GlyphIter::new(rect, shaped_text, text_style, face, dpi);
    glyphs_out.extend(&mut glyph_iter);
    ShapedGlyphsData {
        text_rect: glyph_iter.text_rect.unwrap_or(BoundBox::new2(0, 0, 0, 0,)),
    }
}


#[derive(Debug, Clone, Copy)]
pub struct ShapedGlyphsData {
    pub text_rect: BoundBox<D2, i32>,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderGlyph {
    /// The glyph's position relative to the top-left corner of the text box.
    pub pos: Point2<i32>,
    /// The rectangle that should get filled in by the highlight box when the
    /// glyph gets highlighted.
    pub highlight_rect: BoundBox<D2, i32>,
    /// The index into the string where this character is stored.
    pub str_index: usize,
    /// The glyph's index in the font face.
    pub glyph_index: Option<u32>,
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

    text_rect: Option<BoundBox<D2, i32>>,

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

impl GlyphIter {
    fn new(
        rect: BoundBox<D2, i32>,
        shaped_text: &ShapedBuffer,
        text_style: &ThemeText,
        face: &mut Face<Any>,
        dpi: DPI,
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
        let mut ends_with_newline = false;

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
            let mut trailing_newline_glyph = None;

            // Loop over the glyphs, alternating between inserting renderable glyphs and
            // whitespace. First half handles renderable, second half whitespace.
            'segment_glyphs: while glyphs.peek().is_some() {
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
                        if whitespace_advance == 0 {
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

                for (mut glyph, c) in glyphs.peeking_take_while(|&(_, c)| c.is_whitespace()) {
                    match c {
                        // If the whitespace is a tab, push all the accumulated whitespace, begin a
                        // new run and mark off the old run.
                        '\t' => {
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
                        },
                        '\r' |
                        '\n' => {
                            glyph.advance.x = 0;
                            trailing_newline_glyph = Some(glyph);

                            push_whitespace!();
                            break 'segment_glyphs;
                        }
                        _ => {
                            whitespace_glyph_count += 1;
                            segment_item_count += 1;
                            glyph_items.push(GlyphItem::WhitespaceGlyph(glyph));
                            whitespace_advance += glyph.advance.x
                        },
                    }
                }

                push_whitespace!();
            }

            // If the segment hasn't moved the cursor beyond the line's length, append it to the run.
            if line_advance <= rect.width() as i32 || text_style.line_wrap == LineWrap::None {
                run = run.append_run(segment_run);
            }

            let is_hard_break = segment.break_type.is_hard_break();
            if (is_hard_break || line_advance > rect.width() as i32) && text_style.line_wrap != LineWrap::None {
                glyph_items.insert(run_insert_index, GlyphItem::Run(run.ends_line()));

                num_lines += 1;
                match line_advance > rect.width() as i32 {
                    // Happens if the last segment ran over the rectangle length.
                    true => {
                        line_advance -= run.trailing_whitespace + segment_run.advance();
                        glyph_items.insert(line_insert_index, GlyphItem::Line{ advance: line_advance, hard_break: is_hard_break });

                        line_advance = segment_run.advance();
                        run = segment_run;

                        let insert_index = glyph_items.len() - segment_item_count - 1;
                        line_insert_index = insert_index;
                        run_insert_index = insert_index;
                    },
                    // Happens if we've hit a hard break and the last segment isn't overflowing the rectangle.
                    false => {
                        line_advance -= run.trailing_whitespace;
                        glyph_items.insert(line_insert_index, GlyphItem::Line{ advance: line_advance, hard_break: is_hard_break });

                        line_advance = 0;
                        run = Run::default();

                        line_insert_index = glyph_items.len();
                        run_insert_index = glyph_items.len();
                    }
                }

                if let Some(glyph) = trailing_newline_glyph {
                    glyph_items.push(
                        GlyphItem::Whitespace {
                            glyph_count: 0,
                            advance: 0
                        }
                    );
                    glyph_items.push(GlyphItem::WhitespaceGlyph(glyph));
                    ends_with_newline = true;
                } else {
                    ends_with_newline = false;
                }
            }

            segment_index += 1;
        }

        if run != Run::default() || ends_with_newline {
            glyph_items.insert(run_insert_index, GlyphItem::Run(run));
        }
        if line_advance != 0 || ends_with_newline {
            glyph_items.insert(line_insert_index, GlyphItem::Line{ advance: line_advance, hard_break: true });
            num_lines += 1;
        }

        let face_size = FaceSize::new(text_style.face_size, text_style.face_size);

        let font_metrics = face.metrics_sized(face_size, dpi).unwrap();
        let line_height = (font_metrics.height / 64) as i32;
        let (ascender, descender) = ((font_metrics.ascender / 64) as i32, (font_metrics.descender / 64) as i32);

        let v_advance = match text_style.justify.y {
            Align::Stretch => (rect.height() / (num_lines + 1)) as i32,
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
            text_rect: None,

            on_hard_break: false,
            tab_advance,
            bounds_width: rect.width() as i32,
        }
    }

    fn highlight_rect(&self, glyph_pos: Point2<i32>, glyph_advance: i32) -> BoundBox<D2, i32> {
        BoundBox::new2(
            glyph_pos.x, glyph_pos.y - self.font_ascender,
            glyph_pos.x + glyph_advance, glyph_pos.y - self.font_descender
        )
    }

    fn update_text_rect(&mut self, glyph_rect: BoundBox<D2, i32>) {
        match self.text_rect {
            None => self.text_rect = Some(glyph_rect),
            Some(ref mut rect) => {
                rect.min.x = rect.min.x.min(glyph_rect.min.x);
                rect.min.y = rect.min.y.min(glyph_rect.min.y);
                rect.max.x = rect.max.x.max(glyph_rect.max.x);
                rect.max.y = rect.max.y.max(glyph_rect.max.y);
            }
        }
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
                    self.update_text_rect(render_glyph.highlight_rect);
                    return Some(render_glyph);
                },
                GlyphItem::Word{..} => continue,
                GlyphItem::WhitespaceGlyph(glyph) => {
                    let cursor_advance = match self.x_justify == Align::Stretch && !self.on_hard_break {
                        false => glyph.advance.x,
                        true => {
                            // When you're doing full justification, you need to evenly distribute the whitespace
                            // between all words so that the leftmost word and rightmost word touch the left and
                            // right edges of the text box, respectively. However, since we're using integers for
                            // positioning we run into a problem: if the amount of whitespace available doesn't
                            // evenly divide into the number of whitespace slots available, not enough whitespace
                            // will get inserted to bring the rightmost word to the right edge (see examples).
                            //
                            // Ideal situation: 3 whitespace breaks with 15 pixels of whitespace. Whitespace gets
                            // evenly distributed.
                            // |whitespace     progression     example     text|
                            //
                            // Most situations: 3 whitespace breaks with 17 pixels of whitespace. 17 % 3 != 0, so
                            // there's leftover whitespace at the right edge.
                            // |whitespace     progression     example     text  |
                            //
                            //
                            // We use `OverflowAdd` to distribute the tailing whitespace into the inner whitespace
                            // slots. That way, unideal scenario above becomes this:
                            //
                            // Trailing whitespace is distributed.
                            // |whitespace     progression      example      text|
                            //
                            //
                            // This looks much nicer than the naive situation and handles justification properly.
                            let advance_shift = (glyph.advance.x as i64) << OVERFLOW_SHIFT;
                            let fillable_whitespace = match self.active_run.ends_line {
                                false => self.active_run.whitespace_advance as i64,
                                true => (self.bounds_width - self.run_start_x - self.active_run.glyph_advance) as i64
                            };

                            (
                                self.whitespace_overflower.add(
                                    advance_shift / self.active_run.whitespace_advance as i64 *
                                    fillable_whitespace
                                ) >> OVERFLOW_SHIFT
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

                    self.update_text_rect(render_glyph.highlight_rect);
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
                    continue;
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

                    self.update_text_rect(render_glyph.highlight_rect);
                    return Some(render_glyph);
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
