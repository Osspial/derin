use std::{cmp, mem};
use std::ops::Range;

use crate::cgmath::{EuclideanSpace, Vector2};
use cgmath_geometry::{D2, rect::{DimsBox, OffsetBox, GeoBox}};
use crate::raw::RawAtlas;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeightRange {
    bounds_min: u32,
    // Exclusive max value
    bounds_max: u32,
    height: u32
}

/// A basic texture atlas using the skyline insertion algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkylineAtlas<P: 'static +  Copy> {
    raw: RawAtlas<P>,
    dims: DimsBox<D2, u32>,
    heights: Vec<HeightRange>,
    max_used_height: u32
}

pub struct AddImageDeferred<'a, P: 'static +  Copy> {
    image_dims: DimsBox<D2, u32>,
    insert_over: InsertOver,
    atlas: &'a mut SkylineAtlas<P>
}

#[derive(Debug, Clone)]
struct InsertOver {
    range: Range<usize>,
    width: u32,
    height: u32,
    space_lost: u32
}

impl<P: Copy> SkylineAtlas<P> {
    /// Create a new skyline atlas, clearing the background color to P.
    #[inline]
    pub fn new(dims: DimsBox<D2, u32>, background_color: P) -> SkylineAtlas<P> {
        let base_range = HeightRange {
            bounds_min: 0,
            bounds_max: dims.width(),
            height: 0
        };

        SkylineAtlas {
            raw: RawAtlas::new(dims, background_color),
            dims,
            heights: vec![base_range],
            max_used_height: 0
        }
    }

    /// Get the raw pixel data stored in the atlas.
    #[inline]
    pub fn pixels(&self) -> &[P] {
        &self.raw.pixels()
    }

    /// Get the 2D dimensions of the pixel array.
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

    /// Add a new image to the atlas.
    ///
    /// Returns the image's rectangle in the atlas, if the atlas had space for the image.
    #[must_use]
    pub fn add_image(&mut self, image_dims: DimsBox<D2, u32>, image_data: &[P]) -> Option<OffsetBox<D2, u32>> {
        self.add_image_view(image_dims, image_dims.into(), image_data)
    }

    /// Add a new sub-image to the atlas.
    ///
    /// Returns the image's rectangle in the atlas, if the atlas had space for the image.
    #[must_use]
    pub fn add_image_view(&mut self, image_dims: DimsBox<D2, u32>, image_view: OffsetBox<D2, u32>, image_data: &[P]) -> Option<OffsetBox<D2, u32>> {
        self.add_image_rows(image_view.dims(), rows_from_image(image_dims, image_view, image_data)).ok()
    }

    #[must_use]
    pub fn add_image_rows<'a, I>(&mut self, image_dims: DimsBox<D2, u32>, image_data: I) -> Result<OffsetBox<D2, u32>, I>
        where I: IntoIterator<Item=&'a [P]>,
              P: 'a
    {
        match self.calc_insert_over(image_dims) {
            Some(range) => {
                let insert_rect = self.insert_over(range, image_dims);
                self.blit_slice_iter(image_dims, insert_rect.min().to_vec(), image_data);
                Ok(insert_rect)
            },
            None => Err(image_data)
        }
    }

    #[must_use]
    pub fn add_image_pixels<'a, I>(&mut self, image_dims: DimsBox<D2, u32>, image_data: I) -> Result<OffsetBox<D2, u32>, I>
        where I: IntoIterator<Item=P>,
    {
        match self.calc_insert_over(image_dims) {
            Some(range) =>{
                let insert_rect = self.insert_over(range, image_dims);
                self.blit_pixel_iter(image_dims, insert_rect.min().to_vec(), image_data);
                Ok(insert_rect)
            },
            None => Err(image_data)
        }
    }

