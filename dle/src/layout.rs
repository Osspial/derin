use std::ops::{Range, RangeFrom, RangeFull, RangeTo};

#[derive(Debug, Clone, Copy)]
pub struct DyRange<Idx> {
    pub start: Option<Idx>,
    pub end: Option<Idx>
}

impl<Idx> From<Range<Idx>> for DyRange<Idx> {
    fn from(r: Range<Idx>) -> DyRange<Idx> {
        DyRange {
            start: Some(r.start),
            end: Some(r.end)
        }
    }
}

impl<Idx> From<RangeFrom<Idx>> for DyRange<Idx> {
    fn from(r: RangeFrom<Idx>) -> DyRange<Idx> {
        DyRange {
            start: Some(r.start),
            end: None
        }
    }
}

impl<Idx> From<RangeFull> for DyRange<Idx> {
    fn from(_: RangeFull) -> DyRange<Idx> {
        DyRange {
            start: None,
            end: None
        }
    }
}

impl<Idx> From<RangeTo<Idx>> for DyRange<Idx> {
    fn from(r: RangeTo<Idx>) -> DyRange<Idx> {
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
    pub struct NodeSizing(Option<u32>);

    #[derive(Default, Debug, Clone, Copy)]
    pub struct GridSize(u32);

    #[derive(Debug, Clone, Copy)]
    pub struct NodeSpan(Into<DyRange<u32>>);

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
