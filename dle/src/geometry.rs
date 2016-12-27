use super::Px;
use std::cmp;
use std::ops::{Add, AddAssign, BitOr};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: Px,
    pub y: Px
}

impl Point {
    pub fn new(x: Px, y: Px) -> Point {
        Point {
            x: x,
            y: y
        }
    }

    pub fn min() -> Point {
        Point::new(0, 0)
    }

    pub fn max() -> Point {
        Point::new(Px::max_value(), Px::max_value())
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
    fn width(self) -> Px;
    fn height(self) -> Px;
    fn offset(self, offset: Point) -> OffsetRect;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetRect {
    pub topleft: Point,
    pub lowright: Point
}

impl OffsetRect {
    pub fn new(tl_x: Px, tl_y: Px, lr_x: Px, lr_y: Px) -> OffsetRect {
        OffsetRect {
            topleft: Point::new(tl_x, tl_y),
            lowright: Point::new(lr_x, lr_y)
        }
    }
}

impl Rect for OffsetRect {
    fn width(self) -> Px {
        self.lowright.x - self.topleft.x
    }

    fn height(self) -> Px {
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
    pub fn new(lr_x: Px, lr_y: Px) -> OriginRect {
        OriginRect {
            lowright: Point::new(lr_x, lr_y)
        }
    }

    pub fn min() -> OriginRect {
        OriginRect {
            lowright: Point::min()
        }
    }

    pub fn max() -> OriginRect {
        OriginRect {
            lowright: Point::max()
        }
    }
}

impl Rect for OriginRect {
    fn width(self) -> Px {
        self.lowright.x
    }

    fn height(self) -> Px {
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
