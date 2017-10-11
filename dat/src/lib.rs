#![feature(splice)]

extern crate cgmath;
extern crate cgmath_geometry;

use std::cmp::{self, Ordering};
use std::ops::Range;

use cgmath::Vector2;
use cgmath_geometry::{DimsRect, OffsetRect, Rectangle};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeightRange {
    bounds_min: u32,
    // Exclusive max value
    bounds_max: u32,
    height: u32
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkylineAtlas {
    pixel_size: u8,
    dims: DimsRect<u32>,
    pixels: Box<[u8]>,
    heights: Vec<HeightRange>
}

impl SkylineAtlas {
    #[inline]
    pub fn new(pixel_size: u8, dims: DimsRect<u32>) -> SkylineAtlas {
        let base_range = HeightRange {
            bounds_min: 0,
            bounds_max: dims.width(),
            height: 0
        };

        let pixel_vec_len = (dims.width() * dims.height()) as usize * pixel_size as usize;

        SkylineAtlas {
            pixel_size, dims,
            pixels: vec![0; pixel_vec_len].into_boxed_slice(),
            heights: vec![base_range]
        }
    }

    #[inline]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    #[inline]
    pub fn pixel_size(&self) -> u8 {
        self.pixel_size
    }

    #[inline]
    pub fn dims(&self) -> DimsRect<u32> {
        self.dims
    }

    pub fn add_image(&mut self, image_dims: DimsRect<u32>, image_data: &[u8]) -> Option<OffsetRect<u32>> {
        #[derive(Debug, Clone)]
        struct InsertOver {
            range: Range<usize>,
            width: u32,
            height: u32
        }

        let mut best_range = InsertOver {
            range: 0..self.heights.len(),
            width: self.dims.width(),
            height: self.dims.height()
        };
        for (i, height) in self.heights.iter().enumerate() {
            let mut active_range = InsertOver {
                range: i..i+1,
                width: height.width(),
                height: height.height
            };

            for next_height in self.heights[i + 1..].iter() {
                if active_range.width >= image_dims.width() {
                    break;
                }

                active_range.range.end += 1;
                active_range.width += next_height.width();
                active_range.height = cmp::max(active_range.height, next_height.height);
            }

            if active_range.height < best_range.height && active_range.width >= image_dims.width() {
                best_range = active_range;
            }
        }

        if self.dims.height() - best_range.height < image_dims.height() || best_range.width < image_dims.width() {
            return None;
        }

        let insert_offset = Vector2::new(self.heights[best_range.range.start].bounds_min, best_range.height);

        let bounds_min = self.heights[best_range.range.start].bounds_min;
        let insert_range = HeightRange {
            bounds_min,
            bounds_max: bounds_min + image_dims.width(),
            height: best_range.height + image_dims.height()
        };
        if best_range.width == image_dims.width() {
            self.heights.splice(best_range.range.clone(), Some(insert_range));
        } else {
            self.heights[best_range.range.end - 1].bounds_min = bounds_min + image_dims.width();
            self.heights.splice(best_range.range.start..best_range.range.end - 1, Some(insert_range));
        }

        let insert_rect = OffsetRect::from(image_dims) + insert_offset;
        self.blit(insert_rect, image_data);
        Some(insert_rect)
    }

    fn blit(&mut self, image_rect: OffsetRect<u32>, image_data: &[u8]) {
        let pixel_size = self.pixel_size as usize;
        let image_row_offset = image_rect.min().x as usize * pixel_size;
        let image_row_size = image_rect.width() as usize * pixel_size;

        for atlas_row_num in image_rect.min().y..image_rect.max().y {
            let atlas_row_num = atlas_row_num as usize;
            let row_start_index = atlas_row_num * self.dims.width() as usize *  pixel_size;
            let row_slice = &mut self.pixels[row_start_index..row_start_index + self.dims.width() as usize * pixel_size];

            let source_row_num = atlas_row_num - image_rect.min().y as usize;

            let image_slice = &image_data[image_row_size * source_row_num..image_row_size * source_row_num + image_row_size];
            row_slice[image_row_offset..image_row_offset + image_row_size].copy_from_slice(image_slice);
        }
    }
}

impl HeightRange {
    #[inline]
    fn width(&self) -> u32 {
        self.bounds_max - self.bounds_min
    }
}
