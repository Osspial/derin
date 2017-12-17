use gl_render::GLVertex;
use gl_render::atlas::Atlas;
use gl_render::translate::image::ImageTranslate;
use theme::{ThemeText, RescaleRules};

use cgmath::{EuclideanSpace, ElementWise, Vector2};
use cgmath_geometry::{BoundRect, DimsRect, OffsetRect, Rectangle};

use gl_raii::colors::Rgba;
use gl_raii::glsl::Nu8;

use glyphydog::{ShapedBuffer, ShapedSegment, ShapedGlyph, Face, FaceSize, DPI, LoadFlags, RenderMode};
use dct::hints::{Align, Align2};

use std::ops::Range;


pub(in gl_render) struct TextTranslate<'a> {
    shaped_text: &'a ShapedBuffer,
    glyph_draw: GlyphDraw<'a>,

    rect: BoundRect<u32>,
    active_line: Line<'a>,
    line_num: u32,

    glyph_verts: Option<ImageTranslate>,

    tab_stop: TabStop
}

struct GlyphDraw<'a> {
    face: &'a mut Face<()>,
    atlas: &'a mut Atlas,
    text_style: ThemeText,
    dpi: DPI
}

struct Line<'a> {
    shaped_text: &'a ShapedBuffer,
    segment_range: Range<usize>,

    glyph_offset: Vector2<i32>,

    cur_segment: Option<ShapedSegment<'a>>,
    cur_segment_index: usize,
    cur_glyph: usize,
    segment_cursor: i32,
    segment_glyph_offset: i32,

    tab_stop: TabStop
}

#[derive(Clone, Copy)]
struct TabStop {
    glyph_index: u32,
    tab_glyph_advance: i32,
    advance: i32
}

enum LineItem {
    Glyph(ShapedGlyph),
    /// Contains the advance of the word
    WordEnd(i32)
}

impl<'a> TextTranslate<'a> {
    pub fn new(
        rect: BoundRect<u32>,
        shaped_text: &'a ShapedBuffer,
        text_style: ThemeText,
        face: &'a mut Face<()>,
        dpi: DPI,
        atlas: &'a mut Atlas
    ) -> TextTranslate<'a> {
        let tab_glyph_index = face.char_index('\t');
        let space_glyph_index = face.char_index(' ');

        let tab_glyph_advance = (face.glyph_advance(
            tab_glyph_index,
            FaceSize::new(text_style.face_size, text_style.face_size),
            dpi,
            LoadFlags::empty()
        ).unwrap() + (1 << 15)) >> 16;
        let space_glyph_advance = (face.glyph_advance(
            space_glyph_index,
            FaceSize::new(text_style.face_size, text_style.face_size),
            dpi,
            LoadFlags::empty()
        ).unwrap() + (1 << 15)) >> 16;

        let tab_stop = TabStop {
            glyph_index: tab_glyph_index,
            tab_glyph_advance,
            advance: space_glyph_advance * text_style.tab_size as i32
        };

        TextTranslate {
            active_line: Line::from_segments(0, rect.width(), tab_stop, text_style.justify, shaped_text),
            shaped_text,
            glyph_draw: GlyphDraw {
                face, atlas, text_style, dpi
            },

            rect,
            line_num: 0,

            glyph_verts: None,

            tab_stop
        }
    }
}

impl<'a> GlyphDraw<'a> {
    fn glyph_atlas_image(&mut self, glyph: ShapedGlyph, rect: BoundRect<u32>, line_num: u32) -> ImageTranslate {
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
                            0 => (&[][..], 1, DimsRect::new(0, 0)),
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
                        // (&[], DimsRect::new(0, 0), Vector2::new(0, 0))
                    }
                }
            }
        );
        let line_height = (face.metrics_sized(face_size, dpi).unwrap().height / 64) as i32;

        let glyph_pos =
            // rect top-left
            rect.min().cast::<i32>().unwrap() +
            // Move the point down the correct number of lines
            Vector2::new(0, line_height * (line_num as i32 + 1)) +
            // Advance the cursor down the line. Pos is with TLO, so vertical flip
            Vector2::new(1, -1).mul_element_wise(glyph.pos.to_vec() + glyph_bearing);
        let glyph_rect = BoundRect::new(
            glyph_pos.x,
            glyph_pos.y,
            glyph_pos.x + atlas_rect.width() as i32,
            glyph_pos.y + atlas_rect.height() as i32
        ).cast::<u32>().unwrap_or(BoundRect::new(0, 0, 0, 0));

        ImageTranslate::new(
            glyph_rect,
            atlas_rect.cast::<u16>().unwrap_or(OffsetRect::new(0, 0, 0, 0)),
            text_style.color,
            RescaleRules::Stretch
        )
    }
}

