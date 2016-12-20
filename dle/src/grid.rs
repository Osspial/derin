use super::geometry::{OriginRect, OffsetRect, Rect, Point};
use super::layout::{NodeSpan, GridSize, DyRange};
use std::ops::Range;
use std::cmp;

pub enum SizeResult {
    /// The track's size is completely unchanged.
    NoEffect,
    /// The passed cell was the biggest in the track, but no longer is. The size of the track is therefore
    /// smaller than it was, but `GridDims` cannot determine how much smaller. It must be recalculated by
    /// calling `set_{col|row}_cell_{width|height}` for each widget in the track.
    SizeDownscale,
    /// The track's new size is larger than it was before `set_{col|row}_cell_{width|height}` was called.
    SizeUpscale
}

pub enum MinSizeResult {
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

pub enum MinSizeMasterResult {
    NoEffect,
    SizeUpscale
}

pub enum MaxSizeMasterResult {
    NoEffect,
    SizeDownscale
}

#[derive(Clone, Copy)]
pub enum GridTrack {
    Bounded(BoundedTrack),
    Definite(u32)
}

impl GridTrack {
    pub fn size(self) -> u32 {
        match self {
            GridTrack::Bounded(BoundedTrack{size, ..}) |
            GridTrack::Definite(size) => size
        }
    }

    pub fn min_size(self) -> u32 {
        match self {
            GridTrack::Bounded(bt) => bt.min_size(),
            GridTrack::Definite(size) => size
        }
    }

    pub fn max_size(self) -> u32 {
        match self {
            GridTrack::Bounded(bt) => bt.max_size(),
            GridTrack::Definite(size) => size
        }
    }
}

#[derive(Clone, Copy)]
pub struct BoundedTrack {
    /// The size, in pixels, of this grid track. For columns, this is the width; for rows, the height.
    size: u32,
    num_biggest: u32,

    min_size: u32,
    min_num_biggest: u32,

    min_size_master: u32,
    max_size_master: u32
}

impl BoundedTrack {
    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn min_size(&self) -> u32 {
        cmp::max(self.min_size, self.min_size_master)
    }

    pub fn max_size(&self) -> u32 {
        // If the maximum size is less than the minimum size, which is technically allowed to happen but
        // doesn't logically make sense, clamp the maximum size to the minimum size.
        cmp::max(self.max_size_master, self.min_size)
    }

    /// Set the size of a single cell in the track. Note that this does *not* necessarily set the
    /// actual size of the track, but instead takes into account the sizes of other cells in the track
    /// to determine whether or not to downscale the track's size, upscale it, or leave it unchanged. 
    pub fn set_cell_size(&mut self, mut new_size: u32, old_size: u32) -> SizeResult {
        let is_biggest_size = self.size <= new_size;
        let was_biggest_size = self.size == old_size;

        self.num_biggest += is_biggest_size as u32;
        self.num_biggest -= was_biggest_size as u32;

        if self.size < new_size && self.size != self.max_size() {
            self.size = cmp::min(new_size, self.max_size());
            self.num_biggest = 1;

            SizeResult::SizeUpscale
        } else if self.num_biggest == 0 {
            self.size = 0;
            self.num_biggest = 0;

            SizeResult::SizeDownscale
        } else {
            SizeResult::NoEffect
        }
    }

    pub fn set_cell_min_size(&mut self, new_min_size: u32, old_min_size: u32) -> MinSizeResult {
        let mut ret = MinSizeResult::NoEffect;

        let is_biggest_min_size = self.min_size <= new_min_size;
        let was_biggest_min_size = self.min_size == old_min_size;

        self.min_num_biggest += is_biggest_min_size as u32;
        self.min_num_biggest -= was_biggest_min_size as u32;

        if self.min_size < new_min_size {
            self.min_size = new_min_size;
            self.min_num_biggest = 1;

            ret = MinSizeResult::MinSizeUpscale;
        } else if self.min_num_biggest == 0 {
            self.min_size = 0;
            self.min_num_biggest = 0;

            ret = MinSizeResult::MinSizeDownscale;
        }

        if self.size < self.min_size() {
            self.size = self.min_size();
            ret = MinSizeResult::SizeUpscale;
        }

        ret
    }

    pub fn set_min_size_master(&mut self, min_size_master: u32) -> MinSizeMasterResult {
        self.min_size_master = min_size_master;

        if self.size < self.min_size() {
            self.size = self.min_size();
            MinSizeMasterResult::SizeUpscale
        } else {
            MinSizeMasterResult::NoEffect
        }
    }

