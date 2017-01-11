use super::{Tr, Px, Fr};
use super::geometry::Point;
use super::layout::{GridSize, DyRange};
use std::cmp;

#[derive(Debug, Clone, Copy)]
pub enum SizeResult {
    NoEffectEq,
    NoEffectDown,
    NoEffectUp,
    SizeUpscale,
    SizeUpscaleClamp,
    SizeDownscale,
    SizeDownscaleClamp
}

#[derive(Clone, Copy)]
pub struct GridTrack {
    /// The size of this grid track in pixels. For columns, this is the width; for rows, the height.
    size: Px,
    /// Track-level minimum size. If the child minimum size is less than this, this is used instead.
    min_size_master: Px,
    /// Track-level maximum size. If this is less than the minimum size, minimum size takes priority
    /// and overrides this.
    max_size_master: Px,
    pub fr_size: Fr
}

impl GridTrack {
    /// Get the size of this grid track in pixels.
    pub fn size(&self) -> Px {
        self.size
    }

    /// Get the track-level minimum size of this grid track in pixels.
    pub fn min_size_master(&self) -> Px {
        self.min_size_master
    }

    pub fn max_size_master(&self) -> Px {
        // If the maximum size is less than the minimum size, which is technically allowed to happen but
        // doesn't logically make sense, clamp the maximum size to the minimum size.
        cmp::max(self.max_size_master, self.min_size_master)
    }

    pub fn shrink_size(&mut self) {
        self.size = self.min_size_master();
    }

    pub fn expand_size(&mut self) {
        self.size = self.max_size_master();
    }

    pub fn change_size(&mut self, new_size: Px) -> SizeResult {
        if self.size < new_size && self.size != self.max_size_master() {
            if new_size <= self.max_size_master() {
                self.size = new_size;
                SizeResult::SizeUpscale
            } else {
                self.size = self.max_size_master();
                SizeResult::SizeUpscaleClamp
            }
        } else if self.size > new_size && self.size != self.min_size_master() {
            if new_size >= self.min_size_master() {
                self.size = new_size;
                SizeResult::SizeDownscale
            } else {
                self.size = self.min_size_master();
                SizeResult::SizeDownscaleClamp
            }
        } else if new_size < self.min_size_master() {
            SizeResult::NoEffectDown
        } else if new_size > self.max_size_master() {
            SizeResult::NoEffectUp
        } else {
            SizeResult::NoEffectEq
        }
    }

    /// Sets track-level minimum size.
    pub fn set_min_size_master(&mut self, min_size_master: Px) {
        self.min_size_master = min_size_master;
        self.size = cmp::max(self.size, self.min_size_master());
    }

    /// Sets track-level maximum size.
    pub fn set_max_size_master(&mut self, max_size_master: Px) {
        self.max_size_master = max_size_master;
        self.size = cmp::min(self.size, self.max_size_master());
    }
}

impl Default for GridTrack {
    fn default() -> GridTrack {
        GridTrack {
            size: 0,

            min_size_master: 0,
            max_size_master: Px::max_value(),
            fr_size: 0.0
        }
    }
}

#[derive(Default, Clone)]
pub struct TrackVec<T = GridTrack> {
    num_cols: Tr,
    num_rows: Tr,
    /// A vector that contains the dimensions of the rows and columns of the grid. The first `num_cols`
    /// elements are the column widths, the next `num_rows` elements are the row heights.
    dims: Vec<T>
}

impl<T> TrackVec<T> {
    /// Create a new TrackVec<T>
    pub fn new() -> TrackVec<T> {
        TrackVec {
            num_cols: 0,
            num_rows: 0,
            dims: Vec::new()
        }
    }

    /// Set the number of columns and rows in the layout.
    pub fn set_grid_size(&mut self, size: GridSize)
            where T: Default + Clone
    {
        self.set_num_cols(size.x);
        self.set_num_rows(size.y);
    }

