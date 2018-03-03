use gl_render::GLVertex;
use gullery::glsl::Nu8;
use gullery::colors::Rgba;

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
    pub fn new(rect: BoundBox<Point2<i32>>, clip: BoundBox<Point2<i32>>, atlas_rect: OffsetBox<Point2<u16>>, color: Rgba<Nu8>, rescale: RescaleRules) -> ImageTranslate {
        let clipped_rect = match clip.intersect_rect(rect) {
            Some(clipped_rect) => clipped_rect,
            None => return ImageTranslate {
                verts: TranslateVerts::None,
                cur_vertex: 0
            }
        };
        let (min, max) = (clipped_rect.min(), clipped_rect.max());

        let clip_margins = Margins {
            left: clipped_rect.min().x - rect.min().x,
            right: rect.max().x - clipped_rect.max().x,
            top: clipped_rect.min().y - rect.min().y,
            bottom: rect.max().y - clipped_rect.max().y
        };
        let mut atlas_rect = BoundBox::from(atlas_rect).cast::<f32>().unwrap();
        let atlas_clip_margins = Margins {
            left: clip_margins.left as f32 * (atlas_rect.width() / rect.width() as f32),
            right: clip_margins.right as f32 * (atlas_rect.width() / rect.width() as f32),
            top: clip_margins.top as f32 * (atlas_rect.height() / rect.height() as f32),
            bottom: clip_margins.bottom as f32 * (atlas_rect.height() / rect.height() as f32),
        };
        atlas_rect.min.x += atlas_clip_margins.left;
        atlas_rect.max.x -= atlas_clip_margins.right;
        atlas_rect.min.y += atlas_clip_margins.top;
        atlas_rect.max.y -= atlas_clip_margins.bottom;

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
            ($base:expr, $sign_x:tt ($loc_slice_x:expr, $atlas_slice_x:expr), $sign_y:tt ($loc_slice_y:expr, $atlas_slice_y:expr)) => {{
                [
                    $base,
                    GLVertex {
                        loc: Point2::new($base.loc.x $sign_x $loc_slice_x as i32, $base.loc.y),
                        tex_coord: Point2::new($base.tex_coord.x $sign_x $atlas_slice_x $sign_x 0.5, $base.tex_coord.y),
                        ..$base
                    },
                    GLVertex {
                        loc: Point2::new($base.loc.x $sign_x $loc_slice_x as i32, $base.loc.y $sign_y $loc_slice_y as i32),
                        tex_coord: Point2::new($base.tex_coord.x $sign_x $atlas_slice_x $sign_x 0.5, $base.tex_coord.y $sign_y $atlas_slice_y $sign_y 0.5),
                        ..$base
                    },
                    GLVertex {
                        loc: Point2::new($base.loc.x, $base.loc.y $sign_y $loc_slice_y as i32),
                        tex_coord: Point2::new($base.tex_coord.x, $base.tex_coord.y $sign_y $atlas_slice_y $sign_y 0.5),
                        ..$base
                    },
                ]
            }}
        }
        let verts = match (min == max, rescale) {
            (true, _) => TranslateVerts::None,
            (false, RescaleRules::Stretch) => TranslateVerts::Stretch {
                tl: tl_out,
                tr: tr_out,
                br: br_out,
                bl: bl_out,
            },
            (false, RescaleRules::StretchOnPixelCenter) => TranslateVerts::Stretch {
                tl: derived_verts!(tl_out, +(0., 0.), +(0., 0.))[2],
                tr: derived_verts!(tr_out, -(0., 0.), +(0., 0.))[2],
                br: derived_verts!(br_out, -(0., 0.), -(0., 0.))[2],
                bl: derived_verts!(bl_out, +(0., 0.), -(0., 0.))[2],
            },
            (false, RescaleRules::Slice(margins)) => {
                let atlas_margins = Margins::new(
                    margins.left as f32 - atlas_clip_margins.left,
                    margins.top as f32 - atlas_clip_margins.right,
                    margins.right as f32 - atlas_clip_margins.top,
                    margins.bottom as f32 - atlas_clip_margins.bottom,
                );
                let loc_margins = Margins::new(
                    (margins.left as i32 - clip_margins.left).max(0) as f32,
                    (margins.top as i32 - clip_margins.top).max(0) as f32,
                    (margins.right as i32 - clip_margins.right).max(0) as f32,
                    (margins.bottom as i32 - clip_margins.bottom).max(0) as f32
                );

                TranslateVerts::Slice {
                    tl: derived_verts!(tl_out, +(loc_margins.left, atlas_margins.left), +(loc_margins.top, atlas_margins.top)),
                    tr: derived_verts!(tr_out, -(loc_margins.right, atlas_margins.right), +(loc_margins.top, atlas_margins.top)),
                    br: derived_verts!(br_out, -(loc_margins.right, atlas_margins.right), -(loc_margins.bottom, atlas_margins.bottom)),
                    bl: derived_verts!(bl_out, +(loc_margins.left, atlas_margins.left), -(loc_margins.bottom, atlas_margins.bottom)),
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