    #[must_use]
    pub fn add_image_deferred(&mut self, image_dims: DimsBox<D2, u32>) -> Option<AddImageDeferred<'_, P>> {
        self.calc_insert_over(image_dims)
            .map(move |insert_over| AddImageDeferred {
                image_dims,
                insert_over,
                atlas: self,
            })
    }

    /// Clear the atlas image to the given background color.
    pub fn clear(&mut self, background_color: Option<P>) {
        self.heights.clear();
        self.heights.push(HeightRange {
            bounds_min: 0,
            bounds_max: self.dims.width(),
            height: 0
        });

        if let Some(bgc) = background_color {
            self.raw.clear(bgc);
        }
    }

    /// Compact the atlas, based on the image rectangle data provided in the `rects` iter.
    pub fn compact<'a, I>(&mut self, rects: I)
        where I: IntoIterator<Item=&'a mut OffsetBox<D2, u32>>,
              P: Default
    {
        let mut old_pixels = RawAtlas::new(self.dims, P::default());
        mem::swap(&mut old_pixels, &mut self.raw);
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
            self.blit(dims, remove_rect.0, remove_rect.1.min().to_vec(), &old_pixels.pixels());
            removed_rects.push(remove_rect);
        }

        if reset_atlas {
            self.raw = old_pixels;
            self.heights = old_heights;
            for (old_rect, rect_ref) in rects_sorted.drain(..).chain(removed_rects.drain(..)) {
                *rect_ref = old_rect;
            }
        }
    }

    /// Directly blit image data onto the atlas.
    pub fn blit(&mut self, image_dims: DimsBox<D2, u32>, image_view: OffsetBox<D2, u32>, write_offset: Vector2<u32>, image_data: &[P]) {
        self.raw.blit_slice_iter(
            self.dims,
            rows_from_image(image_dims, image_view, image_data), image_view.dims(),
            write_offset
        );
    }

    pub fn blit_slice_iter<'a, I>(&mut self, image_dims: DimsBox<D2, u32>, write_offset: Vector2<u32>, image_data: I)
        where I: IntoIterator<Item=&'a [P]>,
              P: 'a
    {
        self.raw.blit_slice_iter(
            self.dims,
            image_data, image_dims,
            write_offset
        );
    }

    pub fn blit_pixel_iter<'a, I>(&mut self, image_dims: DimsBox<D2, u32>, write_offset: Vector2<u32>, image_data: I)
        where I: IntoIterator<Item=P>,
    {
        self.raw.blit_pixel_iter(
            self.dims,
            image_data, image_dims,
            write_offset
        );
    }
}

impl<P: Copy> AddImageDeferred<'_, P> {
    #[must_use]
    pub fn add_image_rows<'a, I>(self, image_data: I) -> OffsetBox<D2, u32>
        where I: IntoIterator<Item=&'a [P]>,
              P: 'a
    {
        let insert_rect = self.atlas.insert_over(self.insert_over, self.image_dims);
        self.atlas.blit_slice_iter(self.image_dims, insert_rect.min().to_vec(), image_data);
        insert_rect
    }

    #[must_use]
    pub fn add_image_pixels<'a, I>(self, image_data: I) -> OffsetBox<D2, u32>
        where I: IntoIterator<Item=P>,
    {
        let insert_rect = self.atlas.insert_over(self.insert_over, self.image_dims);
        self.atlas.blit_pixel_iter(self.image_dims, insert_rect.min().to_vec(), image_data);
        insert_rect
    }
}

impl HeightRange {
    #[inline]
    fn width(&self) -> u32 {
        self.bounds_max - self.bounds_min
    }
}

pub fn rows_from_image<'a, P: 'a>(image_dims: DimsBox<D2, u32>, image_view: OffsetBox<D2, u32>, image_data: &'a [P]) -> impl Iterator<Item=&'a [P]> {
    (image_view.min().y as usize..image_view.max().y as usize)
        .map(move |r| &image_data[
            image_dims.width() as usize * r + image_view.min().x as usize..
            image_dims.width() as usize * r + image_view.min().x as usize + image_view.width() as usize
        ])
}
