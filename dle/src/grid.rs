use Tr;
use dct::geometry::{Px, Point};
use dct::hints::{GridSize, TrRange, TrackHints};

use std::cmp;
use std::fmt::{Debug, Formatter, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeResult {
    NoEffectEq,
    NoEffectDown,
    NoEffectUp,
    SizeUpscale,
    SizeUpscaleClamp,
    SizeDownscale,
    SizeDownscaleClamp
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct GridTrack {
    /// The size of this grid track in pixels. For columns, this is the width; for rows, the height.
    /// This must be greater than `min_size` and less than `max_size`.
    size: Px,
    widget_min_size: Px,
    hints: TrackHints
}

impl GridTrack {
    /// Get the size of this grid track in pixels.
    pub fn size(&self) -> Px {
        self.size
    }

    /// Get the minimum size of this grid track in pixels.
    pub fn min_size(&self) -> Px {
        cmp::max(self.hints.min_size, self.widget_min_size)
    }

    /// Get the maximum size of this grid track in pixels.
    pub fn max_size(&self) -> Px {
        // If the maximum size is less than the minimum size, which is technically allowed to happen but
        // doesn't logically make sense, clamp the maximum size to the minimum size.
        cmp::max(self.hints.max_size, self.hints.min_size)
    }

    /// Reset the widget minimum size and track size to the track minimum size.
    pub fn reset_shrink(&mut self) {
        self.size = self.hints.min_size;
        self.widget_min_size = self.hints.min_size;
    }

    pub fn reset_expand(&mut self) {
        self.size = self.max_size();
    }

    pub fn change_size(&mut self, new_size: Px) -> SizeResult {
        if self.size < new_size && self.size != self.max_size() {
            if new_size <= self.max_size() {
                self.size = new_size;
                SizeResult::SizeUpscale
            } else {
                self.size = self.max_size();
                SizeResult::SizeUpscaleClamp
            }
        } else if self.size > new_size && self.size != self.min_size() {
            if new_size >= self.min_size() {
                self.size = new_size;
                SizeResult::SizeDownscale
            } else {
                self.size = self.min_size();
                SizeResult::SizeDownscaleClamp
            }
        } else if new_size < self.min_size() {
            SizeResult::NoEffectDown
        } else if new_size > self.max_size() {
            SizeResult::NoEffectUp
        } else {
            SizeResult::NoEffectEq
        }
    }

    /// Expand the widget minimum size of the track. Returns `Ok(())` if the track size is unchanged, and
    /// `Err(size_expand)` if the track size was increased.
    pub fn expand_widget_min_size(&mut self, widget_min_size: Px) -> Result<(), Px> {
        self.widget_min_size = cmp::max(self.widget_min_size, widget_min_size);

        if self.min_size() <= self.size {
            Ok(())
        } else {
            let old_size = self.size;
            self.size = self.min_size();
            Err(self.size - old_size)
        }
    }

    #[inline]
    pub fn hints(&self) -> TrackHints {
        self.hints
    }

    /// Set the hints for the track. If the track size is outside the bounds of the new
    /// minimum or maximum sizes, bound the size to that range and return an error with the change
    /// in grid size. Note that this doesn't change the track size based on `fr_size` - a full grid
    /// update is needed to do that.
    pub fn set_hints(&mut self, hints: TrackHints) -> Result<(), i32> {
        self.hints = hints;
        self.expand_widget_min_size(hints.min_size).map_err(|px| px as i32)?;
        if self.max_size() >= self.size {
            Ok(())
        } else {
            let old_size = self.size;
            self.size = self.max_size();
            Err(self.size as i32 - old_size as i32)
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
            } else if num_cols < old_num_cols {
                // Drop any columns that are going to be removed.
                for col in self.col_range_mut(num_cols..old_num_cols) {
                    ptr::drop_in_place(col);
                }
            }

            if 0 < num_rows {
                // Shift the row data over.
                ptr::copy(&self.dims[old_num_cols as usize], &mut self.dims[num_cols as usize], num_rows as usize);
            }

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

    pub fn clear(&mut self) {
        self.dims.clear();
        self.num_cols = 0;
        self.num_rows = 0;
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
            where R: Into<TrRange>
    {
        let range = range.into();
        let range_usize = range.start.unwrap_or(0) as usize
            ..cmp::min(range.end.unwrap_or(self.num_cols), self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_cols {
            self.dims.get(range_usize)
        } else {
            None
        }
    }

    /// Take a range and get a slice of rows corresponding to that range. Returns `None` if the
    /// range specifies rows that don't exist.
    pub fn row_range<R>(&self, range: R) -> Option<&[T]>
            where R: Into<TrRange>
    {
        let range = range.into();
        let range_usize = (range.start.unwrap_or(0) + self.num_cols) as usize
            ..(cmp::min(range.end.unwrap_or(self.num_rows), self.num_rows) + self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_rows + self.num_cols {
            self.dims.get(range_usize)
        } else {
            None
        }
    }

    /// Take a range and get a mutable slice of columns corresponding to that range. Returns `None` if the
    /// range specifies columns that don't exist.
    pub fn col_range_mut<R>(&mut self, range: R) -> Option<&mut [T]>
            where R: Into<TrRange>
    {
        let range = range.into();
        let range_usize = range.start.unwrap_or(0) as usize
            ..cmp::min(range.end.unwrap_or(self.num_cols), self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_cols {
            self.dims.get_mut(range_usize)
        } else {
            None
        }
    }

    /// Take a range and get a mutable slice of rows corresponding to that range. Returns `None` if the
    /// range specifies rows that don't exist.
    pub fn row_range_mut<R>(&mut self, range: R) -> Option<&mut [T]>
            where R: Into<TrRange>
    {
        let range = range.into();
        let range_usize = (range.start.unwrap_or(0) + self.num_cols) as usize
            ..(cmp::min(range.end.unwrap_or(self.num_rows), self.num_rows) + self.num_cols) as usize;

        if range_usize.end as u32 <= self.num_rows + self.num_cols {
            self.dims.get_mut(range_usize)
        } else {
            None
        }
    }

    pub fn num_cols(&self) -> Tr {
        self.num_cols
    }

    pub fn num_rows(&self) -> Tr {
        self.num_rows
    }

    pub fn grid_size(&self) -> GridSize {
        GridSize::new(self.num_cols, self.num_rows)
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
}

impl<T: Debug> Debug for TrackVec<T> {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        fmt.debug_struct("TrackVec")
            .field("cols", &self.col_range(..).unwrap())
            .field("rows", &self.row_range(..).unwrap())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use std::cmp;

    quickcheck!{
        fn track_vec_set_num_cols(track_vec: TrackVec, new_num_cols: Tr) -> bool {
            let mut ntv = track_vec.clone();
            ntv.set_num_cols(new_num_cols);

            // Test that the column data is unchanged (except for truncation)
            track_vec.col_range(..cmp::min(new_num_cols, track_vec.num_cols))
            == ntv.col_range(..cmp::min(new_num_cols, track_vec.num_cols))
            &&
            // Test that the row data is unchanged
            track_vec.row_range(..) == ntv.row_range(..)
        }

        // We don't test `TrackVec.set_num_rows` because the implementation is the stock vec implementation.

        fn grid_track_change_size(grid_track: GridTrack, new_size: Px) -> bool {
            let set_size_result = grid_track.clone().change_size(new_size);

            set_size_result == if new_size < grid_track.min_size() {
                if grid_track.min_size() < grid_track.size() {
                    SizeResult::SizeDownscaleClamp
                } else {
                    SizeResult::NoEffectDown
                }
            } else if grid_track.max_size() < new_size {
                if grid_track.size() < grid_track.max_size() {
                    SizeResult::SizeUpscaleClamp
                } else {
                    SizeResult::NoEffectUp
                }
            } else {
                if grid_track.size() < new_size {
                    SizeResult::SizeUpscale
                } else if new_size < grid_track.size() {
                    SizeResult::SizeDownscale
                } else {
                    SizeResult::NoEffectEq
                }
            }
        }
    }

    impl<T> TrackVec<T> {
        fn clear_cols(&mut self) {
            for _ in self.dims.drain(0..self.num_cols as usize) {}
            self.num_cols = 0;
        }

        fn clear_rows(&mut self) {
            self.dims.truncate(self.num_cols as usize);
            self.num_rows = 0;
        }
    }

    impl Arbitrary for GridTrack {
        fn arbitrary<G: Gen>(g: &mut G) -> GridTrack {
            let mut track = GridTrack::default();
            track.set_min_size(g.next_u32());
            track.set_max_size(g.next_u32());
            track.change_size(g.next_u32());
            track.fr_size = g.next_f32();
            track
        }
    }

    impl<A: Arbitrary> Arbitrary for TrackVec<A> {
        fn arbitrary<G: Gen>(g: &mut G) -> TrackVec<A> {
            let size = g.size();
            let num_cols = g.gen_range(0, size);
            let num_rows = g.gen_range(0, size);

            let mut tv: TrackVec<A> = TrackVec::new();
            for col in (0..num_cols).map(|_| A::arbitrary(g)) {
                tv.push_col(col);
            }
            for row in (0..num_rows).map(|_| A::arbitrary(g)) {
                tv.push_row(row);
            }
            tv
        }

        fn shrink(&self) -> Box<Iterator<Item=TrackVec<A>>> {
            struct TrackVecShrinker<A: Arbitrary> {
                source: TrackVec<A>,
                index: usize
            }

            impl<A: Arbitrary> Iterator for TrackVecShrinker<A> {
                type Item = TrackVec<A>;

                /// This is a three-cycle shrink: first, shrink the columns and leave rows untouched. Then, shrink
                /// the rows and leave columns untouched. Finally, shrink both rows and columns.
                fn next(&mut self) -> Option<TrackVec<A>> {
                    if self.source.num_cols > 0 || self.source.num_rows > 0 {
                        self.index += 1;
                        match (self.index - 1) % 3 {
                            0 => {
                                if let Some(col_shrunk) = self.source.col_range(..).unwrap().to_vec().shrink().next() {
                                    let mut new_vec = self.source.clone();
                                    new_vec.clear_cols();
                                    for col in col_shrunk {
                                        new_vec.push_col(col);
                                    }
                                    Some(new_vec)
                                } else {
                                    self.next()
                                }
                            }

                            1 => {
                                if let Some(row_shrunk) = self.source.row_range(..).unwrap().to_vec().shrink().next() {
                                    let mut new_vec = self.source.clone();
                                    new_vec.clear_rows();
                                    for row in row_shrunk {
                                        new_vec.push_row(row);
                                    }
                                    Some(new_vec)
                                } else {
                                    self.next()
                                }
                            }

                            2 => {
                                if let Some(col_shrunk) = self.source.col_range(..).unwrap().to_vec().shrink().next() {
                                if let Some(row_shrunk) = self.source.row_range(..).unwrap().to_vec().shrink().next() {
                                    let mut new_vec = TrackVec::new();

                                    for col in col_shrunk {
                                        new_vec.push_col(col);
                                    }
                                    for row in row_shrunk {
                                        new_vec.push_row(row);
                                    }
                                    self.source = new_vec.clone();
                                    return Some(new_vec);
                                }}
                                None
                            }
                            _ => unreachable!()
                        }
                    } else {
                        None
                    }
                }
            }

            Box::new(TrackVecShrinker {
                source: self.clone(),
                index: 0
            })
        }
    }
}
