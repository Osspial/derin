use gl_render::GLVertex;
use gl_render::atlas::Atlas;
use gl_render::translate::image::ImageTranslate;
use theme::{ThemeText, RescaleRules};

use cgmath::{EuclideanSpace, ElementWise, Vector2};
use cgmath_geometry::{BoundRect, DimsRect, OffsetRect, Rectangle};

use gl_raii::colors::Rgba;
use gl_raii::glsl::Nu8;

use glyphydog::{ShapedBuffer, ShapedGlyph, Face, FaceSize, DPI, LoadFlags, RenderMode};

use std::ops::Range;


pub(in gl_render) struct TextTranslate<'a> {
    shaped_text: &'a ShapedBuffer,
    face: &'a mut Face<()>,
    atlas: &'a mut Atlas,
    text_style: ThemeText,
    dpi: DPI,

    rect: BoundRect<u32>,
    active_line: Line<'a>,
    line_num: u32,

    glyph_verts: Option<ImageTranslate>
}

struct Line<'a> {
    shaped_text: &'a ShapedBuffer,
    segment_range: Range<usize>,

    cur_segment: usize,
    cur_glyph: usize,
    segment_cursor: i32
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
        TextTranslate {
            shaped_text, face, atlas, text_style, dpi,

            rect,
            active_line: Line::from_segments(0, rect.width(), shaped_text),
            line_num: 0,

            glyph_verts: None
        }
    }
}

impl<'a> Iterator for TextTranslate<'a> {
    type Item = GLVertex;

    fn next(&mut self) -> Option<GLVertex> {
        'get_vert: loop {
            match self.glyph_verts.as_mut().map(|v| v.next()).unwrap_or(None) {
                Some(vert) => return Some(vert),
                None => 'set_glyph: loop {
                    match self.active_line.next() {
                        Some(next_glyph) => {
                            let TextTranslate {
                                ref mut atlas,
                                ref mut face,
                                ref mut glyph_verts,
                                ref text_style,
                                dpi,
                                ..
                            } = *self;

                            let face_size = FaceSize::new(text_style.face_size, text_style.face_size);

                            let render_mode = RenderMode::Normal;
                            let (atlas_rect, glyph_bearing) = atlas.glyph_rect(
                                text_style.face.clone(),
                                text_style.face_size,
                                next_glyph.glyph_index,
                                || {
                                    let glyph_res = face.load_glyph(
                                        next_glyph.glyph_index,
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
                                self.rect.min().cast::<i32>().unwrap() +
                                // Move the point down the correct number of lines
                                Vector2::new(0, line_height * (self.line_num as i32 + 1)) +
                                // Advance the cursor down the line. Pos is with TLO, so vertical flip
                                Vector2::new(1, -1).mul_element_wise(next_glyph.pos.to_vec() + glyph_bearing);
                            let glyph_rect = BoundRect::new(
                                glyph_pos.x,
                                glyph_pos.y,
                                glyph_pos.x + atlas_rect.width() as i32,
                                glyph_pos.y + atlas_rect.height() as i32
                            ).cast::<u32>().unwrap_or(BoundRect::new(0, 0, 0, 0));

                            *glyph_verts = Some(ImageTranslate::new(
                                glyph_rect,
                                atlas_rect.cast::<u16>().unwrap_or(OffsetRect::new(0, 0, 0, 0)),
                                text_style.color,
                                RescaleRules::Stretch
                            ));
                            continue 'get_vert;
                        },
                        None => match self.active_line.segment_range.end == self.shaped_text.segments_len() {
                            true => return None,
                            false => {
                                self.active_line = Line::from_segments(
                                    self.active_line.segment_range.end,
                                    self.rect.width(),
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
    fn from_segments(index: usize, max_len_px: u32, shaped_text: &'a ShapedBuffer) -> Line<'a> {
        let mut end_index = index;
        let mut len_px = 0;

        while let Some(segment) = shaped_text.get_segment(end_index) {
            // if segment.advance < 0 {
            //     // TODO: LOG NEGATIVE ADVANCE
            // }
            if segment.advance + len_px as i32 > max_len_px as i32 {
                break;
            }

            len_px += segment.advance as u32;
            end_index += 1;

            if segment.hard_break {
                break;
            }
        }

        if end_index == index && shaped_text.get_segment(end_index).is_some() {
            if let Some(_) = shaped_text.get_segment(end_index) {
                end_index += 1;
                // len_px += segment.advance as u32;
            }
        }

        Line {
            shaped_text,
            segment_range: index..end_index,

            cur_segment: 0,
            cur_glyph: 0,
            segment_cursor: 0
        }
    }
}

impl<'a> Iterator for Line<'a> {
    type Item = ShapedGlyph;

    #[inline(always)]
    fn next(&mut self) -> Option<ShapedGlyph> {
        loop {
            if self.cur_segment >= self.segment_range.len() {
                return None;
            }

            // Bounds check should have been done in line creation;
            let segment = self.shaped_text.get_segment(self.segment_range.start + self.cur_segment).unwrap();

            match segment.shaped_glyphs.get(self.cur_glyph).cloned() {
                Some(mut glyph) => {
                    self.cur_glyph += 1;
                    glyph.pos.x += self.segment_cursor;
                    return Some(glyph);
                }
                None => {
                    self.cur_segment += 1;
                    self.cur_glyph = 0;
                    self.segment_cursor += segment.advance;
                }
            }
        }
    }
}

