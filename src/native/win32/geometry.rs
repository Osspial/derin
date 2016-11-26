use std::ops::{Add, AddAssign, BitOr, Range};
use std::cmp;
use std::os::raw::c_int;

use ui::layout::{Place, PlaceInCell, GridSize, NodeSpan};

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

impl BitOr for OffsetRect {
    type Output = OffsetRect;
    /// "Or"s the two rectangles together, creating a new rectangle that covers the areas of both
    /// rects.
    fn bitor(self, rhs: OffsetRect) -> OffsetRect {
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
            Ordering::Less => self.largest_cells.start == cell_num || cell_num == self.largest_cells.end - 1
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
        dims.expand_size_px(size_px);
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

    pub fn expand_size_px(&mut self, size_px: OriginRect) {
        assert!(self.size_px.width() <= size_px.width());
        assert!(self.size_px.height() <= size_px.height());

        let old_rect = self.get_span_origin_rect(NodeSpan::new(.., ..)).unwrap();
        let width_diff = (size_px.width() - old_rect.width()) as usize;
        let height_diff = (size_px.height() - old_rect.height()) as usize;

        let num_cols = self.num_cols as usize;
        let num_rows = self.num_rows as usize;

        let width_add = (width_diff / num_cols) as c_int;
        let height_add = (height_diff / num_rows) as c_int;

        for col in &mut self.dims[0..width_diff % num_cols] {
            col.size_px += width_add + 1;
        }
        for col in &mut self.dims[width_diff % num_cols..num_cols] {
            col.size_px += width_add;
        }

        for row in &mut self.dims[num_cols..num_cols + (height_diff % num_rows)] {
            row.size_px += height_add + 1;
        }
        for row in &mut self.dims[num_cols + (height_diff % num_rows)..] {
            row.size_px += height_add;
        }

        self.size_px = size_px;
    }

    pub fn zero_column(&mut self, column_num: u32) {
        self.size_px.lowright.x -= self.dims[column_num as usize].size_px;
        self.dims[column_num as usize] = GridLine::default()
    }

    pub fn zero_row(&mut self, row_num: u32) {
        self.size_px.lowright.y -= self.dims[(self.num_cols + row_num) as usize].size_px;
        self.dims[(self.num_cols + row_num) as usize] = GridLine::default()
    }

    pub fn zero_all(&mut self) {
        for d in &mut self.dims {
            *d = GridLine::default();
        }
        self.size_px = OriginRect::default();
    }

    pub fn column_width(&self, column_num: u32) -> Option<c_int> {
        match column_num < self.num_cols {
            true => Some(self.dims[column_num as usize].size_px),
            false => None
        }
    }

    pub fn row_height(&self, row_num: u32) -> Option<c_int> {
        // We can just call `get`, unlike in `column_width`, because the rows are stored at the end
        // of the vector.
        self.dims.get((row_num + self.num_cols) as usize).map(|gl| gl.size_px)
    }

    pub fn expand_cell_rect(&mut self, column_num: u32, row_num: u32, rect: OriginRect) -> bool {
        assert!(column_num < self.num_cols);
        assert!(row_num < self.num_rows);

        // Remove the width and height of the cell from before the expansion from the master pixel size.
        self.size_px.lowright.x -= self.dims[column_num as usize].size_px;
        self.size_px.lowright.y -= self.dims[(self.num_cols + row_num) as usize].size_px;

        // A bitwise or is used here because it doesn't short-circuit, and both of these functions
        // have side effects that need to occur.
        let ret = self.dims[column_num as usize].expand_line_size(row_num, rect.width()) |
        self.dims[(self.num_cols + row_num) as usize].expand_line_size(column_num, rect.height());

        // Add the width of the height of the cell after the expansion back to the master pixel size. This
        // ensures that it's accurate.
        self.size_px.lowright.x += self.dims[column_num as usize].size_px;
        self.size_px.lowright.y += self.dims[(self.num_cols + row_num) as usize].size_px;

        ret
    }

    pub fn get_cell_offset(&self, column_num: u32, row_num: u32) -> Option<Point> {
        // This process could probably be sped up with Rayon. Something to come back to.
        if column_num < self.num_cols &&
           row_num < self.num_rows
        {
            Some(Point::new(
                (0..column_num).map(|c| self.dims[c as usize].size_px).sum(),
                (0..row_num).map(|r| self.dims[(r + self.num_cols) as usize].size_px).sum()
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
                col_range.map(|c| self.dims[c as usize].size_px).sum(),
                row_range.map(|r| self.dims[(r + self.num_cols) as usize].size_px).sum()
            ))
        } else {
            None
        }
    }

    pub fn width(&self) -> c_int {
        self.size_px.width()
    }

    pub fn height(&self) -> c_int {
        self.size_px.height()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HintedCell {
    /// The rectangle that contains the entire cell. The inner rect may be smaller than this, but it
    /// cannot be larger than this.
    outer_rect: OffsetRect,
    place_in_or: PlaceInCell,

    /// The actual rectangle of the element the cell contains. Is an `Option`, as this rectangle doesn't
    /// exist when the cell is created.
    inner_rect: Option<OffsetRect>
}

impl HintedCell {
    pub fn new(outer_rect: OffsetRect, place_in_or: PlaceInCell) -> HintedCell {
        HintedCell {
            outer_rect: outer_rect,
            place_in_or: place_in_or,

            inner_rect: None
        }
    }

    pub fn transform_min_rect(&mut self, minrect: OriginRect) -> OffsetRect {
        macro_rules! place_on_axis {
            ($axis:ident $minrect_size_axis:expr => $inner_rect:ident) => {
                match self.place_in_or.x {
                    Place::Stretch => {
                        $inner_rect.topleft.$axis = self.outer_rect.topleft.$axis;
                        $inner_rect.lowright.$axis = self.outer_rect.lowright.$axis;
                    },
                    Place::Start => {
                        $inner_rect.topleft.$axis = self.outer_rect.topleft.$axis;
                        $inner_rect.lowright.$axis = self.outer_rect.topleft.$axis + $minrect_size_axis;
                    },
                    Place::End => {
                        $inner_rect.lowright.$axis = self.outer_rect.lowright.$axis;
                        $inner_rect.topleft.$axis = self.outer_rect.lowright.$axis - $minrect_size_axis;
                    },
                    Place::Center => {
                        let center = (self.outer_rect.topleft.$axis + self.outer_rect.lowright.$axis) / 2;
                        $inner_rect.topleft.$axis = center - $minrect_size_axis / 2;
                        $inner_rect.lowright.$axis = center + $minrect_size_axis / 2;
                    }
                }
            }
        }

        self.outer_rect = self.outer_rect | minrect.offset(self.outer_rect.topleft);

        let mut inner_rect = OffsetRect::default();
        place_on_axis!(x minrect.width() => inner_rect);
        place_on_axis!(y minrect.height() => inner_rect);
        self.inner_rect = Some(inner_rect);
        
        inner_rect
    }

    pub fn inner_rect(&self) -> Option<OffsetRect> {
        self.inner_rect
    }

    pub fn outer_rect(&self) -> OffsetRect {
        self.outer_rect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::GridLine;
    use ui::layout::{Place, PlaceInCell, GridSize};

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

        let mut gd = GridDims::with_size(GridSize::new(2, 2), OriginRect::default());

        // Test insertion of a cell that isn't the base cell
        gd.expand_cell_rect(1, 1, OriginRect::new(16, 16));
        // We aren't using assert_2x2_grid here because the cell at (0, 0) hasn't been filled in yet.
        assert_eq!(Some(Point::new(0, 0)), gd.get_cell_offset(1, 1));
        assert_eq!(Some(OriginRect::new(16, 16)), gd.get_cell_origin_rect(1, 1));
        assert_eq!(Some(OffsetRect::new(0, 0, 16, 16)), gd.get_cell_rect(1, 1));

        // Test insertion of a cell at (0, 0)
        gd.expand_cell_rect(0, 0, OriginRect::new(16, 32));
        assert_2x2_grid(&gd);

        // Resize the grid to contain space for the new cell
        gd.set_grid_size(GridSize::new(4, 2));

        // Test insertion of a cell offset from the diagonal centerline of the grid. Notice how, because
        // it smaller on the y axis than the cell already in that row, it gets rescaled...
        gd.expand_cell_rect(3, 0, OriginRect::new(8, 8));
        assert_eq!(Some(Point::new(32, 0)), gd.get_cell_offset(3, 0));
        assert_eq!(Some(OriginRect::new(8, 32)), gd.get_cell_origin_rect(3, 0));
        assert_eq!(Some(OffsetRect::new(32, 0, 40, 32)), gd.get_cell_rect(3, 0));
        // ...and make sure that the data of the cells at (0, 0) and (1, 1) are unaffected
        assert_2x2_grid(&gd);

        // Downsize the grid again, cutting off the new cell.
        gd.set_grid_size(GridSize::new(2, 2));
        assert_eq!(None, gd.get_cell_rect(3, 0));
        // Make sure the 2x2 grid is AOK.
        assert_2x2_grid(&gd);
    }

    #[test]
    fn test_cell_hints() {
        let mr = OffsetRect::new(16, 16, 32, 32);

        {
            let mut hc = HintedCell::new(mr, PlaceInCell::new(Place::Stretch, Place::Stretch));
            assert_eq!(mr, hc.transform_min_rect(OriginRect::new(8, 8)));
            assert_eq!(mr, hc.outer_rect());

            hc.transform_min_rect(OriginRect::new(64, 64));
            assert_eq!(OffsetRect::new(16, 16, 80, 80), hc.outer_rect());
        }

        {
            let mut hc = HintedCell::new(mr, PlaceInCell::new(Place::Start, Place::Start));
            assert_eq!(OffsetRect::new(16, 16, 24, 24), hc.transform_min_rect(OriginRect::new(8, 8)));
            assert_eq!(mr, hc.outer_rect());

            hc.transform_min_rect(OriginRect::new(64, 64));
            assert_eq!(OffsetRect::new(16, 16, 80, 80), hc.outer_rect());
        }

        {
            let mut hc = HintedCell::new(mr, PlaceInCell::new(Place::End, Place::End));
            assert_eq!(OffsetRect::new(24, 24, 32, 32), hc.transform_min_rect(OriginRect::new(8, 8)));
            assert_eq!(mr, hc.outer_rect());

            hc.transform_min_rect(OriginRect::new(64, 64));
            assert_eq!(OffsetRect::new(16, 16, 80, 80), hc.outer_rect());
        }

        {
            let mut hc = HintedCell::new(mr, PlaceInCell::new(Place::Center, Place::Center));
            assert_eq!(OffsetRect::new(20, 20, 28, 28), hc.transform_min_rect(OriginRect::new(8, 8)));
            assert_eq!(mr, hc.outer_rect());

            hc.transform_min_rect(OriginRect::new(64, 64));
            assert_eq!(OffsetRect::new(16, 16, 80, 80), hc.outer_rect());
        }
    }
}
