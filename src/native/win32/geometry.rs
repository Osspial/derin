use std::ops::{Add, AddAssign, BitAnd, Range};
use std::cmp;
use std::os::raw::c_int;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: c_int,
    pub y: c_int
}

impl Point {
    pub fn new(x: c_int, y: c_int) -> Point {
        Point {
            x: x,
            y: y
        }
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Point) -> Point {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y
        }
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Point) {
        *self = *self + rhs;
    }
}

pub trait Rect {
    fn width(self) -> c_int;
    fn height(self) -> c_int;
    fn offset(self, offset: Point) -> OffsetRect;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetRect {
    pub topleft: Point,
    pub lowright: Point
}

impl OffsetRect {
    pub fn new(tl_x: c_int, tl_y: c_int, lr_x: c_int, lr_y: c_int) -> OffsetRect {
        OffsetRect {
            topleft: Point::new(tl_x, tl_y),
            lowright: Point::new(lr_x, lr_y)
        }
    }
}

impl Rect for OffsetRect {
    fn width(self) -> c_int {
        self.lowright.x - self.topleft.x
    }

    fn height(self) -> c_int {
        self.lowright.y - self.topleft.y
    }

    fn offset(mut self, offset: Point) -> OffsetRect {
        self.topleft += offset;
        self.lowright += offset;
        self
    }
}

impl BitAnd for OffsetRect {
    type Output = OffsetRect;
    /// "And"s the two rectangles together, creating a new rectangle that covers the areas of both
    /// rects.
    fn bitand(self, rhs: OffsetRect) -> OffsetRect {
        OffsetRect::new(
            cmp::min(self.topleft.x, rhs.topleft.x),
            cmp::min(self.topleft.y, rhs.topleft.y),

            cmp::max(self.lowright.x, rhs.lowright.x),
            cmp::max(self.lowright.y, rhs.lowright.y)
        )
    }
}

impl From<OriginRect> for OffsetRect {
    fn from(ogr: OriginRect) -> OffsetRect {
        OffsetRect {
            topleft: Point::new(0, 0),
            lowright: ogr.lowright
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginRect {
    pub lowright: Point
}

impl OriginRect {
    pub fn new(lr_x: c_int, lr_y: c_int) -> OriginRect {
        OriginRect {
            lowright: Point::new(lr_x, lr_y)
        }
    }
}

impl Rect for OriginRect {
    fn width(self) -> c_int {
        self.lowright.x
    }

    fn height(self) -> c_int {
        self.lowright.y
    }

    fn offset(self, offset: Point) -> OffsetRect {
        OffsetRect {
            topleft: offset,
            lowright: self.lowright + offset
        }
    }
}

impl From<OffsetRect> for OriginRect {
    fn from(rect: OffsetRect) -> OriginRect {
        OriginRect {
            lowright: Point::new(rect.width(), rect.height())
        }
    }
}

#[derive(Clone)]
struct GridLine {
    /// The size, in pixels, of this grid line. For columns, this is the width; for rows, the height.
    size_px: c_int,
    /// The largest cell in the grid line. If only one cell is the largest, then the range only occupies
    /// one cell; however, if there are **multiple** largest cells, this specifies a range that contains
    /// all of them.
    largest_cells: Range<u32>
}

impl Default for GridLine {
    fn default() -> GridLine {
        GridLine {
            size_px: 0,
            largest_cells: 0..0
        }
    }
}

impl GridLine {
    /// Modify the size of this grid line, either expanding it or keeping the size the same. If the new
    /// size is identical to the current size, the `largest_cells` range is modified to include the new
    /// cell num. If it's larger, the `largest_cells` range is modified to only include the new range.
    /// If it's smaller, nothing happens.
    ///
    /// Returns `true` if the new line size has a chance of changing the layout of the grid.
    fn expand_line_size(&mut self, cell_num: u32, new_size: c_int) -> bool {
        use std::cmp::Ordering;

        match new_size.cmp(&self.size_px) {
            Ordering::Greater => {
                self.size_px = new_size;
                self.largest_cells = cell_num..cell_num + 1;
                true
            },
            Ordering::Equal => {
                self.size_px = new_size;
                self.largest_cells.start = cmp::min(cell_num, self.largest_cells.start);
                self.largest_cells.end = cmp::max(cell_num + 1, self.largest_cells.end);
                false
            },
            Ordering::Less => self.largest_cells.start <= cell_num && cell_num < self.largest_cells.end
        }
    }
}

#[derive(Default, Clone)]
pub struct GridDims {
    num_cols: u32,
    num_rows: u32,
    /// A vector that contains the dimensions of the rows and columns of the grid. The first `num_cols`
    /// elements are the column widths, the next `num_rows` elements are the row heights.
    dims: Vec<GridLine>
}

impl GridDims {
    pub fn new() -> GridDims {
        GridDims {
            num_cols: 0,
            num_rows: 0,
            dims: Vec::new()
        }
    }

    pub fn with_dims(num_cols: u32, num_rows: u32) -> GridDims {
        GridDims {
            num_cols: num_cols,
            num_rows: num_rows,
            dims: vec![GridLine::default(); (num_cols + num_rows) as usize]
        }
    }

    pub fn set_dims(&mut self, num_cols: u32, num_rows: u32) {
        use std::ptr;

        unsafe {
            // If the new length of the vector is going to be greater than the current length of the vector,
            // extend it before doing any calculations. Why not resize if the vector is going to be shorter?
            // Well, we need to shift the row data over, so if we resize the vector before doing that we're
            // going to be shifting from undefined data!
            if num_cols + num_rows > self.num_cols + self.num_rows {
                self.dims.resize((num_cols + num_rows) as usize, GridLine::default());
            }

            // Shift the row data over, if it actually needs shifting.
            if num_cols != self.num_cols {
                ptr::copy(&self.dims[self.num_cols as usize], &mut self.dims[num_cols as usize], self.num_rows as usize);
            }
            // If we shifted the row data to the right, fill the new empty space with zeroes. In the event that
            // it was shifted to the left or not shifted at all, nothing is done due to the saturating subtraction.
            ptr::write_bytes(&mut self.dims[self.num_cols as usize], 0, num_cols.saturating_sub(self.num_cols) as usize);
            
            self.num_cols = num_cols;
            self.num_rows = num_rows;

            // Finally, set the length of the vector to be correct. This would have been done already if the
            // grid's size was expanded, but if it was decreased we need to do it here.
            self.dims.set_len((num_cols + num_rows) as usize);
        }
    }

    pub fn zero_column(&mut self, column_num: u32) {
        self.dims[column_num as usize] = GridLine::default()
    }

    pub fn zero_row(&mut self, row_num: u32) {
        self.dims[row_num as usize] = GridLine::default()
    }

    pub fn column_width(&self, column_num: u32) -> Option<c_int> {
        match column_num < self.num_cols {
            true => Some(self.dims[column_num as usize].size_px),
            false => None
        }
    }

    pub fn row_height(&self, row_num: u32) -> Option<c_int> {
        match row_num < self.num_rows {
            true => Some(self.dims[(row_num + self.num_cols) as usize].size_px),
            false => None
        }
    }

    pub fn set_cell(&mut self, column_num: u32, row_num: u32, rect: OriginRect) -> bool {
        assert!(column_num < self.num_cols);
        assert!(row_num < self.num_rows);

        self.num_cols = cmp::max(self.num_cols, column_num + 1);
        self.num_rows = cmp::max(self.num_rows, row_num + 1);
        
        // A bitwise or is used here because it doesn't short-circuit, and both of these functions
        // have side effects that need to occur.
        self.dims[column_num as usize].expand_line_size(row_num, rect.width()) |
        self.dims[(self.num_cols + row_num) as usize].expand_line_size(column_num, rect.height())
    }

    pub fn get_cell_offset(&self, column_num: u32, row_num: u32) -> Option<Point> {
        let mut offset = Point::default();

        if column_num < self.num_cols &&
           row_num < self.num_rows {
            // Sum up the x offset by adding together the widths of all the columns
            for c in 0..column_num {
                offset.x += self.dims[c as usize].size_px;
            }
            // Sum up the y offset by adding together the heights of all the rows
            for r in 0..row_num {
                offset.y += self.dims[(r + self.num_cols) as usize].size_px;
            }

            Some(offset)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::GridLine;

    #[test]
    fn test_expand_line_size() {
        let mut gl = GridLine::default();

        // Add a single cell to the `GridLine`.
        gl.expand_line_size(1, 32);
        assert_eq!(gl.size_px, 32);
        assert_eq!(gl.largest_cells, 1..2);

        // Add another cell to the `GridLine` that is just as large as the first cell
        gl.expand_line_size(5, 32);
        assert_eq!(gl.size_px, 32);
        assert_eq!(gl.largest_cells, 1..6);
    }

    #[test]
    fn test_grid_dims() {
        fn assert_2x2_grid(gd: &GridDims) {
            // Test the cell at (0, 0)
            assert_eq!(Some(Point::new(0, 0)), gd.get_cell_offset(0, 0));
            assert_eq!(Some(OriginRect::new(16, 32)), gd.get_cell_origin_rect(0, 0));
            assert_eq!(Some(OffsetRect::new(0, 0, 16, 32)), gd.get_cell_rect(0, 0));
            // Test the cell at (1, 1)
            assert_eq!(Some(Point::new(16, 32)), gd.get_cell_offset(1, 1));
            assert_eq!(Some(OriginRect::new(16, 16)), gd.get_cell_origin_rect(1, 1));
            assert_eq!(Some(OffsetRect::new(16, 32, 32, 48)), gd.get_cell_rect(1, 1));
        }

        let mut gd = GridDims::with_dims(2, 2);

        // Test insertion of a cell that isn't the base cell
        gd.set_cell(1, 1, OriginRect::new(16, 16));
        // We aren't using assert_2x2_grid here because the cell at (0, 0) hasn't been filled in yet.
        assert_eq!(Some(Point::new(0, 0)), gd.get_cell_offset(1, 1));
        assert_eq!(Some(OriginRect::new(16, 16)), gd.get_cell_origin_rect(1, 1));
        assert_eq!(Some(OffsetRect::new(0, 0, 16, 16)), gd.get_cell_rect(1, 1));

        // Test insertion of a cell at (0, 0)
        gd.set_cell(0, 0, OriginRect::new(16, 32));
        assert_2x2_grid(&gd);

        // Resize the grid to contain space for the new cell
        gd.set_dims(4, 2);

        // Test insertion of a cell offset from the diagonal centerline of the grid. Notice how, because
        // it smaller on the y axis than the cell already in that row, it gets rescaled...
        gd.set_cell(3, 0, OriginRect::new(8, 8));
        assert_eq!(Some(Point::new(32, 0)), gd.get_cell_offset(3, 0));
        assert_eq!(Some(OriginRect::new(8, 32)), gd.get_cell_origin_rect(3, 0));
        assert_eq!(Some(OffsetRect::new(32, 0, 40, 32)), gd.get_cell_rect(3, 0));
        // ...and make sure that the data of the cells at (0, 0) and (1, 1) are unaffected
        assert_2x2_grid(&gd);

        // Downsize the grid again, cutting off the new cell.
        gd.set_dims(2, 2);
        assert_eq!(None, gd.get_cell_rect(3, 0));
        // Make sure the 2x2 grid is AOK.
        assert_2x2_grid(&gd);
    }
}
