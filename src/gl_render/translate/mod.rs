mod image;
mod text;

use cgmath::Point2;
use cgmath_geometry::{GeoBox, OffsetBox, BoundBox};
use glyphydog::{ShapedBuffer, Shaper, FaceSize, DPI};

use gullery::glsl::{Nu8, Ni32};
use gullery::colors::Rgba;

use gl_render::{FrameDraw, GLFrame, PrimFrame};

use theme::Theme;
use core::render::Theme as CoreTheme;

use self::image::ImageTranslate;
use self::text::TextTranslate;

pub use self::text::{EditString, RenderString};

use std::mem;


#[derive(Debug, PartialEq)]
pub struct ThemedPrim<D> {
    pub theme_path: *const str,
    pub min: Point2<RelPoint>,
    pub max: Point2<RelPoint>,
    pub prim: Prim<D>
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelPoint {
    pub frac_origin: f32,
    pub pixel_pos: i32
}

#[derive(Debug, PartialEq, Eq)]
pub enum Prim<D> {
    Image,
    String(*const RenderString),
    EditString(*const EditString),
    DirectRender(*const Fn(&mut D))
}

impl<D> Clone for Prim<D> {
    fn clone(&self) -> Prim<D> {
        match *self {
            Prim::Image => Prim::Image,
            Prim::String(s) => Prim::String(s),
            Prim::EditString(s) => Prim::EditString(s),
            Prim::DirectRender(f) => Prim::DirectRender(f)
        }
    }
}
impl<D> Clone for ThemedPrim<D> {
    fn clone(&self) -> ThemedPrim<D> {
        ThemedPrim {
            theme_path: self.theme_path,
            min: self.min,
            max: self.max,
            prim: self.prim,
        }
    }
}
impl<D> Copy for Prim<D> {}
impl<D> Copy for ThemedPrim<D> {}

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

    pub(in gl_render) fn translate_prims(
        &mut self,
        parent_rect: BoundBox<Point2<i32>>,
        theme: &Theme,
        dpi: DPI,
        prims: impl IntoIterator<Item=ThemedPrim<<GLFrame as PrimFrame>::DirectRender>>,

        draw: &mut FrameDraw
    ) {
        let prim_rect_iter = prims.into_iter().map(move |p| {
            let parent_center = parent_rect.center();
            let parent_dims = parent_rect.dims();

            let bl = Point2 {
                x: parent_center.x + parent_dims.x * Ni32::from_bounded(p.min.x.frac_origin) / 2,
                y: parent_center.y + parent_dims.y * Ni32::from_bounded(p.min.y.frac_origin) / 2
            };
            let tr = Point2 {
                x: parent_center.x + parent_dims.x * Ni32::from_bounded(p.max.x.frac_origin) / 2,
                y: parent_center.y + parent_dims.y * Ni32::from_bounded(p.max.y.frac_origin) / 2
            };
            (BoundBox::new2(bl.x, bl.y, tr.x, tr.y), p)
        });

        for (abs_rect, prim) in prim_rect_iter {
            let theme_path = unsafe{ &*prim.theme_path };
            let node_theme = theme.node_theme(theme_path);

            match (prim.prim, node_theme.icon, node_theme.text) {
                (Prim::Image, Some(image), _) => {
                    let atlas_rect = draw.atlas.image_rect(theme_path, || (&image.pixels, image.dims)).cast::<u16>().unwrap();

                    draw.vertices.extend(ImageTranslate::new(
                        abs_rect,
                        parent_rect,
                        atlas_rect,
                        Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                        image.rescale
                    ));
                },
                (Prim::String(render_string), _, Some(theme_text)) => {
                    match draw.font_cache.face(theme_text.face.clone()) {
                        Ok(face) => {
                            let render_string = unsafe{ &*render_string };

                            draw.vertices.extend(TextTranslate::new_rs(
                                abs_rect,
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
                            ));
                        },
                        Err(_) => {
                            //TODO: log
                        }
                    }
                },
                (Prim::EditString(edit_string), _, Some(theme_text)) => {
                    match draw.font_cache.face(theme_text.face.clone()) {
                        Ok(face) => {
                            let edit_string = unsafe{ &*edit_string };

                            draw.vertices.extend(TextTranslate::new_es(
                                abs_rect,
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
                                edit_string
                            ));
                        },
                        Err(_) => {
                            //TODO: log
                        }
                    }
                },
                (Prim::DirectRender(render_fn), _, _) => {
                    draw.draw_contents();
                    let render_fn = unsafe{ &*render_fn };
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

        self.shaped_text.clear();
    }
}
