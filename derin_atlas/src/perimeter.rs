// use std::{cmp, mem};
// use std::ops::Range;

use crate::cgmath::{EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, rect::{DimsBox, BoundBox, OffsetBox, GeoBox}};
use crate::raw::RawAtlas;
use itertools::Itertools;
use std::{
    cmp::{Ordering::{Less, Equal, Greater}, Ord},
};

// TODO: RENAME, SINCE THIS IS MORESO THE CORNER DIRECTION INSTEAD OF
// THE FIRST ITEM IN THE CORNER.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum First {
    Vertical,
    Horizontal,
}
use First::{Vertical, Horizontal};

pub struct PerimeterAtlas<P: 'static + Copy> {
    raw: RawAtlas<P>,
    dims: DimsBox<D2, u32>,

    corners: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Corner {
    first: First,
    v: u32,
    h: u32,
    vd: i32,
    hd: i32,
    vnorm: i32,
    hnorm: i32,
    concave: bool,
}

// TODO: ADD SIMPLIFICATION PASS

impl<P: Copy> PerimeterAtlas<P> {
    pub fn new(dims: DimsBox<D2, u32>, background_color: P) -> PerimeterAtlas<P> {
        let height = dims.height();
        let width = dims.width();
        println!("{:?}", vec![0, width, height, 0]);
        PerimeterAtlas {
            raw: RawAtlas::new(dims, background_color),
            dims,
            corners: vec![0, width, height, 0],
        }
    }

    /// Get the raw pixel data stored in the atlas.
    #[inline]
    pub fn pixels(&self) -> &[P] {
        &self.raw.pixels()
    }

    #[must_use]
    pub fn add_image(&mut self, image_dims: DimsBox<D2, u32>, image_data: &[P]) -> Option<OffsetBox<D2, u32>> {
        self.best_corner(image_dims)
            .map(|offset| {
                self.raw.blit_slice_iter(
                    self.dims,
                    image_data.chunks(image_dims.width() as usize),
                    image_dims,
                    offset,
                );
                OffsetBox::new(Point2::from_vec(offset), image_dims.dims)
            })
    }

    pub fn edge_image(&self, back: P, mut edge: impl FnMut(usize) -> P) -> (DimsBox<D2, u32>, Box<[P]>) {
        let dims = DimsBox::new(self.dims.dims + Vector2::new(1, 1));
        let mut corners_image = RawAtlas::new(dims, back);

        let first_corner = self.corners().next().unwrap();
        for (i, (a, b)) in self.corners().chain(Some(first_corner)).tuple_windows().enumerate() {
            let rect = {
                let x0 = a.h;
                let y0 = a.v;
                let x1 = b.h;
                let y1 = b.v;
                OffsetBox::from(
                    BoundBox::new2(
                        Ord::min(x0, x1),
                        Ord::min(y0, y1),
                        Ord::max(x0, x1) + 1,
                        Ord::max(y0, y1) + 1,
                    )
                )
            };
            let rect = rect.intersect_rect(dims.into()).either();
            let rect = match rect {
                Some(rect) => rect,
                None => continue
            };

            corners_image.blit_pixel_iter(
                dims,
                std::iter::repeat(edge(i)).take(rect.width() as usize * rect.height() as usize),
                rect.dims().cast().unwrap(),
                rect.origin.to_vec().cast().expect(&format!("{:?}", rect)),
            );
        }

        (dims, corners_image.pixels_box())
    }

