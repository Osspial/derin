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

#![feature(splice)]

use cgmath_geometry::cgmath;
extern crate cgmath_geometry;

use std::{cmp, mem};
use std::ops::Range;

use crate::cgmath::{EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, rect::{DimsBox, OffsetBox, GeoBox}};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeightRange {
    bounds_min: u32,
    // Exclusive max value
    bounds_max: u32,
    height: u32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkylineAtlas<P: Copy> {
    background_color: P,
    dims: DimsBox<D2, u32>,
    pixels: Vec<P>,
    heights: Vec<HeightRange>,
    max_used_height: u32
}

#[derive(Debug, Clone)]
struct InsertOver {
    range: Range<usize>,
    width: u32,
    height: u32,
    space_lost: u32
}

impl<P: Copy> SkylineAtlas<P> {
    #[inline]
    pub fn new(background_color: P, dims: DimsBox<D2, u32>) -> SkylineAtlas<P> {
        let base_range = HeightRange {
            bounds_min: 0,
            bounds_max: dims.width(),
            height: 0
        };

        SkylineAtlas {
            background_color, dims,
            pixels: vec![background_color; (dims.width() * dims.height()) as usize],
            heights: vec![base_range],
            max_used_height: 0
        }
    }

    #[inline]
    pub fn pixels(&self) -> &[P] {
        &self.pixels
    }

    #[inline]
    pub fn dims(&self) -> DimsBox<D2, u32> {
        self.dims
    }

    fn calc_insert_over(&self, image_dims: DimsBox<D2, u32>) -> Option<InsertOver> {
        let mut best_range = InsertOver {
            range: 0..self.heights.len(),
            width: self.dims.width(),
            height: self.dims.height(),
            space_lost: self.dims.width() * self.dims.height()
        };
        for (i, height) in self.heights.iter().enumerate() {
            let mut active_range = InsertOver {
                range: i..i+1,
                width: height.width(),
                height: height.height,
                space_lost: 0
            };

            for next_height in self.heights[i + 1..].iter() {
                if active_range.width >= image_dims.width() {
                    break;
                }

                active_range.range.end += 1;
                if next_height.height > active_range.height {
                    active_range.space_lost += active_range.width * next_height.height - active_range.height;
                    active_range.height = next_height.height;
                } else if next_height.height < active_range.height {
                    let lost_width;
                    if active_range.width + next_height.width() >= image_dims.width() {
                        lost_width = next_height.width() - (image_dims.width() - active_range.width);
                    } else {
                        lost_width = next_height.width();
                    }

                    if lost_width > 0 {
                        active_range.space_lost += lost_width * active_range.height - next_height.height;
                    }
                }

                active_range.width += next_height.width();
            }

            let active_is_better =
                (
                    active_range.space_lost < best_range.space_lost ||
                    active_range.height < best_range.height
                ) &&
                active_range.width >= image_dims.width() &&
                self.dims.height() - active_range.height >= image_dims.height();
            if active_is_better {
                best_range = active_range;
            }
        }

        if self.dims.height() - best_range.height < image_dims.height() || best_range.width < image_dims.width() {
            return None;
        }

        Some(best_range)
    }

    pub fn set_dims(&mut self, background_color: P, dims: DimsBox<D2, u32>) {
        let free_width = {
            let last_height = self.heights.last().unwrap();
            match last_height.height {
                0 => last_height.width(),
                _ => 0
            }
        };
        let free_height = self.dims.height() - self.max_used_height;
        assert!(self.dims.height() - free_height <= dims.height());
        assert!(self.dims.width() - free_width <= dims.width());

        let mut pixel_swap = vec![background_color; (dims.width() * dims.height()) as usize];
        mem::swap(&mut pixel_swap, &mut self.pixels);

        let old_dims = self.dims;
        self.dims = dims;
        self.blit(old_dims, old_dims.into(), Vector2::new(0, 0), &pixel_swap);

        if dims.width() < old_dims.width() {
            self.heights.last_mut().unwrap().bounds_max -= old_dims.width() - dims.width();
        } else {
            self.heights.last_mut().unwrap().bounds_max += dims.width() - old_dims.width();
        }
        if dims.height() < old_dims.height() {
            self.max_used_height -= old_dims.height() - dims.height();
        } else {
            self.max_used_height += dims.height() - old_dims.height();
        }
    }

    pub fn max_used_height(&self) -> u32 {
        self.max_used_height
    }

    fn insert_over(&mut self, insert_over: InsertOver, image_dims: DimsBox<D2, u32>) -> OffsetBox<D2, u32>
    {
        let insert_offset = Vector2::new(self.heights[insert_over.range.start].bounds_min, insert_over.height);

        let bounds_min = self.heights[insert_over.range.start].bounds_min;
        let insert_range = HeightRange {
            bounds_min,
            bounds_max: bounds_min + image_dims.width(),
            height: insert_over.height + image_dims.height()
        };
        if insert_over.width == image_dims.width() {
            self.heights.splice(insert_over.range.clone(), Some(insert_range));
        } else {
            self.heights[insert_over.range.end - 1].bounds_min = bounds_min + image_dims.width();
            self.heights.splice(insert_over.range.start..insert_over.range.end - 1, Some(insert_range));
        }
        self.max_used_height = cmp::max(self.max_used_height, insert_range.height);

        OffsetBox::from(image_dims) + insert_offset
    }

    pub fn add_image(&mut self, image_dims: DimsBox<D2, u32>, image_view: OffsetBox<D2, u32>, image_data: &[P]) -> Option<OffsetBox<D2, u32>> {
        self.add_image_rows(image_view.dims(), rows_from_image(image_dims, image_view, image_data)).ok()
    }

    pub fn add_image_rows<'a, I>(&mut self, image_dims: DimsBox<D2, u32>, image_data: I) -> Result<OffsetBox<D2, u32>, I>
        where I: IntoIterator<Item=&'a [P]>,
              P: 'a
    {
        match self.calc_insert_over(image_dims) {
            Some(range) => {
                let insert_rect = self.insert_over(range, image_dims);
                self.blit_rows(image_dims, insert_rect.min().to_vec(), image_data);
                Ok(insert_rect)
            },
            None => Err(image_data)
        }
    }

    pub fn add_image_pixels<'a, I, J>(&mut self, image_dims: DimsBox<D2, u32>, image_data: I) -> Result<OffsetBox<D2, u32>, I>
        where I: IntoIterator<Item=J>,
              J: IntoIterator<Item=P>
    {
        match self.calc_insert_over(image_dims) {
            Some(range) =>{
                let insert_rect = self.insert_over(range, image_dims);
                self.blit_pixels(image_dims, insert_rect.min().to_vec(), image_data);
                Ok(insert_rect)
            },
            None => Err(image_data)
        }
    }

    pub fn clear(&mut self, background_color: Option<P>) {
        self.heights.clear();
        self.heights.push(HeightRange {
            bounds_min: 0,
            bounds_max: self.dims.width(),
            height: 0
        });

        if let Some(bgc) = background_color {
            for pixel in &mut self.pixels {
                *pixel = bgc;
            }
        }
    }

    pub fn compact<'a, I>(&mut self, rects: I)
        where I: IntoIterator<Item=&'a mut OffsetBox<D2, u32>>
    {
        let mut old_pixels = vec![self.background_color; self.pixels.len()];
        mem::swap(&mut old_pixels, &mut self.pixels);
        let old_heights = self.heights.clone();

        let mut rects_sorted = {
            let mut rects: Vec<(OffsetBox<D2, u32>, &'a mut OffsetBox<D2, u32>)> = rects.into_iter().map(|r| (*r, r)).collect();
            rects.sort_unstable_by(|&(_, ref a), &(_, ref b)| (b.height(), b.width()).cmp(&(a.height(), a.width())));
            rects
        };
        let mut removed_rects = Vec::with_capacity(rects_sorted.len());

        self.max_used_height = 0;
        self.heights.clear();
        self.heights.push(HeightRange {
            bounds_min: 0,
            bounds_max: self.dims.width(),
            height: 0
        });

        let mut reset_atlas = false;
        let dims = self.dims;
        'main_rect: while rects_sorted.len() > 0 {
            let mut best_insert_index = usize::max_value();
            let mut best_insert_over = InsertOver {
                range: 0..self.heights.len(),
                width: self.dims.width(),
                height: self.dims.height(),
                space_lost: self.dims.width() * self.dims.height()
            };

            for (index, &mut (_, ref mut rect)) in rects_sorted.iter_mut().enumerate() {
                match self.calc_insert_over(rect.dims()) {
                    Some(insert_over) => {
                        if insert_over.space_lost < best_insert_over.space_lost || best_insert_index == usize::max_value() {
                            best_insert_index = index;
                            best_insert_over = insert_over;

                            if best_insert_over.space_lost == 0 {
                                break;
                            }
                        }
                    },
                    None => {
                        reset_atlas = true;
                        break 'main_rect;
                    }
                }
            }

            let remove_rect = rects_sorted.remove(best_insert_index);
            *remove_rect.1 = self.insert_over(
                best_insert_over,
                remove_rect.0.dims()
            );
            self.blit(dims, remove_rect.0, remove_rect.1.min().to_vec(), &old_pixels);
            removed_rects.push(remove_rect);
        }

        if reset_atlas {
            self.pixels = old_pixels;
            self.heights = old_heights;
            for (old_rect, rect_ref) in rects_sorted.drain(..).chain(removed_rects.drain(..)) {
                *rect_ref = old_rect;
            }
        }
    }

    pub fn blit(&mut self, image_dims: DimsBox<D2, u32>, image_view: OffsetBox<D2, u32>, write_offset: Vector2<u32>, image_data: &[P]) {
        blit(
            rows_from_image(image_dims, image_view, image_data), image_view.dims(),
            &mut self.pixels, self.dims, write_offset
        );
    }

    pub fn blit_rows<'a, I>(&mut self, image_dims: DimsBox<D2, u32>, write_offset: Vector2<u32>, image_data: I)
        where I: IntoIterator<Item=&'a [P]>,
              P: 'a
    {
        blit(image_data, image_dims, &mut self.pixels, self.dims, write_offset);
    }

    pub fn blit_pixels<'a, I, J>(&mut self, image_dims: DimsBox<D2, u32>, write_offset: Vector2<u32>, image_data: I)
        where I: IntoIterator<Item=J>,
              J: IntoIterator<Item=P>
    {
        blit_pixels(image_data, image_dims, &mut self.pixels, self.dims, write_offset);
    }
}

impl HeightRange {
    #[inline]
    fn width(&self) -> u32 {
        self.bounds_max - self.bounds_min
    }
}

fn rows_from_image<'a, P: 'a>(image_dims: DimsBox<D2, u32>, image_view: OffsetBox<D2, u32>, image_data: &'a [P]) -> impl Iterator<Item=&'a [P]> {
    (image_view.min().y as usize..image_view.max().y as usize)
        .map(move |r| &image_data[
            image_dims.width() as usize * r + image_view.min().x as usize..
            image_dims.width() as usize * r + image_view.min().x as usize + image_view.width() as usize
        ])
}

fn blit<'a, P: 'a + Copy, I: IntoIterator<Item=&'a [P]>>(
    src: I, src_dims: DimsBox<D2, u32>,
    dst: &mut [P], dst_dims: DimsBox<D2, u32>, dst_offset: Vector2<u32>
) {
    let (mut width, mut height) = (src_dims.width(), 0);
    for (row_num, src_row) in src.into_iter().enumerate() {
        let dst_row_num = row_num + dst_offset.y as usize;
        let dst_slice_offset = dst_row_num * dst_dims.width() as usize;
        let dst_row = &mut dst[dst_slice_offset..dst_slice_offset + dst_dims.width() as usize];

        let dst_copy_to_slice = &mut dst_row[dst_offset.x as usize..dst_offset.x as usize + src_row.len()];
        dst_copy_to_slice.copy_from_slice(src_row);

        height += 1;
        width &= src_row.len() as u32;
    }

    assert_eq!(src_dims, DimsBox::new2(width, height));
}

fn blit_pixels<'a, P, I, J>(
    src: I, src_dims: DimsBox<D2, u32>,
    dst: &mut [P], dst_dims: DimsBox<D2, u32>, dst_offset: Vector2<u32>
)
    where I: IntoIterator<Item=J>,
          J: IntoIterator<Item=P>
{
    let (mut width, mut height) = (src_dims.width(), 0);
    for (row_num, src_row) in src.into_iter().enumerate() {
        let dst_row_num = row_num + dst_offset.y as usize;
        let dst_slice_offset = dst_row_num * dst_dims.width() as usize;
        let dst_row = &mut dst[dst_slice_offset..dst_slice_offset + dst_dims.width() as usize];

        let dst_copy_to_slice = &mut dst_row[dst_offset.x as usize..];
        let mut src_row_len = 0;

        for (p, v) in dst_copy_to_slice.iter_mut().zip(src_row.into_iter()) {
            *p = v;
            src_row_len += 1;
        }

        height += 1;
        width &= src_row_len;
    }

    assert_eq!(src_dims, DimsBox::new2(width, height));
}
