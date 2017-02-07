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

pub trait Rect: From<OriginRect> + From<OffsetRect> + Copy {
    fn topleft(self) -> Point;
    fn lowright(self) -> Point;

    fn width(self) -> Px {
        self.lowright().x - self.topleft().x
    }

    fn height(self) -> Px {
        self.lowright().y - self.topleft().y
    }

    fn offset(self, offset: Point) -> OffsetRect {
        OffsetRect {
            topleft: self.topleft() + offset,
            lowright: self.lowright() + offset
        }
    }
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
    #[inline]
    fn topleft(self) -> Point {
        self.topleft
    }

    #[inline]
    fn lowright(self) -> Point {
        self.lowright
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
            lowright: ogr.lowright()
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginRect {
    pub width: Px,
    pub height: Px
}

impl OriginRect {
    pub fn new(width: Px, height: Px) -> OriginRect {
        OriginRect {
            width: width,
            height: height
        }
    }

    pub fn min() -> OriginRect {
        OriginRect {
            width: 0,
            height: 0
        }
    }

    pub fn max() -> OriginRect {
        OriginRect {
            width: Px::max_value(),
            height: Px::max_value()
        }
    }
}

impl Rect for OriginRect {
    #[inline]
    fn topleft(self) -> Point {
        Point::new(0, 0)
    }

    #[inline]
    fn lowright(self) -> Point {
        Point::new(self.width, self.height)
    }
}

impl From<OffsetRect> for OriginRect {
    fn from(rect: OffsetRect) -> OriginRect {
        OriginRect {
            width: rect.width(),
            height: rect.height()
        }
    }
}