    fn corners(&self) -> impl '_ + Iterator<Item=Corner> {
        (0..self.corners.len() as isize)
            .map(move |i| {
                let (a, b) = (self.get(i - 1), self.get(i));
                let (ad, bd) = (self.get(i + 1) as i32 - a as i32, b as i32 - self.get(i - 2) as i32);
                let first = match i % 2 {
                    0 => Horizontal,
                    1 => Vertical,
                    _ => unreachable!(),
                };
                let (v, h, vd, hd, concave) = match first {
                    Vertical => (a, b, ad, bd, ad.signum() == bd.signum()),
                    Horizontal => (b, a, bd, ad, ad.signum() != bd.signum()),
                };
                let vnorm = -vd.signum();
                let hnorm = hd.signum();

                Corner {
                    first,
                    v,
                    h,
                    vd,
                    hd,
                    vnorm,
                    hnorm,
                    concave,
                }
            })
    }

    fn best_corner(&mut self, rect: DimsBox<D2, u32>) -> Option<Vector2<u32>> {
        assert_ne!(0, rect.width());
        assert_ne!(0, rect.height());
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct Best {
            corner: usize,
            vnorm: i32,
            hnorm: i32,
            v: u32,
            h: u32,
            vd: i32,
            hd: i32,
            first: First,
            concave: bool,
            rect: BoundBox<D2, i32>,
        }
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        struct BestSort {
            perimeter_added: u32,
            distance_to_edge: u32,
        }

        if self.corners.len() == 2 {
            return None;
        }

        let width = rect.width();
        let height = rect.height();
        let convex_perimeter_added = 2 * (width + height);
        dbg!(convex_perimeter_added);

        let mut best = None;
        let mut best_sort = BestSort {
            perimeter_added: u32::max_value(),
            distance_to_edge: u32::max_value(),
        };
        // let first_corner = [self.corners[self.corners.len() - 1], self.corners[0]];
        // let mut cursor = Point2::new(self.corners[0], self.corners[self.corners.len() - 1]);

        for (i, corner) in self.corners().enumerate() {
            let Corner {
                first,
                v,
                h,
                vd,
                hd,
                vnorm,
                hnorm,
                concave,
            } = corner;

            let vabs = vd.abs() as u32;
            let habs = hd.abs() as u32;

            let rect = {
                let x0 = h as i32;
                let y0 = v as i32;
                let x1 = x0 as i32 + vnorm * width as i32;
                let y1 = y0 as i32 + hnorm * height as i32;
                BoundBox::new2(
                    Ord::min(x0, x1),
                    Ord::min(y0, y1),
                    Ord::max(x0, x1),
                    Ord::max(y0, y1),
                )
            };

            let distance_to_edge = Ord::min(
                Ord::min(
                    rect.min.x,
                    rect.min.y,
                ),
                Ord::min(
                    self.dims.dims.x as i32 - rect.max.x,
                    self.dims.dims.y as i32 - rect.max.y,
                )
            );

            let valid =
                distance_to_edge >= 0 &&
                self.edge_boxes()
                    .find(|b| b.intersect_rect(rect).overlaps().is_some())
                    .is_none();

            if valid {
                let perimeter_added = match concave {
                    true => height.saturating_sub(vabs) + width.saturating_sub(habs),
                    false => convex_perimeter_added
                };
                let sort = BestSort {
                    perimeter_added,
                    distance_to_edge: distance_to_edge as u32,
                };
                if sort <= best_sort {
                    best = Some(Best {
                        corner: i,
                        vnorm,
                        hnorm,
                        v,
                        h,
                        vd,
                        hd,
                        first,
                        concave,
                        rect,
                    });
                    best_sort = sort;
                }
            }
        }

        // dbg!(best);

        let best = best?;

        let v_offset = height as i32 * best.hnorm;
        let h_offset = width as i32 * best.vnorm;

        let corner = best.corner;
        // let corner = best.corner as isize;
        let add_i32_to_u32 = |u: &mut u32, i: i32| {
            *u = (*u as i32 + i) as u32;
        };
        if best.concave {
            let h_ord = Ord::cmp(&width, &(best.hd.abs() as u32));
            let v_ord = Ord::cmp(&height, &(best.vd.abs() as u32));
            dbg!((h_ord, v_ord, best.first));
            // see onenote sketch for what to do here
            match (h_ord, v_ord) {
                // four corners of sketch
                (Less, Less) |
                (Greater, Less) |
                (Less, Greater) |
                (Greater, Greater) => {
                    let v_ins = (best.v as i32 + v_offset) as u32;
                    let h_ins = (best.h as i32 + h_offset) as u32;
                    self.corners.splice(
                        corner..corner,
                        match best.first {
                            Horizontal => [v_ins, h_ins],
                            Vertical => [h_ins, v_ins],
                        }.iter().cloned()
                    );
                },

                //
                (Equal, Less) |
                (Equal, Greater) => {
                    println!("\t\t\tone");
                    match best.first {
                        Vertical => add_i32_to_u32(
                            self.get_mut(corner as isize - 1),
                            v_offset,
                        ),
                        Horizontal => add_i32_to_u32(
                            self.get_mut(corner as isize),
                            v_offset,
                        ),
                    }
                },
                (Less, Equal) |
                (Greater, Equal) => {
                    println!("\t\t\ttwo");
                    match best.first {
                        Vertical => add_i32_to_u32(
                            self.get_mut(corner as isize),
                            h_offset,
                        ),
                        Horizontal => add_i32_to_u32(
                            self.get_mut(corner as isize - 1),
                            h_offset
                        ),
                    }
                },

                // center
                (Equal, Equal) => {
                    self.remove_corner(corner);
                },
            }
        } else {
            println!("\t\t\tcorner");
            let v_ins = (best.v as i32 + v_offset) as u32;
            let h_ins = (best.h as i32 + h_offset) as u32;

            self.corners.splice(
                (corner + 1)..(corner + 1),
                match best.first {
                    Vertical => [v_ins, h_ins, best.v, best.h],
                    Horizontal => [h_ins, v_ins, best.h, best.v],
                }.iter().cloned()
            );

            // insert = true;
        }

        let mut remove_corners = vec![];
        for (i, corner) in self.corners().enumerate() {
            if corner.vd.abs() <= 4 || corner.hd.abs() <= 4 {
                remove_corners.push(i);
            }
        }

        Some(best.rect.min.to_vec().cast().unwrap())
    }

    fn get(&self, index: isize) -> u32 {
        let len = self.corners.len();
        assert!(index <= (len as isize) * 2);

        if index < 0 {
            self.corners[(len as isize + index) as usize]
        } else if len <= index as usize {
            self.corners[index as usize - len]
        } else {
            self.corners[index as usize]
        }
    }

    fn get_mut(&mut self, index: isize) -> &mut u32 {
        let len = self.corners.len();
        assert!(-(len as isize) < index);
        assert!(index <= len as isize);

        if index < 0 {
            &mut self.corners[(len as isize + index) as usize]
        } else if len <= index as usize {
            &mut self.corners[index as usize - len]
        } else {
            &mut self.corners[index as usize]
        }
    }

    fn remove_corner(&mut self, index: usize) {
        if index == 0 {
            self.corners.pop();
            self.corners.remove(0);
        } else {
            self.corners.splice((index - 1)..=index, None);
        }
    }

    // fn insert_rect(&mut self, corner_index: usize, dims: DimsBox<D2, u32>) {

    // }

    // fn points(&self) -> impl '_ + Iterator<Item=Point2<i32>> {
    //     let mut cursor = self.edge_origin;
    //     once(cursor).chain(
    //         self.corners
    //             .iter()
    //             .enumerate()
    //             .map(move |(i, n)| {
    //                 match i % 2 {
    //                     0 => cursor.x += n,
    //                     1 => cursor.y += n,
    //                     _ => unreachable!()
    //                 }
    //                 cursor
    //             })
    //         )
    // }

    // fn lines(&self) -> impl '_ + Iterator<Item=Segment<D2, i32>> {
    //     self.points()
    //         .tuple_windows::<(_, _)>()
    //         .map(|(a, b)| Segment::new(a, b))
    // }

    fn edge_boxes(&self) -> impl '_ + Iterator<Item=BoundBox<D2, i32>> {
        self.corners()
            .map(move |corner| match corner.first {
                Horizontal => {
                    let Corner {
                        h,
                        v,
                        hd,
                        hnorm,
                        ..
                    } = corner;
                    // let (h, hd, hnorm) = (corner.h, corner.hd corner.hnorm);
                    let rect_h = {
                        let x0 = h as i32;
                        let y0 = v as i32;
                        let x1 = h as i32 + hd;
                        let y1 = v as i32 - hnorm;
                        BoundBox::new2(
                            Ord::min(x0, x1),
                            Ord::min(y0, y1),
                            Ord::max(x0, x1),
                            Ord::max(y0, y1),
                        )
                    };

                    rect_h
                },
                Vertical => {
                    let Corner {
                        h,
                        v,
                        vd,
                        vnorm,
                        ..
                    } = corner;
                    let rect_v = {
                        let x0 = h as i32;
                        let y0 = v as i32;
                        let x1 = h as i32 - vnorm;
                        let y1 = v as i32 + vd;
                        BoundBox::new2(
                            Ord::min(x0, x1),
                            Ord::min(y0, y1),
                            Ord::max(x0, x1),
                            Ord::max(y0, y1),
                        )
                    };

                    rect_v
                }
            })
    }

    pub fn verify(&self) {
        // assert_eq!(0, self.corners.len() % 2);
        // let mut cursor = self.cursor_origin();
        // let dims = self.dims.dims.cast::<i32>().unwrap();
        // for (h, v) in self.corners.iter().cloned().tuples() {
        //     cursor += Vector2::new(h, 0);
        //     cursor += Vector2::new(0, v);
        //     assert!(h != 0);
        //     assert!(v != 0);
        //     assert!(cursor.x >= 0, "{}", cursor.x);
        //     assert!(cursor.y >= 0, "{}", cursor.y);
        //     assert!(cursor.x <= dims.x, "{}", cursor.x);
        //     assert!(cursor.y <= dims.y, "{}", cursor.y);
        // }
        // assert_eq!(self.cursor_origin(), cursor);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_corners() {
        let atlas = PerimeterAtlas::new(DimsBox::new2(8, 12), 0);
        let expected = vec![
            Corner {
                first: Horizontal,
                v: 0,
                h: 0,
                vd: -12,
                hd: 8,
                vnorm: 1,
                hnorm: 1,
                concave: true
            },
            Corner {
                first: Vertical,
                v: 0,
                h: 8,
                vd: 12,
                hd: 8,
                vnorm: -1,
                hnorm: 1,
                concave: true,
            },
            Corner {
                first: Horizontal,
                v: 12,
                h: 8,
                vd: 12,
                hd: -8,
                vnorm: -1,
                hnorm: -1,
                concave: true,
            },
            Corner {
                first: Vertical,
                v: 12,
                h: 0,
                vd: -12,
                hd: -8,
                vnorm: 1,
                hnorm: -1,
                concave: true,
            }
        ];
        let actual = atlas.corners().collect::<Vec<_>>();
        assert_eq!(
            expected,
            actual,
            "{:#?} != {:#?}", expected, actual
        );
    }

    #[test]
    fn test_edge_boxes() {
        let atlas = PerimeterAtlas::new(DimsBox::new2(512, 512), 0);
        let expected = vec![
            BoundBox::new2(0, -1, 512, 0),
            BoundBox::new2(512, 0, 513, 512),
            BoundBox::new2(0, 512, 512, 513),
            BoundBox::new2(-1, 0, 0, 512),
        ];
        let actual = atlas.edge_boxes().collect::<Vec<_>>();
        assert_eq!(
            expected,
            actual,
            "{:#?} != {:#?}", expected, actual
        );
    }
}
