use super::{Px, Tr, Fr};
use geometry::{OriginRect, Rect};
use std::ops::{Range, RangeFrom, RangeFull, RangeTo};

#[derive(Debug, Clone, Copy)]
pub struct DyRange {
    pub start: Option<Tr>,
    pub end: Option<Tr>
}

impl DyRange {
    /// Get the size of the DyRange, using `start_opt` if `self.start` is `None` and `end_opt` if
    /// `self.end` is `None`.
    pub fn size(self, start_opt: Tr, end_opt: Tr) -> Tr {
        self.end.unwrap_or(end_opt) - self.start.unwrap_or(start_opt)
    }
}

impl From<Tr> for DyRange {
    fn from(n: Tr) -> DyRange {
        DyRange::from(n..n + 1)
    }
}

impl From<Range<Tr>> for DyRange {
    fn from(r: Range<Tr>) -> DyRange {
        DyRange {
            start: Some(r.start),
            end: Some(r.end)
        }
    }
}

impl From<RangeFrom<Tr>> for DyRange {
    fn from(r: RangeFrom<Tr>) -> DyRange {
        DyRange {
            start: Some(r.start),
            end: None
        }
    }
}

impl From<RangeFull> for DyRange {
    fn from(_: RangeFull) -> DyRange {
        DyRange {
            start: None,
            end: None
        }
    }
}

impl From<RangeTo<Tr>> for DyRange {
    fn from(r: RangeTo<Tr>) -> DyRange {
        DyRange {
            start: None,
            end: Some(r.end)
        }
    }
}

macro_rules! two_axis_type {
    () => {};
    ($(#[$attr:meta])* pub struct $name:ident (Into<$t:ty>); $($rest:tt)*) => {
        $(#[$attr])*
        pub struct $name {
            pub x: $t,
            pub y: $t
        }

        impl $name {
            #[inline]
            pub fn new<X, Y>(x: X, y: Y) -> $name
                    where X: Into<$t>,
                          Y: Into<$t> {
                $name {
                    x: x.into(),
                    y: y.into()
                }
            }
        }

        two_axis_type!($($rest)*);
    };
    ($(#[$attr:meta])* pub struct $name:ident ($t:ty); $($rest:tt)*) => {
        $(#[$attr])*
        pub struct $name {
            pub x: $t,
            pub y: $t
        }

        impl $name {
            #[inline]
            pub fn new(x: $t, y: $t) -> $name {
                $name {
                    x: x,
                    y: y
                }
            }
        }

        two_axis_type!($($rest)*);
    }
}

two_axis_type!{
    #[derive(Default, Debug, Clone, Copy)]
    pub struct NodeSizing(Option<Tr>);

    #[derive(Default, Debug, Clone, Copy)]
    pub struct GridSize(Tr);

    #[derive(Debug, Clone, Copy)]
    pub struct NodeSpan(Into<DyRange>);

    #[derive(Default, Debug, Clone, Copy)]
    pub struct PlaceInCell(Place);
}

impl Default for NodeSpan {
    fn default() -> NodeSpan {
        NodeSpan::new(0..0, 0..0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Place {
    Stretch,
    Start,
    End,
    Center
}

impl Default for Place {
    fn default() -> Place {
        Place::Stretch
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
            desired_size.lowright.x = self.min.width();
            size_bounded = true;
        } else if desired_size.width() > self.max.width() {
            desired_size.lowright.x = self.max.width();
            size_bounded = true;
        }

        if desired_size.height() < self.min.height() {
            desired_size.lowright.y = self.min.height();
            size_bounded = true;
        } else if desired_size.height() > self.max.height() {
            desired_size.lowright.y = self.max.height();
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
pub struct WidgetLayoutInfo {
    pub size_bounds: SizeBounds,
    pub node_span: NodeSpan,
    pub place_in_cell: PlaceInCell
}

#[derive(Default, Debug, Clone, Copy)]
pub struct TrackLayoutInfo {
    pub min_size: Px,
    pub max_size: Px,
    pub fr_size: Fr
}
