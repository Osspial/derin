mod image;
mod text;

use cgmath::Point2;
use cgmath_geometry::{GeoBox, BoundBox};
use glyphydog::{ShapedBuffer, Shaper, FaceSize, DPI};

use gullery::glsl::{Nu8, Ni32};
use gullery::colors::Rgba;

use gl_render::GLVertex;
use gl_render::atlas::Atlas;
use gl_render::font_cache::FontCache;

use theme::Theme;
use core::render::Theme as CoreTheme;

use self::image::ImageTranslate;
use self::text::TextTranslate;

pub use self::text::{EditString, RenderString};


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemedPrim {
    pub theme_path: *const str,
    pub min: Point2<RelPoint>,
    pub max: Point2<RelPoint>,
    pub prim: Prim
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelPoint {
    pub frac_origin: f32,
    pub pixel_pos: i32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prim {
    Image,
    String(*const RenderString),
    EditString(*const EditString)
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

    pub(in gl_render) fn translate_prims<I>(
        &mut self,
        parent_rect: BoundBox<Point2<i32>>,
        theme: &Theme,
        atlas: &mut Atlas,
        font_cache: &mut FontCache,
        dpi: DPI,
        prims: I,

        vertex_buf: &mut Vec<GLVertex>
    )
        where I: IntoIterator<Item=ThemedPrim>
    {
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
                    let atlas_rect = atlas.image_rect(theme_path, || (&image.pixels, image.dims)).cast::<u16>().unwrap();

                    vertex_buf.extend(ImageTranslate::new(
                        abs_rect,
                        atlas_rect,
                        Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                        image.rescale
                    ));
                },
                (Prim::String(render_string), _, Some(theme_text)) => {
                    match font_cache.face(theme_text.face.clone()) {
                        Ok(face) => {
                            let render_string = unsafe{ &*render_string };

                            vertex_buf.extend(TextTranslate::new_rs(
                                abs_rect,
                                theme_text.clone(),
                                face,
                                dpi,
                                atlas,
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
                    match font_cache.face(theme_text.face.clone()) {
                        Ok(face) => {
                            let edit_string = unsafe{ &*edit_string };

                            vertex_buf.extend(TextTranslate::new_es(
                                abs_rect,
                                theme_text.clone(),
                                face,
                                dpi,
                                atlas,
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
                _ => {
                } //TODO: log
            }
        }

        self.shaped_text.clear();
    }
}