    pub fn set_max_size_master(&mut self, max_size_master: u32) -> MaxSizeMasterResult {
        self.max_size_master = max_size_master;

        if self.size > self.max_size() {
            self.size = self.max_size();
            MaxSizeMasterResult::SizeDownscale
        } else {
            MaxSizeMasterResult::NoEffect
        }
    }
}

impl Default for BoundedTrack {
    fn default() -> BoundedTrack {
        BoundedTrack {
            size: 0,
            num_biggest: 0,
            
            min_size: 0,
            min_num_biggest: 0,

            min_size_master: 0,
            max_size_master: u32::max_value()
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
                let old_num_cols = self.num_cols;
                let old_num_rows = self.num_rows;
                // If the new length of the vector is going to be greater than the current length of the vector,
                // extend it before doing any calculations. Why not resize if the vector is going to be shorter?
                // Well, we need to shift the row data over, so if we resize the vector before doing that we're
                // going to be shifting from undefined data!
                if size.x + size.y > old_num_cols + old_num_rows {
                    self.dims.resize((size.x + size.y) as usize, GridTrack::Bounded(BoundedTrack::default()));
                }

                // Shift the row data over, if the number of columns has changed.
                if size.x != old_num_cols {
                    ptr::copy(&self.dims[old_num_cols as usize], &mut self.dims[size.x as usize], old_num_rows as usize);
                }

                // If the number of columns was increased and the row data shifted to the right, fill the new 
                // empty space with bounded tracks. In the event that it was shifted to the left or not shifted
                // at all, nothing is done due to the saturating subtraction.
                for gt in &mut self.dims[old_num_cols as usize..(old_num_cols + size.x.saturating_sub(old_num_cols)) as usize] {
                    *gt = GridTrack::Bounded(BoundedTrack::default());
                }
                
                self.num_cols = size.x;
                self.num_rows = size.y;

                // Finally, set the length of the vector to be correct. This would have been done already if the
                // grid's size was expanded, but if it was decreased we need to do it here.
                self.dims.set_len((size.x + size.y) as usize);
            }
        }
    }

    pub fn column_width(&self, column_num: u32) -> Option<u32> {
        self.get_col(column_num).map(|gt| gt.size())
    }

    pub fn row_height(&self, row_num: u32) -> Option<u32> {
        self.get_row(row_num).map(|gt| gt.size())
    }

    pub fn get_cell_offset(&self, column_num: u32, row_num: u32) -> Option<Point> {
        // This process could probably be sped up with Rayon. Something to come back to.
        if column_num < self.num_cols &&
           row_num < self.num_rows
        {
            Some(Point::new(
                (0..column_num).map(|c| self.get_col(c).unwrap().size()).sum(),
                (0..row_num).map(|r| self.get_row(r).unwrap().size()).sum()
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
                self.get_cols(col_range).unwrap().iter().map(|t| t.size()).sum(),
                self.get_rows(row_range).unwrap().iter().map(|t| t.size()).sum()
            ))
        } else {
            None
        }
    }


    pub fn width(&self) -> u32 {
        self.get_cols(0..self.num_cols).unwrap().iter()
            .map(|c| c.size()).sum()
    }

    pub fn height(&self) -> u32 {
        self.get_rows(0..self.num_rows).unwrap().iter()
            .map(|r| r.size()).sum()
    }

    pub fn min_width(&self) -> u32 {
        self.get_cols(0..self.num_cols).unwrap().iter()
            .map(|c| c.min_size()).sum()
    }

    pub fn min_height(&self) -> u32 {
        self.get_rows(0..self.num_rows).unwrap().iter()
            .map(|r| r.min_size()).sum()
    }

    pub fn max_width(&self) -> u32 {
        self.get_cols(0..self.num_cols).unwrap().iter()
            .fold(0, |acc, c| acc.saturating_add(c.max_size()))
    }


    pub fn max_height(&self) -> u32 {
        self.get_rows(0..self.num_rows).unwrap().iter()
            .fold(0, |acc, r| acc.saturating_add(r.max_size()))
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

    pub fn get_cols<R>(&self, range: R) -> Option<&[GridTrack]>
            where R: Into<DyRange<u32>>
    {
        let range = range.into();
        let range_usize = range.start.unwrap_or(0) as usize
                          ..range.end.unwrap_or(self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_cols {
            Some(&self.dims[range_usize])
        } else {
            None
        }
    }

    pub fn get_rows<R>(&self, range: R) -> Option<&[GridTrack]>
            where R: Into<DyRange<u32>>
    {
        let range = range.into();
        let range_usize = (range.start.unwrap_or(0) + self.num_cols) as usize
                          ..(range.end.unwrap_or(self.num_rows) + self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_cols {
            Some(&self.dims[range_usize])
        } else {
            None
        }
    }

    pub fn get_cols_mut<R>(&mut self, range: R) -> Option<&mut [GridTrack]>
            where R: Into<DyRange<u32>>
    {
        let range = range.into();
        let range_usize = range.start.unwrap_or(0) as usize
                          ..range.end.unwrap_or(self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_cols {
            Some(&mut self.dims[range_usize])
        } else {
            None
        }
    }

    pub fn get_rows_mut<R>(&mut self, range: R) -> Option<&mut [GridTrack]>
            where R: Into<DyRange<u32>>
    {
        let range = range.into();
        let range_usize = (range.start.unwrap_or(0) + self.num_cols) as usize
                          ..(range.end.unwrap_or(self.num_rows) + self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_cols {
            Some(&mut self.dims[range_usize])
        } else {
            None
        }
    }
}
