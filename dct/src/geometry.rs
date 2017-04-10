use std::ops::{Add, AddAssign};

pub type Px = u16;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeBounds {
    pub min: OriginRect,
    pub max: OriginRect
}

impl SizeBounds {
    pub fn new(min: OriginRect, max: OriginRect) -> SizeBounds {
        SizeBounds {
            min: min,
            max: max
        }
    }

    /// Bound a rectangle to be within the size bounds. Returns `Ok` if the rect wasn't bounded, and
    /// `Err` if it was bounded.
    pub fn bound_rect(self, mut desired_size: OriginRect) -> Result<OriginRect, OriginRect> {
        let mut size_bounded = false;

        if desired_size.width() < self.min.width() {
            desired_size.width = self.min.width();
            size_bounded = true;
        } else if desired_size.width() > self.max.width() {
            desired_size.width = self.max.width();
            size_bounded = true;
        }

        if desired_size.height() < self.min.height() {
            desired_size.height = self.min.height();
            size_bounded = true;
        } else if desired_size.height() > self.max.height() {
            desired_size.height = self.max.height();
            size_bounded = true;
        }

        if !size_bounded {
            Ok(desired_size)
        } else {
            Err(desired_size)
        }
    }
}

impl Default for SizeBounds {
    fn default() -> SizeBounds {
        SizeBounds {
            min: OriginRect::min(),
            max: OriginRect::max()
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Margins {
    pub top: Px,
    pub bottom: Px,
    pub left: Px,
    pub right: Px
}

impl Margins {
    pub fn new(top: Px, bottom: Px, left: Px, right: Px) -> Margins {
        Margins {
            top: top,
            bottom: bottom,
            left: left,
            right: right
        }
    }
}
