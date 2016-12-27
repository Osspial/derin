use super::{Tr, Px};
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
pub struct GridTrack {
    /// The size of this grid track in pixels. For columns, this is the width; for rows, the height.
    size: Px,
    /// The number of cells in this track that are as large as the track's size.
    num_biggest: u32,

    /// The minimum size of the cells in the grid track.
    min_size: Px,
    /// The number of cells in this track that have a minimum size equal to the track's minimum size.
    min_num_biggest: u32,

    /// Track-level minimum size. If the child minimum size is less than this, this is used instead.
    min_size_master: Px,
    /// Track-level maximum size. If this is less than the minimum size, minimum size takes priority
    /// and overrides this.
    max_size_master: Px
}

impl GridTrack {
    /// Get the size of this grid track in pixels.
    pub fn size(&self) -> Px {
        self.size
    }

    /// Get the minimum size of this grid track in pixels.
    pub fn min_size(&self) -> Px {
        cmp::max(self.min_size, self.min_size_master)
    }

    pub fn max_size(&self) -> Px {
        // If the maximum size is less than the minimum size, which is technically allowed to happen but
        // doesn't logically make sense, clamp the maximum size to the minimum size.
        cmp::max(self.max_size_master, self.min_size())
    }

    /// Set the size of a single cell in the track. Note that this does *not* necessarily set the
    /// actual size of the track, but instead takes into account the sizes of other cells in the track
    /// to determine whether or not to downscale the track's size, upscale it, or leave it unchanged.
    pub fn set_cell_size(&mut self, mut new_size: Px, old_size: Px) -> SizeResult {
        // Figure out if the cell WAS the biggest in the track and if it's GOING to be the biggest in the
        // track after the size. If it was the biggest, subtract one from the biggest size count. If it's
        // going to be, add one.
        let is_biggest_size = self.size <= new_size;
        let was_biggest_size = self.size == old_size;
        self.num_biggest += is_biggest_size as Px;
        self.num_biggest -= was_biggest_size as Px;

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

    /// Sets the minimum size of a single cell in the track. Like `set_cell_size`, this doesn't
    /// directly set the cell size but takes into account the minimum sizes of the other cells in the
    /// track.
    pub fn set_cell_min_size(&mut self, new_min_size: Px, old_min_size: Px) -> MinSizeResult {
        let mut ret = MinSizeResult::NoEffect;

        // Like in `set_cell_size`, was/will be biggest calculation and count incrementation.
        let is_biggest_min_size = self.min_size <= new_min_size;
        let was_biggest_min_size = self.min_size == old_min_size;
        self.min_num_biggest += is_biggest_min_size as Px;
        self.min_num_biggest -= was_biggest_min_size as Px;

        if self.min_size < new_min_size {
            self.min_size = new_min_size;
            self.min_num_biggest = 1;

            ret = MinSizeResult::MinSizeUpscale;
        } else if self.min_num_biggest == 0 {
            self.min_size = 0;
            self.min_num_biggest = 0;

            ret = MinSizeResult::MinSizeDownscale;
        }

        // If the new minimum size of the track is greater than the size of the track, increase the size
        // to equal the minimum size.
        if self.size < self.min_size() {
            self.size = self.min_size();
            ret = MinSizeResult::SizeUpscale;
        }

        ret
    }

    /// Sets track-level minimum size.
    pub fn set_min_size_master(&mut self, min_size_master: Px) -> MinSizeMasterResult {
        self.min_size_master = min_size_master;

        if self.size < self.min_size() {
            self.size = self.min_size();
            MinSizeMasterResult::SizeUpscale
        } else {
            MinSizeMasterResult::NoEffect
        }
    }

    /// Sets track-level maximum size.
    pub fn set_max_size_master(&mut self, max_size_master: Px) -> MaxSizeMasterResult {
        self.max_size_master = max_size_master;

        if self.size > self.max_size() {
            self.size = self.max_size();
            MaxSizeMasterResult::SizeDownscale
        } else {
            MaxSizeMasterResult::NoEffect
        }
    }
}

impl Default for GridTrack {
    fn default() -> GridTrack {
        GridTrack {
            size: 0,
            num_biggest: 0,

            min_size: 0,
            min_num_biggest: 0,

            min_size_master: 0,
            max_size_master: Px::max_value()
        }
    }
}

#[derive(Default, Clone)]
pub struct GridDims {
    num_cols: Tr,
    num_rows: Tr,
    /// A vector that contains the dimensions of the rows and columns of the grid. The first `num_cols`
    /// elements are the column widths, the next `num_rows` elements are the row heights.
    dims: Vec<GridTrack>
}

impl GridDims {
    /// Create a new GridDims
    pub fn new() -> GridDims {
        GridDims {
            num_cols: 0,
            num_rows: 0,
            dims: Vec::new()
        }
    }

    /// Set the number of columns and rows in the layout.
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
                    self.dims.resize((size.x + size.y) as usize, GridTrack::default());
                }

                // Shift the row data over, if the number of columns has changed.
                if size.x != old_num_cols {
                    ptr::copy(&self.dims[old_num_cols as usize], &mut self.dims[size.x as usize], old_num_rows as usize);
                }

                // If the number of columns was increased and the row data shifted to the right, fill the new
                // empty space with bounded tracks. In the event that it was shifted to the left or not shifted
                // at all, nothing is done due to the saturating subtraction.
                for gt in &mut self.dims[old_num_cols as usize..(old_num_cols + size.x.saturating_sub(old_num_cols)) as usize] {
                    *gt = GridTrack::default();
                }

