mod image;
mod text;

use cgmath::{Point2, Vector2, Array};
use cgmath_geometry::{GeoBox, BoundBox};
use glyphydog::{ShapedBuffer, Shaper, FaceSize, DPI};

use gl_raii::glsl::{Nu8, Ni32};
use gl_raii::colors::Rgba;

use gl_render::GLVertex;
use gl_render::atlas::Atlas;
use gl_render::font_cache::FontCache;

use theme::Theme;
use core::render::Theme as CoreTheme;

use self::image::ImageTranslate;
use self::text::TextTranslate;


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
    Text(*const str)
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
        parent_rect: BoundBox<Point2<u32>>,
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
            let parent_center = parent_rect.center().cast::<i32>().unwrap_or(Point2::from_value(0));
            let parent_dims = parent_rect.dims().cast::<i32>().unwrap_or(Vector2::from_value(0));

            let bl = Point2 {
                x: (parent_center.x + parent_dims.x * Ni32::from_bounded(p.min.x.frac_origin) / 2) as u32,
                y: (parent_center.y + parent_dims.y * Ni32::from_bounded(p.min.y.frac_origin) / 2) as u32
            };
            let tr = Point2 {
                x: (parent_center.x + parent_dims.x * Ni32::from_bounded(p.max.x.frac_origin) / 2) as u32,
                y: (parent_center.y + parent_dims.y * Ni32::from_bounded(p.max.y.frac_origin) / 2) as u32
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
                (Prim::Text(string), _, Some(theme_text)) => {
                    match font_cache.face(theme_text.face.clone()) {
                        Ok(face) => {
                            self.shaper.shape_text(
                                unsafe{ &*string },
                                face,
                                FaceSize::new(theme_text.face_size, theme_text.face_size),
                                dpi,
                                &mut self.shaped_text
                            ).ok();

                            vertex_buf.extend(TextTranslate::new(abs_rect, &self.shaped_text, theme_text, face, dpi, atlas));
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