    pub fn set_num_cols(&mut self, num_cols: Tr)
            where T: Default + Clone
    {
        use std::ptr;
        unsafe {
            let old_num_cols = self.num_cols;
            let num_rows = self.num_rows;

            // If the new length of the vector is going to be greater than the current length of the vector,
            // extend it before doing any calculations. Why not resize if the vector is going to be shorter?
            // Well, we need to shift the row data over, so if we resize the vector before doing that we're
            // going to be shifting from undefined data!
            if num_cols > old_num_cols {
                self.dims.resize((num_cols + num_rows) as usize, T::default());
            }

            // Drop any columns that are going to be removed.
            for col in self.col_range_mut(num_cols..old_num_cols) {
                ptr::drop_in_place(col);
            }

            // Shift the row data over.
            ptr::copy(&self.dims[old_num_cols as usize], &mut self.dims[num_cols as usize], num_rows as usize);

            // If the number of columns was increased and the row data shifted to the right, fill the new
            // empty space with the default for the data type. In the event that it was shifted to the left
            // or not shifted at all, nothing is done due to the saturating subtraction.
            for gt in &mut self.dims[old_num_cols as usize..(old_num_cols + num_cols.saturating_sub(old_num_cols)) as usize] {
                *gt = T::default();
            }

            self.num_cols = num_cols;
        }
    }

    pub fn set_num_rows(&mut self, num_rows: Tr)
            where T: Default + Clone
    {
        self.dims.resize((self.num_cols + num_rows) as usize, T::default());
        self.num_rows = num_rows;
    }

    pub fn push_col(&mut self, col: T) {
        self.dims.insert(self.num_cols as usize, col);
        self.num_cols += 1;
    }

    pub fn push_row(&mut self, row: T) {
        self.dims.push(row);
        self.num_rows += 1;
    }

    pub fn remove_col(&mut self, column_num: u32) {
        self.dims.remove(column_num as usize);
        self.num_cols -= 1;
    }

    pub fn remove_row(&mut self, row_num: u32) {
        self.dims.remove((self.num_cols + row_num) as usize);
        self.num_rows -= 1;
    }


    /// Get a reference to the column at the specified column number. Returns `None` if the number
    /// is >= `num_cols`.
    pub fn get_col(&self, column_num: Tr) -> Option<&T> {
        if column_num < self.num_cols {
            self.dims.get(column_num as usize)
        } else {
            None
        }
    }

    /// Get a reference to the row at the specified row number. Returns `None` if the number
    /// is >= `num_rows`.
    pub fn get_row(&self, row_num: Tr) -> Option<&T> {
        self.dims.get((self.num_cols + row_num) as usize)
    }

    /// Get a mutable to the column at the specified column number. Returns `None` if the number
    /// is >= `num_cols`.
    pub fn get_col_mut(&mut self, column_num: Tr) -> Option<&mut T> {
        if column_num < self.num_cols {
            self.dims.get_mut(column_num as usize)
        } else {
            None
        }
    }

    /// Get a mutable reference to the row at the specified row number. Returns `None` if the number
    /// is >= `num_rows`.
    pub fn get_row_mut(&mut self, row_num: Tr) -> Option<&mut T> {
        self.dims.get_mut((self.num_cols + row_num) as usize)
    }

    /// Take a range and get a slice of columns corresponding to that range. Returns `None` if the
    /// range specifies columns that don't exist.
    pub fn col_range<R>(&self, range: R) -> Option<&[T]>
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
    pub fn row_range<R>(&self, range: R) -> Option<&[T]>
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
    pub fn col_range_mut<R>(&mut self, range: R) -> Option<&mut [T]>
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
    pub fn row_range_mut<R>(&mut self, range: R) -> Option<&mut [T]>
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

impl TrackVec<GridTrack> {
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

    /// Get the total width of the layout in pixels.
    pub fn width(&self) -> Px {
        self.col_range(0..self.num_cols).unwrap().iter()
            .map(|c| c.size()).sum()
    }

    /// Get the total height of the layout in pixels.
    pub fn height(&self) -> Px {
        self.row_range(0..self.num_rows).unwrap().iter()
            .map(|r| r.size()).sum()
    }

    /// Get the maximum width of the layout in pixels
    pub fn max_width(&self) -> Px {
        self.col_range(0..self.num_cols).unwrap().iter()
            .fold(0, |acc, c| acc.saturating_add(c.max_size_master()))
    }

    /// Get the maximum height of the layout in pixels
    pub fn max_height(&self) -> Px {
        self.row_range(0..self.num_rows).unwrap().iter()
            .fold(0, |acc, r| acc.saturating_add(r.max_size_master()))
    }
}
