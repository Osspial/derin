// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::{Glyph, FaceMetrics};
use crate::rect_layout::theme::{TextLayoutStyle, LineWrap};

use crate::cgmath::{EuclideanSpace, ElementWise, Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};

use derin_common_types::layout::Align;

use itertools::Itertools;
use std::vec;

use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy)]
pub struct ShapedGlyphsData {
    pub text_rect: BoundBox<D2, i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderGlyph {
    /// The glyph's position relative to the top-left corner of the text box.
    pub pos: Point2<i32>,
    /// The rectangle that should get filled in by the highlight box when the
    /// glyph gets highlighted.
    pub highlight_rect: BoundBox<D2, i32>,
    /// The index into the string where this character is stored.
    pub str_index: usize,
    /// The length, in bytes, of the grapheme cluster the glyph represents.
    pub grapheme_len: usize,
    /// The glyph's index in the font face.
    pub glyph_index: Option<u32>,
}

pub struct GlyphIterBuilder {
    rect: DimsBox<D2, i32>,
    text_style: TextLayoutStyle,
    face_metrics: FaceMetrics,
    glyph_items: Vec<GlyphItem>,
    run: Run,
    line_advance: i32,
    num_lines: i32,
    line_insert_index: usize,
    run_insert_index: usize,
    ends_with_newline: bool,
}

pub struct GlyphIter {
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
    Glyph {
        glyph: Glyph,
        grapheme_len: usize,
    },
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
    WhitespaceGlyph(Glyph),
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

impl GlyphIterBuilder {
    pub fn new(
        rect: DimsBox<D2, i32>,
        text_style: TextLayoutStyle,
        face_metrics: FaceMetrics,
    ) -> GlyphIterBuilder
    {
        GlyphIterBuilder {
            rect,
            text_style,
            face_metrics,

            glyph_items: Vec::new(),
            run: Run::default(),
            line_advance: 0,
            num_lines: 0,
            line_insert_index: 0,
            run_insert_index: 0,
            ends_with_newline: false
        }
    }

    pub fn add_segment(&mut self, text: &str, str_offset: usize, hard_break: bool, glyphs: impl Iterator<Item=Glyph>) {
        // Create an iterator over every glyph in the segment.
        let mut glyphs = {
            let glyph_with_char = |glyph: Glyph| (glyph, text[glyph.str_index..].chars().next().unwrap());
            glyphs.map(glyph_with_char).peekable()
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

            let word_insert_index = self.glyph_items.len();
            let (mut glyph_count, mut word_advance) = (0, 0);

            // Continue taking glyphs until we hit whitespace.
            for (mut glyph, _) in glyphs.peeking_take_while(|&(_, c)| !c.is_whitespace()) {
                glyph_count += 1;
                word_advance += glyph.advance.x;
                let segment_str_index = glyph.str_index;
                glyph.str_index += str_offset;
                self.glyph_items.push(GlyphItem::Glyph {
                    glyph,
                    grapheme_len: text[segment_str_index..].graphemes(true).next().unwrap().len(),
                });
            }
            // If there are glyphs to add, insert a `Word` and increment the advances.
            if glyph_count > 0 {
                segment_run.trailing_whitespace = 0;

                segment_item_count += glyph_count as usize;
                self.line_advance += word_advance;
                segment_run.glyph_advance += word_advance;
                self.glyph_items.insert(word_insert_index, GlyphItem::Word{ glyph_count, advance: word_advance });
            }

            // Add sequence of whitespace characters.

            let mut whitespace_advance = 0;
            let mut whitespace_glyph_count = 0;
            let mut whitespace_insert_index = self.glyph_items.len();
            macro_rules! push_whitespace {
                () => {{
                    if whitespace_advance == 0 {
                        self.line_advance += whitespace_advance;
                        segment_run = segment_run.append_run(Run::tail_whitespace(whitespace_advance));
                        self.glyph_items.insert(
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
                            whitespace_insert_index = self.glyph_items.len();
                        }
                    }
                }}
            }

            for (mut glyph, c) in glyphs.peeking_take_while(|&(_, c)| c.is_whitespace()) {
                glyph.str_index += str_offset;
                match c {
                    // If the whitespace is a tab, push all the accumulated whitespace, begin a
                    // new self.run and mark off the old self.run.
                    '\t' => {
                        push_whitespace!();

                        // Move the advance to the next tab stop.
                        self.line_advance = ((self.line_advance/self.face_metrics.tab_advance) + 1) * self.face_metrics.tab_advance;
                        // If the last thing in `self.glyph_items` is a tab, then we're in a sequence of `Tab`s
                        // and the `Run` was already inserted by the first tab.
                        match self.glyph_items.last() {
                            Some(&GlyphItem::Tab{..}) => (),
                            _ => self.glyph_items.insert(self.run_insert_index, GlyphItem::Run(self.run.append_run(segment_run)))
                        }
                        self.glyph_items.push(GlyphItem::Tab{ str_index: glyph.str_index });
                        self.run = Run::default();
                        segment_run = Run::default();
                        self.run_insert_index = self.glyph_items.len();
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
                        self.glyph_items.push(GlyphItem::WhitespaceGlyph(glyph));
                        whitespace_advance += glyph.advance.x
                    },
                }
            }

            push_whitespace!();
        }

        // If the segment hasn't moved the cursor beyond the line's length, append it to the self.run.
        if self.line_advance <= self.rect.width() as i32 || self.text_style.line_wrap == LineWrap::None {
            self.run = self.run.append_run(segment_run);
        }

        let is_hard_break = hard_break;
        if (is_hard_break || self.line_advance > self.rect.width() as i32) && self.text_style.line_wrap != LineWrap::None {
            self.glyph_items.insert(self.run_insert_index, GlyphItem::Run(self.run.ends_line()));

            self.num_lines += 1;
            match self.line_advance > self.rect.width() as i32 {
                // Happens if the last segment ran over the rectangle length.
                true => {
                    self.line_advance -= self.run.trailing_whitespace + segment_run.advance();
                    self.glyph_items.insert(self.line_insert_index, GlyphItem::Line{ advance: self.line_advance, hard_break: is_hard_break });

                    self.line_advance = segment_run.advance();
                    self.run = segment_run;

                    let insert_index = self.glyph_items.len() - segment_item_count - 1;
                    self.line_insert_index = insert_index;
                    self.run_insert_index = insert_index;
                },
                // Happens if we've hit a hard break and the last segment isn't overflowing the rectangle.
                false => {
                    self.line_advance -= self.run.trailing_whitespace;
                    self.glyph_items.insert(self.line_insert_index, GlyphItem::Line{ advance: self.line_advance, hard_break: is_hard_break });

                    self.line_advance = 0;
                    self.run = Run::default();

                    self.line_insert_index = self.glyph_items.len();
                    self.run_insert_index = self.glyph_items.len();
                }
            }

            if let Some(glyph) = trailing_newline_glyph {
                self.glyph_items.push(
                    GlyphItem::Whitespace {
                        glyph_count: 0,
                        advance: 0
                    }
                );
                self.glyph_items.push(GlyphItem::WhitespaceGlyph(glyph));
                self.ends_with_newline = true;
            } else {
                self.ends_with_newline = false;
            }
        }
    }

