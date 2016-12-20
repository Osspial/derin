use super::geometry::{OriginRect, OffsetRect, Rect, Point};
use super::layout::{NodeSpan, GridSize};
use std::ops::Range;
use std::cmp::Ordering;

pub enum SetSizeResult {
    /// The track's size is completely unchanged.
    NoEffect,
    /// The passed cell was the biggest in the track, but no longer is. The size of the track is therefore
    /// smaller than it was, but `GridDims` cannot determine how much smaller. It must be recalculated by
    /// calling `set_{col|row}_cell_{width|height}` for each widget in the track.
    SizeDownscale,
    /// The track's new size is larger than it was before `set_{col|row}_cell_{width|height}` was called.
    SizeUpscale
}

pub enum SetMinSizeResult {
    /// The track's minimum size is completely unchanged
    NoEffect,
    /// The track's minimum size is smaller than it was before the function was called. It must be
    /// recalculated by calling `set_{col|row}_cell_min_{width|height}` for each widget in the track.
    MinSizeDownscale,
    /// The track's minimum size was increased, but not enough to result requiring any recalculations.
    MinSizeUpscale,
    /// The track's minimum with was increased past the track size, resulting in the track size becoming
    /// larger.
    SizeUpscale
}

#[derive(Clone, Copy)]
pub enum GridTrack {
    Bounded(BoundedTrack),
    Definite(u32)
}

impl GridTrack {
    pub fn size_px(self) -> u32 {
        match self {
            GridTrack::Bounded(BoundedTrack{size_px, ..}) |
            GridTrack::Definite(size_px) => size_px
        }
    }

    pub fn min_size_px(self) -> u32 {
        match self {
            GridTrack::Bounded(BoundedTrack{min_size_px, ..}) |
            GridTrack::Definite(min_size_px) => min_size_px
        }
    }
}

#[derive(Clone, Copy)]
pub struct BoundedTrack {
    /// The size, in pixels, of this grid track. For columns, this is the width; for rows, the height.
    size_px: u32,
    num_biggest: u32,
    min_size_px: u32,
    min_num_biggest: u32
}

impl BoundedTrack {
    pub fn size_px(&self) -> u32 {
        self.size_px
    }

    pub fn min_size_px(&self) -> u32 {
        self.min_size_px
    }

    /// Set the size of a single cell in the track. Note that this does *not* necessarily set the
    /// actual size of the track, but instead takes into account the sizes of other cells in the track
    /// to determine whether or not to downscale the track's size, upscale it, or leave it unchanged. 
    pub fn set_cell_size_px(&mut self, mut new_size: u32, old_size: u32) -> SetSizeResult {
        let is_biggest_size = self.size_px <= new_size;
        let was_biggest_size = self.size_px == old_size;

        self.num_biggest += is_biggest_size as u32;
        self.num_biggest -= was_biggest_size as u32;

        if self.size_px < new_size {
            self.size_px = new_size;
            self.num_biggest = 1;

            SetSizeResult::SizeUpscale
        } else if self.num_biggest == 0 {
            self.size_px = 0;
            self.num_biggest = 0;

            SetSizeResult::SizeDownscale
        } else {
            SetSizeResult::NoEffect
        }
    }

    pub fn set_cell_min_size_px(&mut self, new_min_size: u32, old_min_size: u32) -> SetMinSizeResult {
        let mut ret = SetMinSizeResult::NoEffect;

        let is_biggest_min_size = self.min_size_px <= new_min_size;
        let was_biggest_min_size = self.min_size_px == old_min_size;

        self.min_num_biggest += is_biggest_min_size as u32;
        self.min_num_biggest -= was_biggest_min_size as u32;

        if self.min_size_px < new_min_size {
            self.min_size_px = new_min_size;
            self.min_num_biggest = 1;

            ret = SetMinSizeResult::MinSizeUpscale;
        } else if self.min_num_biggest == 0 {
            self.min_size_px = 0;
            self.min_num_biggest = 0;

            ret = SetMinSizeResult::MinSizeDownscale;
        }

        if self.size_px < new_min_size {
            self.size_px = new_min_size;
            ret = SetMinSizeResult::SizeUpscale;
        }

        ret
    }
}

impl Default for BoundedTrack {
    fn default() -> BoundedTrack {
        BoundedTrack {
            size_px: 0,
            num_biggest: 0,
            min_size_px: 0,
            min_num_biggest: 0
        }
    }
}

#[derive(Default, Clone)]
pub struct GridDims {
    num_cols: u32,
    num_rows: u32,
    /// A vector that contains the dimensions of the rows and columns of the grid. The first `num_cols`
    /// elements are the column widths, the next `num_rows` elements are the row heights.
    dims: Vec<GridTrack>
}

impl GridDims {
    pub fn new() -> GridDims {
        GridDims {
            num_cols: 0,
            num_rows: 0,
            dims: Vec::new()
        }
    }

    pub fn set_grid_size(&mut self, size: GridSize) {
        use std::ptr;

        unsafe {
            if 0 < size.x + size.y {
                // If the new length of the vector is going to be greater than the current length of the vector,
                // extend it before doing any calculations. Why not resize if the vector is going to be shorter?
                // Well, we need to shift the row data over, so if we resize the vector before doing that we're
                // going to be shifting from undefined data!
                if size.x + size.y > self.num_cols + self.num_rows {
                    self.dims.resize((size.x + size.y) as usize, GridTrack::Definite(0));
                }

                // Shift the row data over, if it actually needs shifting.
                if size.x != self.num_cols {
                    ptr::copy(&self.dims[self.num_cols as usize], &mut self.dims[size.x as usize], self.num_rows as usize);
                }
                // If we shifted the row data to the right, fill the new empty space with zeroes. In the event that
                // it was shifted to the left or not shifted at all, nothing is done due to the saturating subtraction.
                ptr::write_bytes(&mut self.dims[self.num_cols as usize], 0, size.x.saturating_sub(self.num_cols) as usize);
                
                self.num_cols = size.x;
                self.num_rows = size.y;

                // Finally, set the length of the vector to be correct. This would have been done already if the
                // grid's size was expanded, but if it was decreased we need to do it here.
                self.dims.set_len((size.x + size.y) as usize);
            }
        }
    }

