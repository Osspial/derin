use gl_render::GLVertex;
use gl_raii::glsl::Nu8;
use gl_raii::colors::Rgba;

use cgmath::Point2;
use cgmath_geometry::{OffsetBox, BoundBox, GeoBox};

use theme::RescaleRules;

use dct::hints::Margins;

pub(in gl_render) struct ImageTranslate {
    verts: TranslateVerts,
    cur_vertex: usize
}

enum TranslateVerts {
    Stretch {
        tl: GLVertex,
        tr: GLVertex,
        br: GLVertex,
        bl: GLVertex,
    },
    Slice {
        tl: [GLVertex; 4],
        tr: [GLVertex; 4],
        br: [GLVertex; 4],
        bl: [GLVertex; 4],
    },
    None
}

impl ImageTranslate {
    pub fn new(rect: BoundBox<Point2<i32>>, atlas_rect: OffsetBox<Point2<u16>>, color: Rgba<Nu8>, rescale: RescaleRules) -> ImageTranslate {
        let (min, max) = (rect.min(), rect.max());
        let atlas_rect = atlas_rect.cast::<f32>().unwrap();
        let verts = match (min == max, rescale) {
            (true, _) => TranslateVerts::None,
            (false, RescaleRules::Stretch) => TranslateVerts::Stretch {
                tl: GLVertex {
                    loc: min,
                    color,
                    tex_coord: atlas_rect.min()
                },
                tr: GLVertex {
                    loc: Point2::new(max.x, min.y),
                    color,
                    tex_coord: Point2::new(atlas_rect.max().x, atlas_rect.min().y)
                },
                br: GLVertex {
                    loc: max,
                    color,
                    tex_coord: atlas_rect.max()
                },
                bl: GLVertex {
                    loc: Point2::new(min.x, max.y),
                    color,
                    tex_coord: Point2::new(atlas_rect.min().x, atlas_rect.max().y)
                },
            },
            (false, RescaleRules::Slice(margins)) => {
                let margins = Margins::new(margins.left as f32, margins.top as f32, margins.right as f32, margins.bottom as f32);
                let tl_out = GLVertex {
                    loc: min,
                    color,
                    tex_coord: atlas_rect.min()
                };
                let tr_out = GLVertex {
                    loc: Point2::new(max.x, min.y),
                    color,
                    tex_coord: Point2::new(atlas_rect.max().x, atlas_rect.min().y)
                };
                let br_out = GLVertex {
                    loc: max,
                    color,
                    tex_coord: atlas_rect.max()
                };
                let bl_out = GLVertex {
                    loc: Point2::new(min.x, max.y),
                    color,
                    tex_coord: Point2::new(atlas_rect.min().x, atlas_rect.max().y)
                };

                macro_rules! derived_verts {
                    ($base:expr, $sign_x:tt $slice_x:expr, $sign_y:tt $slice_y:expr) => {{
                        [
                            $base,
                            GLVertex {
                                loc: Point2::new($base.loc.x $sign_x $slice_x as i32, $base.loc.y),
                                tex_coord: Point2::new($base.tex_coord.x $sign_x $slice_x $sign_x 0.5, $base.tex_coord.y),
                                ..$base
                            },
                            GLVertex {
                                loc: Point2::new($base.loc.x $sign_x $slice_x as i32, $base.loc.y $sign_y $slice_y as i32),
                                tex_coord: Point2::new($base.tex_coord.x $sign_x $slice_x $sign_x 0.5, $base.tex_coord.y $sign_y $slice_y $sign_y 0.5),
                                ..$base
                            },
                            GLVertex {
                                loc: Point2::new($base.loc.x, $base.loc.y $sign_y $slice_y as i32),
                                tex_coord: Point2::new($base.tex_coord.x, $base.tex_coord.y $sign_y $slice_y $sign_y 0.5),
                                ..$base
                            },
                        ]
                    }}
                }

                TranslateVerts::Slice {
                    tl: derived_verts!(tl_out, +margins.left, +margins.top),
                    tr: derived_verts!(tr_out, -margins.right, +margins.top),
                    br: derived_verts!(br_out, -margins.right, -margins.bottom),
                    bl: derived_verts!(bl_out, +margins.left, -margins.bottom),
                }
            }
        };

        ImageTranslate {
            verts,
            cur_vertex: 0
        }
    }
}

impl Iterator for ImageTranslate {
    type Item = GLVertex;

    #[inline]
    fn next(&mut self) -> Option<GLVertex> {
        let ret = match self.verts {
            TranslateVerts::Stretch{tl, tr, br, bl} => {
                let tris = [
                    tl, tr, br,
                    br, bl, tl
                ];

                tris.get(self.cur_vertex).cloned()
            },
            TranslateVerts::Slice{tl, tr, br, bl} => {
                let tris = [
                    tl[0], tl[1], tl[2],
                    tl[2], tl[3], tl[0],

                        tl[1], tr[1], tr[2],
                        tr[3], tl[2], tl[1],

                    tr[0], tr[1], tr[2],
                    tr[2], tr[3], tr[0],

                        tr[3], br[3], br[2],
                        br[2], tr[2], tr[3],

                    br[0], br[1], br[2],
                    br[2], br[3], br[0],

                        br[1], bl[1], bl[2],
                        bl[3], br[2], br[1],

                    bl[0], bl[1], bl[2],
                    bl[2], bl[3], bl[0],

                        tl[3], bl[3], bl[2],
                        bl[2], tl[2], tl[3],

                    tl[2], tr[2], br[2],
                    br[2], bl[2], tl[2],
                ];

                tris.get(self.cur_vertex).cloned()
            },
            TranslateVerts::None => None
        };

        self.cur_vertex += 1;
        ret
    }
}
