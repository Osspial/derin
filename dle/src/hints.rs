use super::Tr;
use geometry::OriginRect;
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

pub enum GridArea {
    Concrete(NodeSizing),
    Tagged(&'static str)
}

impl Default for GridArea {
    fn default() -> GridArea {
        GridArea::Concrete(NodeSizing::default())
    }
}

#[derive(Default)]
pub struct LayoutHint {
    pub grid_area: GridArea,
    pub place_in_cell: PlaceInCell
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeBounds {
    pub min: OriginRect,
    pub max: OriginRect
}

#[derive(Default, Debug, Clone, Copy)]
pub struct WidgetLayoutInfo {
    pub size_bounds: SizeBounds,
    pub node_span: NodeSpan,
    pub placement: PlaceInCell
}
