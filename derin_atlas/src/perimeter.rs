// use std::{cmp, mem};
// use std::ops::Range;

use crate::cgmath::{EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, line::Segment, rect::{DimsBox, BoundBox, OffsetBox, GeoBox}};
use crate::raw::RawAtlas;
use itertools::Itertools;
use std::{
    cmp::{Ordering::{Less, Equal, Greater}, Ord},
    iter::once,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum First {
    Vertical,
    Horizontal,
}
use First::{Vertical, Horizontal};

pub struct PerimeterAtlas<P: 'static + Copy> {
    raw: RawAtlas<P>,
    dims: DimsBox<D2, u32>,

    /// The edges of the internal perimeter.
    ///
    /// Edges alternate in a Horizontal/Vertical/Horizontal/Vertical pattern. Positive values represent
    /// an edge that's moving away from the origin, and negative values represent an edge that's moving
    /// towards the origin.
    edges: Vec<i32>,
    edge_origin: Point2<i32>,
}

impl<P: Copy> PerimeterAtlas<P> {
    pub fn new(dims: DimsBox<D2, u32>, background_color: P) -> PerimeterAtlas<P> {
        let height = dims.height() as i32;
        let width = dims.width() as i32;
        PerimeterAtlas {
            raw: RawAtlas::new(dims, background_color),
            dims,
            // edges: vec![height, width, -height, -width]
            edges: vec![width, height, -width, -height],
            edge_origin: Point2::new(0, 0),
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
        let mut edges_image = RawAtlas::new(dims, back);

        let first_corner = [self.edges[self.edges.len() - 1], self.edges[0]];
        let mut cursor = self.edge_origin;

        for (i, [a, b]) in once(first_corner).chain(self.edges.windows(2).map(|w| [w[0], w[1]])).enumerate() {
            let first = match i % 2 {
                0 => Vertical,
                1 => Horizontal,
                _ => unreachable!()
            };
            let (v, h) = match first {
                Vertical => (a, b),
                Horizontal => (b, a),
            };

            let mut next_cursor = cursor;
            match first {
                Vertical => next_cursor.x += h,
                Horizontal => next_cursor.y += v,
            }

            let rect = {
                let x0 = cursor.x;
                let y0 = cursor.y;
                let x1 = next_cursor.x;
                let y1 = next_cursor.y;
                OffsetBox::from(
                    BoundBox::new2(
                        Ord::min(x0, x1),
                        Ord::min(y0, y1),
                        Ord::max(x0, x1) + 1,
                        Ord::max(y0, y1) + 1,
                    )
                )
            };
            let rect = rect.intersect_rect(dims.cast::<i32>().unwrap().into()).either();
            let rect = match rect {
                Some(rect) => rect,
                None => {
                    cursor = next_cursor;
                    continue
                }
            };

            edges_image.blit_pixel_iter(
                dims,
                std::iter::repeat(edge(i)).take(rect.width() as usize * rect.height() as usize),
                rect.dims().cast().unwrap(),
                rect.origin.to_vec().cast().expect(&format!("{:?}", rect)),
            );

            cursor = next_cursor;
        }

        (dims, edges_image.pixels_box())
    }

    fn best_corner(&mut self, rect: DimsBox<D2, u32>) -> Option<Vector2<u32>> {
        assert_ne!(0, rect.width());
        assert_ne!(0, rect.height());
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct Best {
            corner: usize,
            vnorm: i32,
            hnorm: i32,
            v: i32,
            h: i32,
            first: First,
            cursor: Point2<i32>,
            concave: bool,
            rect: BoundBox<D2, i32>,
        }
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        struct BestSort {
            perimeter_added: u32,
            distance_to_edge: u32,
        }

        let width = rect.width();
        let height = rect.height();
        let convex_perimeter_added = 2 * width * height;

        let mut best = None;
        let mut best_sort = BestSort {
            perimeter_added: u32::max_value(),
            distance_to_edge: u32::max_value(),
        };
        let first_corner = [self.edges[self.edges.len() - 1], self.edges[0]];
        let mut cursor = self.edge_origin;

        for (i, [a, b]) in once(first_corner).chain(self.edges.windows(2).map(|w| [w[0], w[1]])).enumerate() {
            let first = match i % 2 {
                0 => Vertical,
                1 => Horizontal,
                _ => unreachable!()
            };
            let (v, h, concave) = match first {
                Vertical => (a, b, a.signum() != b.signum()),
                Horizontal => (b, a, a.signum() == b.signum()),
            };

            let vnorm = -v.signum();
            let hnorm = h.signum();

            let vabs = v.abs() as u32;
            let habs = h.abs() as u32;

            let rect = {
                let x0 = cursor.x;
                let y0 = cursor.y;
                let x1 = x0 + vnorm * width as i32;
                let y1 = y0 + hnorm * height as i32;
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
                        first,
                        cursor,
                        concave,
                        rect,
                    });
                    best_sort = sort;
                }
            }

