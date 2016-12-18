use super::geometry::{OriginRect, OffsetRect, Rect, Point};
use super::layout::{NodeSpan, GridSize};
use std::ops::Range;
use std::cmp::Ordering;

pub struct SetLineStatus {
    pub size_px: Ordering,
    pub min_size_px: Ordering
}

pub struct SetCellStatus {
    pub col_status: SetLineStatus,
    pub row_status: SetLineStatus
}

#[derive(Clone, Copy)]
struct GridLine {
    /// The size, in pixels, of this grid line. For columns, this is the width; for rows, the height.
    size_px: u32,
    num_biggest: u32
}

impl Default for GridLine {
    fn default() -> GridLine {
        GridLine {
            size_px: 0,
            num_biggest: 0
        }
    }
}

#[derive(Default, Clone)]
pub struct GridDims {
    num_cols: u32,
    num_rows: u32,
    /// A vector that contains the dimensions of the rows and columns of the grid. The first `num_cols`
    /// elements are the column widths, the next `num_rows` elements are the row heights.
    dims: Vec<GridLine>,
    size_px: OriginRect
}

impl GridDims {
    pub fn new() -> GridDims {
        GridDims {
            num_cols: 0,
            num_rows: 0,
            dims: Vec::new(),
            size_px: OriginRect::default()
        }
    }

    pub fn with_size(grid_size: GridSize, size_px: OriginRect) -> GridDims {
        let mut dims = GridDims::new();
        dims.set_grid_size(grid_size);
        dims.set_size_px(size_px).ok();
        dims
    }

    pub fn with_grid_size(grid_size: GridSize) -> GridDims {
        let mut dims = GridDims::new();
        dims.set_grid_size(grid_size);
        dims
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
                    self.dims.resize((size.x + size.y) as usize, GridLine::default());
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
        // We can just call `get`, unlike in `column_width`, because the rows are stored at the end
        // of the vector.
        self.get_row(row_num).map(|gl| gl.size_px)
    }

    pub fn set_col_width(&mut self, column_num: u32, new_width: u32, old_width: u32) -> SetLineStatus {
        let mut width_delta = 0i32;
        {        
            let col = self.get_col_mut(column_num).expect("Invalid column number passed");

            let was_biggest_width = col.size_px == old_width;
            let is_biggest_width = col.size_px <= new_width;

            col.num_biggest += is_biggest_width as u32;
            col.num_biggest -= was_biggest_width as u32;

            if col.size_px < new_width {
                width_delta -= col.size_px as i32;
                width_delta += new_width as i32;
                
                col.size_px = new_width;
                col.num_biggest = 1;
            } else if col.num_biggest == 0 {
                width_delta -= col.size_px as i32;
                
                col.size_px = 0;
                col.num_biggest = 0;
            }
        }

        self.size_px.lowright.x = (self.size_px.lowright.x as i32 + width_delta) as u32;

        SetLineStatus {
            size_px: width_delta.cmp(&0),
            min_size_px: Ordering::Equal
        }
    }

    pub fn set_row_height(&mut self, row_num: u32, new_height: u32, old_height: u32) -> SetLineStatus {
        let mut height_delta = 0i32;
        {        
            let row = self.get_row_mut(row_num).expect("Invalid row number passed");

            let was_biggest_height = row.size_px == old_height;
            let is_biggest_height = row.size_px <= new_height;

            row.num_biggest += is_biggest_height as u32;
            row.num_biggest -= was_biggest_height as u32;

            if row.size_px < new_height {
                height_delta -= row.size_px as i32;
                height_delta += new_height as i32;
                
                row.size_px = new_height;
                row.num_biggest = 1;
            } else if row.num_biggest == 0 {
                height_delta -= row.size_px as i32;
                
                row.size_px = 0;
                row.num_biggest = 0;
            }
        }

        self.size_px.lowright.y = (self.size_px.lowright.y as i32 + height_delta) as u32;

        SetLineStatus {
            size_px: height_delta.cmp(&0),
            min_size_px: Ordering::Equal
        }
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

    fn get_col(&self, column_num: u32) -> Option<&GridLine> {
        if column_num < self.num_cols {
            self.dims.get(column_num as usize)
        } else {
            None
        }
    }

    fn get_row(&self, row_num: u32) -> Option<&GridLine> {
        self.dims.get((self.num_cols + row_num) as usize)
    }

    fn get_cell_lines_mut(&mut self, column_num: u32, row_num: u32) -> (Option<&mut GridLine>, Option<&mut GridLine>) {
        if self.num_cols <= column_num {
            (None, self.get_row_mut(row_num))
        } else {
            let (cols, rows) = self.dims.split_at_mut(self.num_cols as usize);
            (cols.get_mut(column_num as usize), rows.get_mut(row_num as usize))
        }
    }

    fn get_col_mut(&mut self, column_num: u32) -> Option<&mut GridLine> {
        if column_num < self.num_cols {
            self.dims.get_mut(column_num as usize)
        } else {
            None
        }
    }

    fn get_row_mut(&mut self, row_num: u32) -> Option<&mut GridLine> {
        self.dims.get_mut((self.num_cols + row_num) as usize)
    }

    fn col_iter<'a>(&'a self, range: Range<u32>) -> Option<impl Iterator<Item = &'a GridLine>> {
        if range.end <= self.num_cols {
            let range_usize = range.start as usize..range.end as usize;
            Some(self.dims[range_usize].iter())
        } else {
            None
        }
    }

    fn row_iter<'a>(&'a self, range: Range<u32>) -> Option<impl Iterator<Item = &'a GridLine>> {
        if range.end <= self.dims.len() as u32 {
            let range_usize = (range.start + self.num_cols) as usize..(range.end + self.num_cols)as usize;
            Some(self.dims[range_usize].iter())
        } else {
            None
        }
    }

    fn col_iter_mut<'a>(&'a mut self, range: Range<u32>) -> Option<impl Iterator<Item = &'a mut GridLine>> {
        if range.end <= self.num_cols {
            let range_usize = range.start as usize..range.end as usize;
            Some(self.dims[range_usize].iter_mut())
        } else {
            None
        }
    }

    fn row_iter_mut<'a>(&'a mut self, range: Range<u32>) -> Option<impl Iterator<Item = &'a mut GridLine>> {
        if range.end <= self.dims.len() as u32 {
            let range_usize = (range.start + self.num_cols) as usize..(range.end + self.num_cols)as usize;
            Some(self.dims[range_usize].iter_mut())
        } else {
            None
        }
    }
}
