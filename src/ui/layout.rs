use std::ops::{Range, RangeFrom, RangeFull, RangeTo};
use boolinator::Boolinator;

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

#[derive(Debug, Default, Clone, Copy)]
pub struct GridSlot {
    pub node_span: NodeSpan,
    pub place_in_cell: PlaceInCell
}

/// An iterator adapter that turns `LayoutHint`s into specific `NodeSpan`es
pub trait GridLayout: Iterator<Item = GridSlot> {
    fn grid_size(&self) -> GridSize;
}

#[derive(Default, Clone)]
pub struct SingleNodeLayout {
    consumed: bool
}

impl SingleNodeLayout {
    pub fn new() -> SingleNodeLayout {
        SingleNodeLayout::default()
    }
}

impl GridLayout for SingleNodeLayout {
    fn grid_size(&self) -> GridSize {
        GridSize::new(1, 1)
    }
}

impl Iterator for SingleNodeLayout {
    type Item = GridSlot;

    fn next(&mut self) -> Option<GridSlot> {
        match self.consumed {
            true => None,
            false => {
                self.consumed = true;
                Some(GridSlot {
                    node_span: NodeSpan::new(0..1, 0..1),
                    place_in_cell: PlaceInCell::default()
                })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, Some(1))
    }
}
impl ExactSizeIterator for SingleNodeLayout {}

#[derive(Default, Clone)]
pub struct EmptyNodeLayout;

impl GridLayout for EmptyNodeLayout {
    fn grid_size(&self) -> GridSize {
        GridSize::new(0, 0)
    }
}

impl Iterator for EmptyNodeLayout {
    type Item = GridSlot;

    fn next(&mut self) -> Option<GridSlot> {None}

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}
impl ExactSizeIterator for EmptyNodeLayout {}

#[derive(Default, Clone)]
pub struct VerticalLayout {
    len: u32,
    cur: u32
}

impl VerticalLayout {
    pub fn new(len: u32) -> VerticalLayout {
        VerticalLayout {
            len: len,
            cur: 0
        }
    }
}

impl GridLayout for VerticalLayout {
    fn grid_size(&self) -> GridSize {
        GridSize::new(1, self.len)
    }
}

impl Iterator for VerticalLayout {
    type Item = GridSlot;

    fn next(&mut self) -> Option<GridSlot> {
        (self.cur < self.len).as_some({
            let slot = GridSlot {
                node_span: NodeSpan::new(0..1, self.cur..self.cur+1),
                place_in_cell: PlaceInCell::default()
            };
            self.cur += 1;
            slot
        })
    }
}