    pub fn build(mut self) -> GlyphIter {
        if self.run != Run::default() || self.ends_with_newline {
            self.glyph_items.insert(self.run_insert_index, GlyphItem::Run(self.run));
        }
        if self.line_advance != 0 || self.ends_with_newline {
            self.glyph_items.insert(self.line_insert_index, GlyphItem::Line{ advance: self.line_advance, hard_break: true });
            self.num_lines += 1;
        }

        let line_height = (self.face_metrics.line_height / 64) as i32;
        let (ascender, descender) = ((self.face_metrics.ascender / 64) as i32, (self.face_metrics.descender / 64) as i32);

        let v_advance = match self.text_style.justify.y {
            Align::Stretch => (self.rect.height() / (self.num_lines + 1)) as i32,
            _ => line_height
        };

        GlyphIter {
            glyph_items: self.glyph_items.into_iter(),
            v_advance,
            cursor: Vector2 {
                x: 0,
                y: match self.text_style.justify.y {
                    Align::Center => (self.rect.height() as i32 - (line_height * self.num_lines as i32)) / 2,
                    Align::End => self.rect.height() as i32 - (line_height * self.num_lines as i32),
                    _ => 0
                }
            },
            line_start_x: 0,
            run_start_x: 0,
            x_justify: self.text_style.justify.x,
            active_run: Run::default(),
            whitespace_overflower: OverflowAdd::default(),

            font_ascender: ascender,
            font_descender: descender,
            text_rect: None,

            on_hard_break: false,
            tab_advance: self.face_metrics.tab_advance,
            bounds_width: self.rect.width() as i32,
        }
    }
}

impl GlyphIter {
    pub fn shaped_data(&self) -> ShapedGlyphsData {
        ShapedGlyphsData {
            text_rect: self.text_rect.unwrap_or(BoundBox::new2(0, 0, 0, 0)),
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
                GlyphItem::Glyph{glyph, grapheme_len} => {
                    let render_glyph = RenderGlyph {
                        pos: Point2::from_vec(self.cursor),
                        highlight_rect: self.highlight_rect(Point2::from_vec(self.cursor), glyph.advance.x),
                        str_index: glyph.str_index,
                        grapheme_len,
                        glyph_index: Some(glyph.glyph_index),
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
                        grapheme_len: 1,
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
                        grapheme_len: 1,
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
