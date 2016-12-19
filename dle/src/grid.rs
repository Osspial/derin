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
struct GridTrack {
    /// The size, in pixels, of this grid track. For columns, this is the width; for rows, the height.
    size_px: u32,
    num_biggest: u32,
    min_size_px: u32,
    min_num_biggest: u32
}

impl GridTrack {
    fn set_cell_size_px(&mut self, mut new_size: u32, old_size: u32) -> SetSizeResult {
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

    fn set_cell_min_size_px(&mut self, new_min_size: u32, old_min_size: u32) -> SetMinSizeResult {
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

impl Default for GridTrack {
    fn default() -> GridTrack {
        GridTrack {
            size_px: 0,
            num_biggest: 0,
            min_size_px: 0,
            min_num_biggest: 0
        }
    }
}

macro_rules! fn_set_cell_size_px {
    () => ();
    (
        $(#[$attr:meta])*
        pub fn $fn_name:ident(&mut self, $num:ident: u32, $new_cell_size:ident: u32, $old_cell_size:ident: u32) -> $ret:ty
            where get = self.$get:ident,
                  for<$cell:ident> set = $set:expr,
                  acc = self.$acc_field:ident.lowright.$acc_dim:ident,
        $($rest:tt)*
    ) => {
        $(#[$attr])*
        pub fn $fn_name(&mut self, $num: u32, $new_cell_size: u32, $old_cell_size: u32) -> $ret {
            let old_size: u32;
            let new_size: u32;
            let ret: $ret;
            {
                let $cell = self.$get($num).expect(concat!("Invalid ", stringify!($cell), " number passed"));

                old_size = $cell.size_px;
                ret = $set;
                new_size = $cell.size_px;
            }

            self.$acc_field.lowright.$acc_dim += old_size;
            self.$acc_field.lowright.$acc_dim -= new_size;

            ret
        }

        fn_set_cell_size_px!($($rest)*);
    }
}

#[derive(Default, Clone)]
pub struct GridDims {
    num_cols: u32,
    num_rows: u32,
    /// A vector that contains the dimensions of the rows and columns of the grid. The first `num_cols`
    /// elements are the column widths, the next `num_rows` elements are the row heights.
    dims: Vec<GridTrack>,
    size_px: OriginRect,
    min_size_px: OriginRect
}

impl GridDims {
    pub fn new() -> GridDims {
        GridDims {
            num_cols: 0,
            num_rows: 0,
            dims: Vec::new(),
            size_px: OriginRect::default(),
            min_size_px: OriginRect::default()
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
                    self.dims.resize((size.x + size.y) as usize, GridTrack::default());
                } else if size.x + size.y < self.num_cols + self.num_rows {
                    self.size_px.lowright.x -= self.dims[size.x as usize..self.num_cols as usize].iter().map(|d| d.size_px).sum();
                    self.size_px.lowright.y -= self.dims[(self.num_cols + size.y) as usize..].iter().map(|d| d.size_px).sum();
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

    pub fn set_size_px(&mut self, size_px: OriginRect) -> Result<(), OriginRect> {
        // TODO: HANDLE MIN SIZE
        let width_delta = size_px.width() as i32 - self.size_px.width() as i32;
        let height_delta = size_px.height() as i32 - self.size_px.height() as i32;

        let width_add = width_delta / self.num_cols as i32;
        let height_add = height_delta / self.num_rows as i32;

        // Because we're dealing with integers, most sizes passed through `size_px` aren't going to be
        // divided cleanly by `num_cols` and `num_rows`. To compensate for that, we take the remainder of
        // the `width_add` and `height_add` operations, and for the interval from zero to that remainder
        // the sign number (-1 or 1) is added in addition to `width_add` and `height_add`.
        let col_add_po = width_delta.abs() as u32 % self.num_cols;
        let row_add_po = height_delta.abs() as u32 % self.num_rows;

        let num_cols = self.num_cols;
        for col in self.col_iter_mut(0..col_add_po).unwrap() {
            col.size_px = (col.size_px as i32 + width_add + width_add.signum()) as u32;
        }
        for col in self.col_iter_mut(col_add_po..num_cols).unwrap() {
            col.size_px = (col.size_px as i32 + width_add) as u32;
        }

        let num_rows = self.num_rows;
        for row in self.row_iter_mut(0..row_add_po).unwrap() {
            row.size_px = (row.size_px as i32 + height_add + height_add.signum()) as u32;
        }
        for row in self.row_iter_mut(row_add_po..num_rows).unwrap() {
            row.size_px = (row.size_px as i32 + height_add) as u32;
        }

        Ok(())
    }

    pub fn column_width(&self, column_num: u32) -> Option<u32> {
        self.get_col(column_num).map(|gl| gl.size_px)
    }

    pub fn row_height(&self, row_num: u32) -> Option<u32> {
        self.get_row(row_num).map(|gl| gl.size_px)
    }

    fn_set_cell_size_px!{
        /// Given a column num, set the width of a single cell in the column. Note that this does *not*
        /// necessarily set the actual width of the column, but instead takes into account the widths of
        /// other cells in the column to determine whether or not to downscale the column's width, upscale
        /// it, or leave it unchanged.
        pub fn set_col_cell_width(&mut self, column_num: u32, new_cell_width: u32, old_cell_width: u32) -> SetSizeResult
            where get = self.get_col_mut,
                  for<col> set = col.set_cell_size_px(new_cell_width, old_cell_width),
                  acc = self.size_px.lowright.x,

        /// See `set_col_cell_width`'s documentation. The concept is the same, except for rows and not columns.
        pub fn set_row_cell_height(&mut self, row_num: u32, new_cell_height: u32, old_cell_height: u32) -> SetSizeResult
            where get = self.get_row_mut,
                  for<row> set = row.set_cell_size_px(new_cell_height, old_cell_height),
                  acc = self.size_px.lowright.y,


        /// Given a column num, set the minimum width of a single cell in the column. 
        pub fn set_col_cell_min_width(&mut self, column_num: u32, new_min_width: u32, old_min_width: u32) -> SetMinSizeResult
            where get = self.get_col_mut,
                  for<col> set = col.set_cell_min_size_px(new_min_width, old_min_width),
                  acc = self.min_size_px.lowright.x,

        pub fn set_row_cell_min_width(&mut self, row_num: u32, new_min_height: u32, old_min_height: u32) -> SetMinSizeResult
            where get = self.get_row_mut,
                  for<row> set = row.set_cell_min_size_px(new_min_height, old_min_height),
                  acc = self.min_size_px.lowright.y,
    }

    pub fn get_cell_offset(&self, column_num: u32, row_num: u32) -> Option<Point> {
        // This process could probably be sped up with Rayon. Something to come back to.
        if column_num < self.num_cols &&
           row_num < self.num_rows
        {
            Some(Point::new(
                (0..column_num).map(|c| self.get_col(c).unwrap().size_px).sum(),
                (0..row_num).map(|r| self.get_row(r).unwrap().size_px).sum()
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
                self.col_iter(col_range).unwrap().map(|l| l.size_px).sum(),
                self.row_iter(row_range).unwrap().map(|l| l.size_px).sum()
            ))
        } else {
            None
        }
    }

    pub fn width(&self) -> u32 {
        self.size_px.width()
    }

    pub fn height(&self) -> u32 {
        self.size_px.height()
    }

    fn get_col(&self, column_num: u32) -> Option<&GridTrack> {
        if column_num < self.num_cols {
            self.dims.get(column_num as usize)
        } else {
            None
        }
    }

    fn get_row(&self, row_num: u32) -> Option<&GridTrack> {
        self.dims.get((self.num_cols + row_num) as usize)
    }

    fn get_cell_tracks_mut(&mut self, column_num: u32, row_num: u32) -> (Option<&mut GridTrack>, Option<&mut GridTrack>) {
        if self.num_cols <= column_num {
            (None, self.get_row_mut(row_num))
        } else {
            let (cols, rows) = self.dims.split_at_mut(self.num_cols as usize);
            (cols.get_mut(column_num as usize), rows.get_mut(row_num as usize))
        }
    }

    fn get_col_mut(&mut self, column_num: u32) -> Option<&mut GridTrack> {
        if column_num < self.num_cols {
            self.dims.get_mut(column_num as usize)
        } else {
            None
        }
    }

    fn get_row_mut(&mut self, row_num: u32) -> Option<&mut GridTrack> {
        self.dims.get_mut((self.num_cols + row_num) as usize)
    }

    fn col_iter<'a>(&'a self, range: Range<u32>) -> Option<impl Iterator<Item = &'a GridTrack>> {
        if range.end <= self.num_cols {
            let range_usize = range.start as usize..range.end as usize;
            Some(self.dims[range_usize].iter())
        } else {
            None
        }
    }

    fn row_iter<'a>(&'a self, range: Range<u32>) -> Option<impl Iterator<Item = &'a GridTrack>> {
        if range.end <= self.dims.len() as u32 {
            let range_usize = (range.start + self.num_cols) as usize..(range.end + self.num_cols)as usize;
            Some(self.dims[range_usize].iter())
        } else {
            None
        }
    }

    fn col_iter_mut<'a>(&'a mut self, range: Range<u32>) -> Option<impl Iterator<Item = &'a mut GridTrack>> {
        if range.end <= self.num_cols {
            let range_usize = range.start as usize..range.end as usize;
            Some(self.dims[range_usize].iter_mut())
        } else {
            None
        }
    }

    fn row_iter_mut<'a>(&'a mut self, range: Range<u32>) -> Option<impl Iterator<Item = &'a mut GridTrack>> {
        if range.end <= self.dims.len() as u32 {
            let range_usize = (range.start + self.num_cols) as usize..(range.end + self.num_cols)as usize;
            Some(self.dims[range_usize].iter_mut())
        } else {
            None
        }
    }
}