            match first {
                Vertical => cursor.x += h,
                Horizontal => cursor.y += v,
            }
        }

        let best = best?;

        let width = width as i32;
        let height = height as i32;

        let insert: bool;
        let h_ins = width * best.h.signum();
        let v_ins = height * best.v.signum();
        let insert_array = match best.first {
            Vertical => [h_ins, v_ins],
            Horizontal => [v_ins, h_ins],
        };
        let insert_array_concave = match best.first {
            Vertical => [-h_ins, v_ins, h_ins, -v_ins],
            Horizontal => [-v_ins, h_ins, v_ins, -h_ins],
        };

        let h_ord = Ord::cmp(&width, &best.h.abs());
        let v_ord = Ord::cmp(&height, &best.v.abs());

        let corner = best.corner as isize;
        if best.concave {
            // see onenote sketch for what to do here
            match (h_ord, v_ord, best.first) {
                // four corners of sketch
                (Less, Less, _) |
                (Greater, Less, _) |
                (Less, Greater, _) |
                (Greater, Greater, _) => {
                    insert = true;
                    *self.get_mut(corner - 1) -= insert_array[1];
                    *self.get_mut(corner) -= insert_array[0];
                },

                // PERHAPS HORIZONTAL AND VERTICAL HANDLING ARE DIFFERENT FOR THESE? SKETCH IT
                // center column
                (Equal, Less, Vertical) |
                (Equal, Greater, Vertical) => {
                    insert = false;
                    *self.get_mut(corner - 1) -= insert_array[1];
                    *self.get_mut(corner + 1) += insert_array[1];
                },
                (Equal, Less, Horizontal) |
                (Equal, Greater, Horizontal) => {
                    insert = false;
                    *self.get_mut(corner - 2) += insert_array[0];
                    *self.get_mut(corner) -= insert_array[0];
                },
                // center row
                (Less, Equal, Vertical) |
                (Greater, Equal, Vertical) => {
                    insert = false;
                    *self.get_mut(corner - 2) += insert_array[0];
                    *self.get_mut(corner) -= insert_array[0];
                },
                (Less, Equal, Horizontal) |
                (Greater, Equal, Horizontal) => {
                    insert = false;
                    *self.get_mut(corner - 1) -= insert_array[1];
                    *self.get_mut(corner + 1) += insert_array[1];
                },

                // center
                (Equal, Equal, _) => {
                    insert = false;
                    *self.get_mut(corner - 2) += insert_array[0];
                    *self.get_mut(corner + 1) += insert_array[1];
                    self.remove_corner(corner as usize);
                },
            }
        } else {
            insert = true;
        }

        println!("{:?},{},{},{},{:?},{:?},{},{}", best.first, best.corner, self.edges.len(), best.concave, h_ord, v_ord, h_ins, v_ins);

        if insert {
            let corner = corner as usize;
            if corner == 0 {
                self.edge_origin += match best.first {
                    Vertical => Vector2::new(0, -v_ins),
                    Horizontal => Vector2::new(h_ins, 0),
                };
            }
            self.edges.splice(
                corner..corner,
                match best.concave {
                    true => insert_array.iter().cloned(),
                    false => insert_array_concave.iter().cloned(),
                }
            );
        }


        // self.verify();

        Some(best.rect.min.to_vec().cast().unwrap())
    }

    fn get(&self, index: isize) -> i32 {
        let len = self.edges.len();
        assert!(index <= (len as isize) * 2);

        if index < 0 {
            self.edges[(len as isize + index) as usize]
        } else if len <= index as usize {
            self.edges[index as usize - len]
        } else {
            self.edges[index as usize]
        }
    }

    fn get_mut(&mut self, index: isize) -> &mut i32 {
        let len = self.edges.len();
        assert!(-(len as isize) < index);
        assert!(index <= len as isize);

        if index < 0 {
            &mut self.edges[(len as isize + index) as usize]
        } else if len <= index as usize {
            &mut self.edges[index as usize - len]
        } else {
            &mut self.edges[index as usize]
        }
    }

    fn remove_corner(&mut self, index: usize) {
        if index == 0 {
            self.edges.pop();
            self.edges.remove(0);
        } else {
            self.edges.splice((index - 1)..=index, None);
        }
    }

    // fn insert_rect(&mut self, corner_index: usize, dims: DimsBox<D2, u32>) {

    // }

    fn points(&self) -> impl '_ + Iterator<Item=Point2<i32>> {
        let mut cursor = self.edge_origin;
        once(cursor).chain(
            self.edges
                .iter()
                .enumerate()
                .map(move |(i, n)| {
                    match i % 2 {
                        0 => cursor.x += n,
                        1 => cursor.y += n,
                        _ => unreachable!()
                    }
                    cursor
                })
            )
    }

    fn lines(&self) -> impl '_ + Iterator<Item=Segment<D2, i32>> {
        self.points()
            .tuple_windows::<(_, _)>()
            .map(|(a, b)| Segment::new(a, b))
    }

    fn edge_boxes(&self) -> impl '_ + Iterator<Item=BoundBox<D2, i32>> {
        let mut cursor = self.edge_origin;

        self.edges
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, n)| (
                match i % 2 {
                    0 => Horizontal,
                    1 => Vertical,
                    _ => unreachable!()
                },
                n
            ))
            .map(move |(f, n)| match f {
                Horizontal => {
                    let h = n;
                    let hnorm = h.signum();
                    let rect_h = {
                        let x0 = cursor.x;
                        let y0 = cursor.y;
                        let x1 = cursor.x + h;
                        let y1 = cursor.y - hnorm;
                        BoundBox::new2(
                            Ord::min(x0, x1),
                            Ord::min(y0, y1),
                            Ord::max(x0, x1),
                            Ord::max(y0, y1),
                        )
                    };

                    cursor.x += h;
                    rect_h
                },
                Vertical => {
                    let v = n;
                    let vnorm = -v.signum();
                    let rect_v = {
                        let x0 = cursor.x;
                        let y0 = cursor.y;
                        let x1 = cursor.x - vnorm;
                        let y1 = cursor.y + v;
                        BoundBox::new2(
                            Ord::min(x0, x1),
                            Ord::min(y0, y1),
                            Ord::max(x0, x1),
                            Ord::max(y0, y1),
                        )
                    };

                    cursor.y += v;
                    rect_v
                }
            })
    }

    pub fn verify(&self) {
        assert_eq!(0, self.edges.len() % 2);
        let mut cursor = self.edge_origin;
        let dims = self.dims.dims.cast::<i32>().unwrap();
        for (h, v) in self.edges.iter().cloned().tuples() {
            cursor += Vector2::new(h, 0);
            cursor += Vector2::new(0, v);
            assert!(h != 0);
            assert!(v != 0);
            assert!(cursor.x >= 0, "{}", cursor.x);
            assert!(cursor.y >= 0, "{}", cursor.y);
            assert!(cursor.x <= dims.x, "{}", cursor.x);
            assert!(cursor.y <= dims.y, "{}", cursor.y);
        }
        assert_eq!(self.edge_origin, cursor);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_edge_boxes() {
        let atlas = PerimeterAtlas::new(DimsBox::new2(512, 512), 0);
        assert_eq!(
            vec![
                BoundBox::new2(0, -1, 512, 0),
                BoundBox::new2(512, 0, 513, 512),
                BoundBox::new2(0, 512, 512, 513),
                BoundBox::new2(-1, 0, 0, 512),
            ],
            atlas.edge_boxes().collect::<Vec<_>>()
        );
    }
}
