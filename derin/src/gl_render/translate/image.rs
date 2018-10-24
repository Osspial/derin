// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::gl_render::GLVertex;
use gullery::image_format::Rgba;

use crate::cgmath::{Point2, EuclideanSpace};
use cgmath_geometry::{D2, rect::{OffsetBox, BoundBox, GeoBox}};

use crate::theme::RescaleRules;

use derin_common_types::layout::{Align, Margins};

pub(in crate::gl_render) struct ImageTranslate {
    verts: TranslateVerts,
    rect: Option<BoundBox<D2, i32>>,
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
    pub fn new(rect: BoundBox<D2, i32>, clip: BoundBox<D2, i32>, atlas_rect: OffsetBox<D2, u16>, color: Rgba<u8>, rescale: RescaleRules) -> ImageTranslate {
        let clipped_rect = match clip.intersect_rect(rect) {
            Some(clipped_rect) => clipped_rect,
            None => return ImageTranslate {
                verts: TranslateVerts::None,
                rect: None,
                cur_vertex: 0
            }
        };
        let (min, max) = (clipped_rect.min(), clipped_rect.max());
        let gen_corners = || {
            let clip_margins = Margins {
                left: clipped_rect.min().x - rect.min().x,
                right: rect.max().x - clipped_rect.max().x,
                top: clipped_rect.min().y - rect.min().y,
                bottom: rect.max().y - clipped_rect.max().y
            };
            let mut atlas_rect_clipped = BoundBox::from(atlas_rect).cast::<f32>().unwrap();
            let atlas_clip_margins = Margins {
                left: clip_margins.left as f32 * (atlas_rect_clipped.width() / rect.width() as f32),
                right: clip_margins.right as f32 * (atlas_rect_clipped.width() / rect.width() as f32),
                top: clip_margins.top as f32 * (atlas_rect_clipped.height() / rect.height() as f32),
                bottom: clip_margins.bottom as f32 * (atlas_rect_clipped.height() / rect.height() as f32),
            };
            atlas_rect_clipped.min.x += atlas_clip_margins.left;
            atlas_rect_clipped.max.x -= atlas_clip_margins.right;
            atlas_rect_clipped.min.y += atlas_clip_margins.top;
            atlas_rect_clipped.max.y -= atlas_clip_margins.bottom;

            let tl_out = GLVertex {
                loc: min.cast::<f32>().unwrap(),
                color,
                tex_coord: atlas_rect_clipped.min()
            };
            let tr_out = GLVertex {
                loc: Point2::new(max.x as f32, min.y as f32),
                color,
                tex_coord: Point2::new(atlas_rect_clipped.max().x, atlas_rect_clipped.min().y)
            };
            let br_out = GLVertex {
                loc: max.cast::<f32>().unwrap(),
                color,
                tex_coord: atlas_rect_clipped.max()
            };
            let bl_out = GLVertex {
                loc: Point2::new(min.x as f32, max.y as f32),
                color,
                tex_coord: Point2::new(atlas_rect_clipped.min().x, atlas_rect_clipped.max().y)
            };
            (tl_out, tr_out, br_out, bl_out, clip_margins, atlas_clip_margins)
        };

        macro_rules! derived_verts {
            ($base:expr, $sign_x:tt ($loc_slice_x:expr, $atlas_slice_x:expr), $sign_y:tt ($loc_slice_y:expr, $atlas_slice_y:expr)) => {{
                [
                    $base,
                    GLVertex {
                        loc: Point2::new($base.loc.x $sign_x $loc_slice_x, $base.loc.y),
                        tex_coord: Point2::new($base.tex_coord.x $sign_x $atlas_slice_x $sign_x 0.5, $base.tex_coord.y),
                        ..$base
                    },
                    GLVertex {
                        loc: Point2::new($base.loc.x $sign_x $loc_slice_x, $base.loc.y $sign_y $loc_slice_y),
                        tex_coord: Point2::new($base.tex_coord.x $sign_x $atlas_slice_x $sign_x 0.5, $base.tex_coord.y $sign_y $atlas_slice_y $sign_y 0.5),
                        ..$base
                    },
                    GLVertex {
                        loc: Point2::new($base.loc.x, $base.loc.y $sign_y $loc_slice_y),
                        tex_coord: Point2::new($base.tex_coord.x, $base.tex_coord.y $sign_y $atlas_slice_y $sign_y 0.5),
                        ..$base
                    },
                ]
            }}
        }
        let (verts, rect_out);
        match (min == max, rescale) {
            (true, _) => {
                rect_out = None;
                verts = TranslateVerts::None;
            },
            (false, RescaleRules::Stretch) => {
                let (tl_out, tr_out, br_out, bl_out, _, _) = gen_corners();
                rect_out = Some(rect);
                verts = TranslateVerts::Stretch {
                    tl: tl_out,
                    tr: tr_out,
                    br: br_out,
                    bl: bl_out,
                };
            },
            (false, RescaleRules::StretchOnPixelCenter) => {
                let (tl_out, tr_out, br_out, bl_out, _, _) = gen_corners();
                rect_out = Some(rect);
                verts = TranslateVerts::Stretch {
                    tl: derived_verts!(tl_out, +(0., 0.), +(0., 0.))[2],
                    tr: derived_verts!(tr_out, -(0., 0.), +(0., 0.))[2],
                    br: derived_verts!(br_out, -(0., 0.), -(0., 0.))[2],
                    bl: derived_verts!(bl_out, +(0., 0.), -(0., 0.))[2],
                };
            },
            (false, RescaleRules::Slice(mut margins)) => {
                let (tl_out, tr_out, br_out, bl_out, clip_margins, atlas_clip_margins) = gen_corners();
                let margins_width = margins.width();
                if margins_width as i32 > rect.width() {
                    margins.left -= margins_width / 2;
                    margins.right -= (margins_width + 1) / 2;
                }
                let margins_height = margins.height();
                if margins_height as i32 > rect.height() {
                    margins.top -= margins_height / 2;
                    margins.bottom -= (margins_height + 1) / 2;
                }

                let atlas_margins = Margins::new(
                    margins.left as f32 - atlas_clip_margins.left,
                    margins.top as f32 - atlas_clip_margins.top,
                    margins.right as f32 - atlas_clip_margins.right,
                    margins.bottom as f32 - atlas_clip_margins.bottom,
                );
                let loc_margins = Margins::new(
                    (margins.left as i32 - clip_margins.left).max(0) as f32,
                    (margins.top as i32 - clip_margins.top).max(0) as f32,
                    (margins.right as i32 - clip_margins.right).max(0) as f32,
                    (margins.bottom as i32 - clip_margins.bottom).max(0) as f32
                );

                rect_out = Some(rect);
                verts = TranslateVerts::Slice {
                    tl: derived_verts!(tl_out, +(loc_margins.left, atlas_margins.left), +(loc_margins.top, atlas_margins.top)),
                    tr: derived_verts!(tr_out, -(loc_margins.right, atlas_margins.right), +(loc_margins.top, atlas_margins.top)),
                    br: derived_verts!(br_out, -(loc_margins.right, atlas_margins.right), -(loc_margins.bottom, atlas_margins.bottom)),
                    bl: derived_verts!(bl_out, +(loc_margins.left, atlas_margins.left), -(loc_margins.bottom, atlas_margins.bottom)),
                };
            }
            (false, RescaleRules::Align(alignment)) => {
                let get_dims = |align, atlas_size, fill_size| {
                    let (min, max) = match align {
                        Align::Start => (0, atlas_size),
                        Align::Center => ((fill_size - atlas_size) / 2, (fill_size + atlas_size) / 2),
                        Align::End => (fill_size - atlas_size, fill_size),
                        Align::Stretch => (0, fill_size)
                    };
                    (min, max)
                };

                let (min_x, max_x) = get_dims(alignment.x, atlas_rect.width() as i32, rect.width());
                let (min_y, max_y) = get_dims(alignment.y, atlas_rect.height() as i32, rect.height());

                let bound_x = |i: i32| i.min(clipped_rect.max().x).max(clipped_rect.min().x);
                let bound_y = |i: i32| i.min(clipped_rect.max().y).max(clipped_rect.min().y);
                rect_out = Some(BoundBox::new2(min_x, min_y, max_x, max_y) + rect.min.to_vec());
                let bounds = BoundBox::new2(
                    bound_x(min_x + rect.min.x),
                    bound_y(min_y + rect.min.y),
                    bound_x(max_x + rect.min.x),
                    bound_y(max_y + rect.min.y),
                );
                let clip_margins = Margins {
                    left: min_x - (bounds.min.x - rect.min.x),
                    right: max_x - (bounds.max.x - rect.min.x),
                    top: min_y - (bounds.min.y - rect.min.y),
                    bottom: max_y - (bounds.max.y - rect.min.y),
                };

                let mut atlas_rect_clipped = BoundBox::from(atlas_rect).cast::<f32>().unwrap();
                let atlas_clip_margins = Margins {
                    left: clip_margins.left as f32 * (atlas_rect_clipped.width() / rect.width() as f32),
                    right: clip_margins.right as f32 * (atlas_rect_clipped.width() / rect.width() as f32),
                    top: clip_margins.top as f32 * (atlas_rect_clipped.height() / rect.height() as f32),
                    bottom: clip_margins.bottom as f32 * (atlas_rect_clipped.height() / rect.height() as f32),
                };
                atlas_rect_clipped.min.x += atlas_clip_margins.left;
                atlas_rect_clipped.max.x -= atlas_clip_margins.right;
                atlas_rect_clipped.min.y += atlas_clip_margins.top;
                atlas_rect_clipped.max.y -= atlas_clip_margins.bottom;

                verts = TranslateVerts::Stretch {
                    tl: GLVertex {
                        loc: bounds.min().cast::<f32>().unwrap(),
                        color,
                        tex_coord: atlas_rect_clipped.min()
                    },
                    tr: GLVertex {
                        loc: Point2::new(bounds.max.x as f32, bounds.min.y as f32),
                        color,
                        tex_coord: Point2::new(atlas_rect_clipped.max().x, atlas_rect_clipped.min().y)
                    },
                    br: GLVertex {
                        loc: bounds.max.cast::<f32>().unwrap(),
                        color,
                        tex_coord: atlas_rect_clipped.max()
                    },
                    bl: GLVertex {
                        loc: Point2::new(bounds.min.x as f32, bounds.max.y as f32),
                        color,
                        tex_coord: Point2::new(atlas_rect_clipped.min().x, atlas_rect_clipped.max().y)
                    }
                };
            }
        };

        ImageTranslate {
            verts,
            rect: rect_out,
            cur_vertex: 0
        }
    }

    pub fn rect(&self) -> Option<BoundBox<D2, i32>> {
        self.rect
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
