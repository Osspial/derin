// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod image;
mod text;

use crate::cgmath::{Point2, EuclideanSpace};
use cgmath_geometry::{D2, rect::{GeoBox, OffsetBox, BoundBox}};
use glyphydog::{ShapedBuffer, Shaper, FaceSize, DPI};

use gullery::image_format::Rgba;

use crate::gl_render::{FrameDraw, GLFrame, PrimFrame};

use crate::theme::Theme;
use crate::core::render::Theme as CoreTheme;

use self::image::ImageTranslate;
use self::text::TextTranslate;

pub use self::text::RenderString;

use std::mem;


#[derive(Debug, PartialEq)]
pub struct ThemedPrim<D> {
    pub theme_path: *const str,
    pub min: Point2<RelPoint>,
    pub max: Point2<RelPoint>,
    pub prim: Prim<D>,
    /// Optionally outputs the widget's transformed pixel rectangle.
    pub rect_px_out: Option<*mut BoundBox<D2, i32>>
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelPoint {
    pub frac_origin: f32,
    pub pixel_pos: i32
}

#[derive(Debug, PartialEq, Eq)]
pub enum Prim<D> {
    Image,
    String(*mut RenderString),
    DirectRender(*mut FnMut(&mut D))
}

impl RelPoint {
    #[inline]
    pub fn new(frac_origin: f32, pixel_pos: i32) -> RelPoint {
        RelPoint{ frac_origin, pixel_pos }
    }
}

pub struct Translator {
    shaped_text: ShapedBuffer,
    shaper: Shaper,
}

impl Translator {
    pub fn new() -> Translator {
        Translator {
            shaped_text: ShapedBuffer::new(),
            shaper: Shaper::new()
        }
    }

    pub(in crate::gl_render) fn translate_prims(
        &mut self,
        parent_rect: BoundBox<D2, i32>,
        clip_rect: BoundBox<D2, i32>,
        theme: &Theme,
        dpi: DPI,
        prims: impl IntoIterator<Item=ThemedPrim<<GLFrame as PrimFrame>::DirectRender>>,
        draw: &mut FrameDraw
    ) {
        let prim_rect_iter = prims.into_iter().map(move |p| {
            let parent_center = parent_rect.center();
            let parent_dims = parent_rect.dims();

            let bl = Point2 {
                x: parent_center.x + (parent_dims.width() as f32 * p.min.x.frac_origin) as i32 / 2 + p.min.x.pixel_pos,
                y: parent_center.y + (parent_dims.height() as f32 * p.min.y.frac_origin) as i32 / 2 + p.min.y.pixel_pos
            };
            let tr = Point2 {
                x: parent_center.x + (parent_dims.width() as f32 * p.max.x.frac_origin) as i32 / 2 + p.max.x.pixel_pos,
                y: parent_center.y + (parent_dims.height() as f32 * p.max.y.frac_origin) as i32 / 2 + p.max.y.pixel_pos
            };
            (BoundBox::new2(bl.x, bl.y, tr.x, tr.y), p)
        });

        for (mut abs_rect, prim) in prim_rect_iter {
            let theme_path = unsafe{ &*prim.theme_path };
            let widget_theme = theme.widget_theme(theme_path);

            if let Some(parent_clipped) = clip_rect.intersect_rect(parent_rect) {
                match (prim.prim, widget_theme.image, widget_theme.text) {
                    (Prim::Image, Some(image), _) => {
                        let atlas_rect = draw.atlas.image_rect(theme_path, || (&image.pixels, image.dims)).cast::<u16>().unwrap();

                        let abs_rect_dims = abs_rect.dims();
                        let abs_rect_dims_bounded = image.size_bounds.bound_rect(abs_rect_dims);
                        abs_rect.max.x = abs_rect.min.x + abs_rect_dims_bounded.width();
                        abs_rect.max.y = abs_rect.min.y + abs_rect_dims_bounded.height();
                        abs_rect = abs_rect + (abs_rect_dims.dims - abs_rect_dims_bounded.dims) / 2;

                        let image_translate = ImageTranslate::new(
                            abs_rect,
                            parent_clipped,
                            atlas_rect,
                            Rgba::new(255, 255, 255, 255),
                            image.rescale
                        );
                        if let (Some(rect_px_out), Some(image_rect)) = (prim.rect_px_out, image_translate.rect()) {
                            unsafe{ *rect_px_out = image_rect - parent_rect.min().to_vec() };
                        }

                        draw.vertices.extend(image_translate);
                    },
                    (Prim::String(render_string), _, Some(theme_text)) => {
                        match draw.font_cache.face(theme_text.face.clone()) {
                            Ok(face) => {
                                let render_string = unsafe{ &mut *render_string };

                                let vertex_iter = TextTranslate::new(
                                    abs_rect,
                                    parent_clipped,
                                    theme_text.clone(),
                                    face,
                                    dpi,
                                    &mut draw.atlas,
                                    |string, face| {
                                        self.shaper.shape_text(
                                            string,
                                            face,
                                            FaceSize::new(theme_text.face_size, theme_text.face_size),
                                            dpi,
                                            &mut self.shaped_text
                                        ).ok();
                                        &self.shaped_text
                                    },
                                    render_string
                                );
                                draw.vertices.extend(vertex_iter);
                                if let (Some(rect_px_out), Some(text_rect)) = (prim.rect_px_out, render_string.text_rect()) {
                                    unsafe{ *rect_px_out = text_rect + abs_rect.min().to_vec() - parent_rect.min().to_vec() };
                                }
                            },
                            Err(_) => {
                                //TODO: log
                            }
                        }
                    },
                    (Prim::DirectRender(render_fn), _, _) => {
                        draw.draw_contents();
                        let render_fn = unsafe{ &mut *render_fn };
                        let mut framebuffer = unsafe{ mem::uninitialized() };
                        mem::swap(&mut framebuffer, &mut draw.fb);

                        let viewport_origin = Point2::new(abs_rect.min().x.max(0) as u32, abs_rect.min().y.max(0) as u32);
                        let viewport_rect = OffsetBox::new2(
                            viewport_origin.x,
                            viewport_origin.y,
                            (abs_rect.width() - (viewport_origin.x as i32 - abs_rect.min().x)) as u32,
                            (abs_rect.height() - (viewport_origin.y as i32 - abs_rect.min().y)) as u32,
                        );
                        let mut draw_tuple = (framebuffer, viewport_rect, draw.context_state.clone());
                        render_fn(&mut draw_tuple);
                        mem::swap(&mut draw_tuple.0, &mut draw.fb);
                        mem::forget(draw_tuple.0);
                    }
                    _ => {
                    } //TODO: log
                }
            }
        }

        self.shaped_text.clear();
    }
}
