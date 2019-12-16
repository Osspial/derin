// use std::{cmp, mem};
// use std::ops::Range;

use crate::cgmath::{EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, rect::{DimsBox, BoundBox, OffsetBox, GeoBox}};
use crate::raw::RawAtlas;
use itertools::Itertools;
use std::{
    cmp::{Ordering::{Less, Equal, Greater}, Ord},
    iter::once,
};

pub struct PerimeterAtlas<P: 'static + Copy> {
    raw: RawAtlas<P>,
    dims: DimsBox<D2, u32>,

    /// The edges of the internal perimeter.
    ///
    /// Edges alternate in a Horizontal/Vertical/Horizontal/Vertical pattern. Positive values represent
    /// an edge that's moving away from the origin, and negative values represent an edge that's moving
    /// towards the origin.
    edges: Vec<i32>,
}

impl<P: Copy> PerimeterAtlas<P> {
    pub fn new(dims: DimsBox<D2, u32>, background_color: P) -> PerimeterAtlas<P> {
        let height = dims.height() as i32;
        let width = dims.width() as i32;
        println!("{:?}", vec![height, width, -height, -width]);
        PerimeterAtlas {
            raw: RawAtlas::new(dims, background_color),
            dims,
            // edges: vec![height, width, -height, -width]
            edges: vec![width, height, -width, -height],
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

    pub fn edge_image(&self, back: P, edge: P) -> (DimsBox<D2, u32>, Box<[P]>) {
        let dims = DimsBox::new(self.dims.dims + Vector2::new(1, 1));
        let mut edges_image = RawAtlas::new(dims, back);

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum First {
            Vertical,
            Horizontal,
        }
        use First::{Vertical, Horizontal};

        let first_corner = [self.edges[self.edges.len() - 1], self.edges[0]];
        let mut cursor = Point2::new(0, 0);

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

            edges_image.blit_pixel_iter(
                dims,
                std::iter::repeat(edge).take(dbg!(rect.width() as usize * rect.height() as usize)),
                rect.dims().cast().unwrap(),
                rect.origin.to_vec().cast().unwrap(),
            );

            cursor = next_cursor;
        }

        (dims, edges_image.pixels_box())
    }

    fn best_corner(&mut self, rect: DimsBox<D2, u32>) -> Option<Vector2<u32>> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum First {
            Vertical,
            Horizontal,
        }
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
        }
        use First::{Vertical, Horizontal};
        println!("\n\n\n");
        dbg!(rect);

        let width = rect.width();
        let height = rect.height();
        let convex_perimeter_added = 2 * width * height;

        let mut best = Best {
            corner: 0,
            vnorm: 0,
            hnorm: 0,
            v: 0,
            h: 0,
            first: Vertical,
            cursor: Point2::new(0, 0),
            concave: false,
        };
        let mut best_perimeter_added = u32::max_value();
        let first_corner = [self.edges[self.edges.len() - 1], self.edges[0]];
        let mut cursor = Point2::new(0, 0);

        for (i, [a, b]) in once(first_corner).chain(self.edges.windows(2).map(|w| [w[0], w[1]])).enumerate() {
            // println!("{} {} {:?}", a, b, cursor);
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

            let perimeter_added = match concave {
                true => height.saturating_sub(vabs) + width.saturating_sub(habs),
                false => convex_perimeter_added
            };
            // println!("{:?}", concave);
            // println!("{} - {} = {}", height, vabs, height.saturating_sub(vabs));
            // println!("{} - {} = {}", width, habs, width.saturating_sub(habs));
            // println!("{} {}", perimeter_added, best_perimeter_added);
            if perimeter_added <= best_perimeter_added {
                best = Best {
                    corner: i,
                    vnorm,
                    hnorm,
                    v,
                    h,
                    first,
                    cursor,
                    concave,
                };
                best_perimeter_added = perimeter_added;
            }

            match first {
                Vertical => cursor.x += h,
                Horizontal => cursor.y += v,
            }
        }

        dbg!(best);

        let width = width as i32;
        let height = height as i32;

        let rect = {
            let x0 = best.cursor.x;
            let y0 = best.cursor.y;
            let x1 = x0 + best.vnorm * width;
            let y1 = y0 + best.hnorm * height;
            dbg!((x0, y0), (x1, y1));
            BoundBox::new2(
                Ord::min(x0, x1),
                Ord::min(y0, y1),
                Ord::max(x0, x1),
                Ord::max(y0, y1),
            )
        };

        let insert: bool;
        let h_ins = width * best.h.signum();
        let v_ins = height * best.v.signum();
        let insert_array = match best.first {
            Vertical => [h_ins, v_ins],
            Horizontal => [v_ins, h_ins],
        };

        dbg!(insert_array);

        let h_ord = Ord::cmp(&width, &best.h.abs());
        let v_ord = Ord::cmp(&height, &best.v.abs());

        let corner = best.corner as isize;
        // see onenote sketch for what to do here
        match dbg!((h_ord, v_ord)) {
            // four corners of sketch
            (Less, Less) |
            (Greater, Less) |
            (Less, Greater) |
            (Greater, Greater) => {
                insert = true;
                *self.get_mut(corner - 1) -= insert_array[1];
                *self.get_mut(corner) -= insert_array[0];
            },

            // center column
            (Equal, Less) |
            (Equal, Greater) => {
                insert = false;
                *self.get_mut(corner - 2) += insert_array[0];
                *self.get_mut(corner) -= insert_array[0];
            },

            // center row
            (Less, Equal) |
            (Greater, Equal) => {
                insert = false;
                *self.get_mut(corner - 1) -= insert_array[1];
                *self.get_mut(corner + 1) += insert_array[1];
            },

            // center
            (Equal, Equal) => {
                insert = false;
                *self.get_mut(corner - 2) += insert_array[0];
                *self.get_mut(corner + 1) += insert_array[1];
                self.remove_corner(corner as usize);
            },
        }

        if insert {
            let corner = corner as usize;
            self.edges.splice(corner..corner, insert_array.iter().cloned());
        }

        println!("{:?}", self.edges);
        self.verify();

        Some(rect.min.to_vec().cast().unwrap())
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

    fn verify(&self) {
        assert_eq!(0, self.edges.len() % 2);
        let mut cursor = Point2::new(0, 0);
        let dims = self.dims.dims.cast::<i32>().unwrap();
        println!("\nverifying:");
        for (h, v) in self.edges.iter().cloned().tuples() {
            cursor += Vector2::new(h, 0);
            println!("+{:?}\t= {:?}", Vector2::new(h, 0), cursor);
            cursor += Vector2::new(0, v);
            println!("+{:?}\t= {:?}", Vector2::new(0, v), cursor);
            assert!(cursor.x >= 0, "{}", cursor.x);
            assert!(cursor.y >= 0, "{}", cursor.y);
            assert!(cursor.x <= dims.x, "{}", cursor.x);
            assert!(cursor.y <= dims.y, "{}", cursor.y);
        }
        assert_eq!(Point2::new(0, 0), cursor);
    }
}