impl<'a> Iterator for TextTranslate<'a> {
    type Item = GLVertex;

    fn next(&mut self) -> Option<GLVertex> {
        'get_vert: loop {
            match self.glyph_verts.as_mut().map(|v| v.next()).unwrap_or(None) {
                Some(vert) => return Some(vert),
                None => 'set_glyph: loop {
                    match self.active_line.by_ref().filter_map(|i| i.to_glyph_opt()).next() {
                        Some(next_glyph) => {
                            self.glyph_verts = Some(self.glyph_draw.glyph_atlas_image(next_glyph, self.rect, self.line_num));
                            continue 'get_vert;
                        },
                        None => match self.active_line.segment_range.end == self.shaped_text.segments_len() {
                            true => return None,
                            false => {
                                self.active_line = Line::from_segments(
                                    self.active_line.segment_range.end,
                                    self.rect.width(),
                                    self.tab_stop,
                                    self.glyph_draw.text_style.justify,
                                    self.shaped_text
                                );
                                self.line_num += 1;
                                continue 'set_glyph;
                            }
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Line<'a> {
    fn from_segments(index: usize, max_len_px: u32, tab_stop: TabStop, justify: Align2, shaped_text: &'a ShapedBuffer) -> Line<'a> {
        let end_index: usize;
        let mut line_len = 0;

        {
            let mut lit = Line {
                shaped_text,
                segment_range: index..shaped_text.segments_len(),

                glyph_offset: Vector2::new(0, 0),

                cur_segment: shaped_text.get_segment(index),
                cur_segment_index: 0,
                cur_glyph: 0,
                segment_cursor: 0,
                segment_glyph_offset: 0,

                tab_stop
            };

            while let Some(item) = lit.next() {
                match item {
                    LineItem::Glyph(glyph) => {
                        if glyph.pos.x + glyph.advance.x > max_len_px as i32 {
                            break;
                        }
                    },
                    LineItem::WordEnd(advance) => line_len += advance
                }
            }

            end_index = match lit.cur_segment_index {
                0 => index + 1,
                _ => lit.cur_segment_index + index
            };
        }

        Line {
            shaped_text,
            segment_range: index..end_index,

            glyph_offset: Vector2 {
                x: match justify.x {
                    Align::Stretch => 0, // TODO: JUSTIFY
                    Align::Start => 0,
                    Align::Center => (max_len_px as i32 - line_len) / 2,
                    Align::End => max_len_px as i32 - line_len
                },
                y: 0
            },

            cur_segment: shaped_text.get_segment(index),
            cur_segment_index: 0,
            cur_glyph: 0,
            segment_cursor: 0,
            segment_glyph_offset: 0,

            tab_stop
        }
    }
}

impl<'a> Iterator for Line<'a> {
    type Item = LineItem;

    #[inline(always)]
    fn next(&mut self) -> Option<LineItem> {
        loop {
            if self.cur_segment_index >= self.segment_range.len() {
                return None;
            }

            match self.cur_segment {
                Some(segment) => match segment.shaped_glyphs.get(self.cur_glyph).cloned() {
                    Some(mut glyph) => {
                        self.cur_glyph += 1;
                        glyph.pos.x += self.segment_cursor + self.segment_glyph_offset;

                        match segment.text[glyph.word_str_index..].chars().next().unwrap_or(' ').is_whitespace() {
                            false => {
                                glyph.pos += self.glyph_offset;
                                return Some(LineItem::Glyph(glyph));
                            },
                            true if glyph.glyph_index == self.tab_stop.glyph_index => {
                                let advance_to_stop = self.tab_stop.advance - (glyph.pos.x % self.tab_stop.advance);
                                self.segment_glyph_offset += advance_to_stop - self.tab_stop.tab_glyph_advance;
                            },
                            true => continue
                        }
                    },
                    None => {
                        let word_advance = segment.advance + self.segment_glyph_offset;

                        self.cur_segment_index += 1;
                        self.cur_glyph = 0;
                        self.segment_cursor += segment.advance + self.segment_glyph_offset;
                        self.segment_glyph_offset = 0;
                        self.cur_segment = None;

                        return Some(LineItem::WordEnd(word_advance))
                    }
                },
                None => self.cur_segment = self.shaped_text.get_segment(self.segment_range.start + self.cur_segment_index)
            }
        }
    }
}

impl LineItem {
    #[inline(always)]
    fn to_glyph_opt(self) -> Option<ShapedGlyph> {
        match self {
            LineItem::Glyph(glyph) => Some(glyph),
            LineItem::WordEnd(..) => None
        }
    }
}
