use std::ops::{Add, AddAssign};

pub type Px = i32;

two_axis_type!{
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Point(Px);
}

impl Point {
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

    #[inline]
    fn width(self) -> Px {
        self.lowright().x.saturating_sub(self.topleft().x)
    }

    #[inline]
    fn height(self) -> Px {
        self.lowright().y.saturating_sub(self.topleft().y)
    }

    #[inline]
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
    pub fn new(left: Px, top: Px, right: Px, bottom: Px) -> OffsetRect {
        OffsetRect {
            topleft: Point::new(left, top),
            lowright: Point::new(right, bottom)
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

impl From<OriginRect> for OffsetRect {
    fn from(ogr: OriginRect) -> OffsetRect {
        OffsetRect {
            topleft: Point::new(0, 0),
            lowright: ogr.lowright()
        }
    }
}

// This is #[repr(C)] because of stupid evil pointer hacks in dww.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
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