    pub fn column_width(&self, column_num: u32) -> Option<u32> {
        self.get_col(column_num).map(|gl| gl.size_px())
    }

    pub fn row_height(&self, row_num: u32) -> Option<u32> {
        self.get_row(row_num).map(|gl| gl.size_px())
    }

    pub fn get_cell_offset(&self, column_num: u32, row_num: u32) -> Option<Point> {
        // This process could probably be sped up with Rayon. Something to come back to.
        if column_num < self.num_cols &&
           row_num < self.num_rows
        {
            Some(Point::new(
                (0..column_num).map(|c| self.get_col(c).unwrap().size_px()).sum(),
                (0..row_num).map(|r| self.get_row(r).unwrap().size_px()).sum()
            ))
        } else {
            None
        }
    }

    /// Get the rect of the cell, without accounting for offset.
    pub fn get_cell_origin_rect(&self, column_num: u32, row_num: u32) -> Option<OriginRect> {
        self.column_width(column_num)
            .and_then(|cw| self.row_height(row_num)
                 .map(|rh| OriginRect::new(cw, rh))
            )
    }

    /// Get the rect of the cell, accounting for offset.
    pub fn get_cell_rect(&self, column_num: u32, row_num: u32) -> Option<OffsetRect> {
        self.get_cell_origin_rect(column_num, row_num)
            .map(|rect| rect.offset(self.get_cell_offset(column_num, row_num).unwrap()))
    }

    pub fn get_span_origin_rect(&self, span: NodeSpan) -> Option<OriginRect> {
        let col_range = span.x.start.unwrap_or(0)..span.x.end.unwrap_or(self.num_cols);
        let row_range = span.y.start.unwrap_or(0)..span.y.end.unwrap_or(self.num_rows);

        if col_range.end <= self.num_cols &&
           row_range.end <= self.num_rows
        {
            Some(OriginRect::new(
                self.col_iter(col_range).unwrap().map(|t| t.size_px()).sum(),
                self.row_iter(row_range).unwrap().map(|t| t.size_px()).sum()
            ))
        } else {
            None
        }
    }

    pub fn width(&self) -> u32 {
        self.col_iter(0..self.num_cols).unwrap().map(|c| c.size_px()).sum()
    }

    pub fn height(&self) -> u32 {
        self.row_iter(0..self.num_rows).unwrap().map(|r| r.size_px()).sum()
    }

    pub fn min_width(&self) -> u32 {
        self.col_iter(0..self.num_cols).unwrap().map(|c| c.min_size_px()).sum()
    }

    pub fn min_height(&self) -> u32 {
        self.row_iter(0..self.num_rows).unwrap().map(|r| r.min_size_px()).sum()
    }

    pub fn get_col(&self, column_num: u32) -> Option<&GridTrack> {
        if column_num < self.num_cols {
            self.dims.get(column_num as usize)
        } else {
            None
        }
    }

    pub fn get_row(&self, row_num: u32) -> Option<&GridTrack> {
        self.dims.get((self.num_cols + row_num) as usize)
    }

    pub fn get_col_mut(&mut self, column_num: u32) -> Option<&mut GridTrack> {
        if column_num < self.num_cols {
            self.dims.get_mut(column_num as usize)
        } else {
            None
        }
    }

    pub fn get_row_mut(&mut self, row_num: u32) -> Option<&mut GridTrack> {
        self.dims.get_mut((self.num_cols + row_num) as usize)
    }

    pub fn get_cell_tracks_mut(&mut self, column_num: u32, row_num: u32) -> (Option<&mut GridTrack>, Option<&mut GridTrack>) {
        if self.num_cols <= column_num {
            (None, self.get_row_mut(row_num))
        } else {
            let (cols, rows) = self.dims.split_at_mut(self.num_cols as usize);
            (cols.get_mut(column_num as usize), rows.get_mut(row_num as usize))
        }
    }

    pub fn col_iter<'a>(&'a self, range: Range<u32>) -> Option<impl Iterator<Item = &'a GridTrack>> {
        if range.end <= self.num_cols {
            let range_usize = range.start as usize..range.end as usize;
            Some(self.dims[range_usize].iter())
        } else {
            None
        }
    }

    pub fn row_iter<'a>(&'a self, range: Range<u32>) -> Option<impl Iterator<Item = &'a GridTrack>> {
        if range.end <= self.dims.len() as u32 {
            let range_usize = (range.start + self.num_cols) as usize..(range.end + self.num_cols)as usize;
            Some(self.dims[range_usize].iter())
        } else {
            None
        }
    }

    pub fn col_iter_mut<'a>(&'a mut self, range: Range<u32>) -> Option<impl Iterator<Item = &'a mut GridTrack>> {
        if range.end <= self.num_cols {
            let range_usize = range.start as usize..range.end as usize;
            Some(self.dims[range_usize].iter_mut())
        } else {
            None
        }
    }

    pub fn row_iter_mut<'a>(&'a mut self, range: Range<u32>) -> Option<impl Iterator<Item = &'a mut GridTrack>> {
        if range.end <= self.dims.len() as u32 {
            let range_usize = (range.start + self.num_cols) as usize..(range.end + self.num_cols)as usize;
            Some(self.dims[range_usize].iter_mut())
        } else {
            None
        }
    }
}