                self.num_cols = size.x;
                self.num_rows = size.y;

                // Finally, set the length of the vector to be correct. This would have been done already if the
                // grid's size was expanded, but if it was decreased we need to do it here.
                self.dims.set_len((size.x + size.y) as usize);
            }
        }
    }

    /// Shrink down the internal dimensions vector as much as possible.
    pub fn shrink_to_fit(&mut self) {
        self.dims.shrink_to_fit();
    }

    /// Get the width of the specified column.
    pub fn column_width(&self, column_num: Tr) -> Option<Px> {
        self.get_col(column_num).map(|gt| gt.size())
    }

    /// Get the height of the specified row.
    pub fn row_height(&self, row_num: Tr) -> Option<Px> {
        self.get_row(row_num).map(|gt| gt.size())
    }

    /// Get the given cell's offset from the origin point of the layout.
    pub fn get_cell_offset(&self, column_num: Tr, row_num: Tr) -> Option<Point> {
        if column_num < self.num_cols &&
           row_num < self.num_rows
        {
            // Sum up the sizes of every column and row up to `column_num` and `row_num` variables. That sum
            // is the offset of the given column and row.
            Some(Point::new(
                (0..column_num).map(|c| self.get_col(c).unwrap().size()).sum(),
                (0..row_num).map(|r| self.get_row(r).unwrap().size()).sum()
            ))
        } else {
            None
        }
    }

    /// Get the rect of the cell, without accounting for offset.
    pub fn get_cell_origin_rect(&self, column_num: Tr, row_num: Tr) -> Option<OriginRect> {
        self.column_width(column_num)
            .and_then(|cw| self.row_height(row_num)
                 .map(|rh| OriginRect::new(cw, rh)))
    }

    /// Get the rect of the cell, accounting for offset.
    pub fn get_cell_rect(&self, column_num: Tr, row_num: Tr) -> Option<OffsetRect> {
        self.get_cell_origin_rect(column_num, row_num)
            .map(|rect| rect.offset(self.get_cell_offset(column_num, row_num).unwrap()))
    }

    /// Get the rect of the given span, without accounting for offset.
    pub fn get_span_origin_rect(&self, span: NodeSpan) -> Option<OriginRect> {
        if let Some(lr_x) = self.get_cols(span.x).map(|cols| cols.iter().map(|t| t.size()).sum()) {
            if let Some(lr_y) = self.get_rows(span.y).map(|rows| rows.iter().map(|t| t.size()).sum()) {
                return Some(OriginRect::new(lr_x, lr_y));
            }
        }

        None
    }


    /// Get the total width of the layout in pixels.
    pub fn width(&self) -> Px {
        self.get_cols(0..self.num_cols).unwrap().iter()
            .map(|c| c.size()).sum()
    }

    /// Get the total height of the layout in pixels.
    pub fn height(&self) -> Px {
        self.get_rows(0..self.num_rows).unwrap().iter()
            .map(|r| r.size()).sum()
    }

    /// Get the minimum width of the layout in pixels
    pub fn min_width(&self) -> Px {
        self.get_cols(0..self.num_cols).unwrap().iter()
            .map(|c| c.min_size()).sum()
    }

    /// Get the minimum height of the layout in pixels
    pub fn min_height(&self) -> Px {
        self.get_rows(0..self.num_rows).unwrap().iter()
            .map(|r| r.min_size()).sum()
    }

    /// Get the maximum width of the layout in pixels
    pub fn max_width(&self) -> Px {
        self.get_cols(0..self.num_cols).unwrap().iter()
            .fold(0, |acc, c| acc.saturating_add(c.max_size()))
    }

    /// Get the maximum height of the layout in pixels
    pub fn max_height(&self) -> Px {
        self.get_rows(0..self.num_rows).unwrap().iter()
            .fold(0, |acc, r| acc.saturating_add(r.max_size()))
    }


    /// Get a reference to the column at the specified column number. Returns `None` if the number
    /// is >= `num_cols`.
    pub fn get_col(&self, column_num: Tr) -> Option<&GridTrack> {
        if column_num < self.num_cols {
            self.dims.get(column_num as usize)
        } else {
            None
        }
    }

    /// Get a reference to the row at the specified row number. Returns `None` if the number
    /// is >= `num_rows`.
    pub fn get_row(&self, row_num: Tr) -> Option<&GridTrack> {
        self.dims.get((self.num_cols + row_num) as usize)
    }

    /// Get a mutable to the column at the specified column number. Returns `None` if the number
    /// is >= `num_cols`.
    pub fn get_col_mut(&mut self, column_num: Tr) -> Option<&mut GridTrack> {
        if column_num < self.num_cols {
            self.dims.get_mut(column_num as usize)
        } else {
            None
        }
    }

    /// Get a mutable reference to the row at the specified row number. Returns `None` if the number
    /// is >= `num_rows`.
    pub fn get_row_mut(&mut self, row_num: Tr) -> Option<&mut GridTrack> {
        self.dims.get_mut((self.num_cols + row_num) as usize)
    }

    /// Take a range and get a slice of columns corresponding to that range. Returns `None` if the
    /// range specifies columns that don't exist.
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

    /// Take a range and get a slice of rows corresponding to that range. Returns `None` if the
    /// range specifies rows that don't exist.
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

    /// Take a range and get a mutable slice of columns corresponding to that range. Returns `None` if the
    /// range specifies columns that don't exist.
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

    /// Take a range and get a mutable slice of rows corresponding to that range. Returns `None` if the
    /// range specifies rows that don't exist.
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
