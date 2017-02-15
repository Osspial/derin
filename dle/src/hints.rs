use super::{Tr, Fr};
use dct::geometry::{Px, SizeBounds, Margins};
use std::ops::{Range, RangeFrom, RangeFull, RangeTo};

#[derive(Debug, Clone, Copy)]
pub struct TrRange {
    pub start: Option<Tr>,
    pub end: Option<Tr>
}

impl TrRange {
    /// Get the size of the TrRange, using `start_opt` if `self.start` is `None` and `end_opt` if
    /// `self.end` is `None`.
    pub fn size(self, start_opt: Tr, end_opt: Tr) -> Tr {
        self.end.unwrap_or(end_opt) - self.start.unwrap_or(start_opt)
    }
}

impl From<Tr> for TrRange {
    fn from(n: Tr) -> TrRange {
        TrRange::from(n..n + 1)
    }
}

impl From<Range<Tr>> for TrRange {
    fn from(r: Range<Tr>) -> TrRange {
        TrRange {
            start: Some(r.start),
            end: Some(r.end)
        }
    }
}

impl From<RangeFrom<Tr>> for TrRange {
    fn from(r: RangeFrom<Tr>) -> TrRange {
        TrRange {
            start: Some(r.start),
            end: None
        }
    }
}

impl From<RangeFull> for TrRange {
    fn from(_: RangeFull) -> TrRange {
        TrRange {
            start: None,
            end: None
        }
    }
}

impl From<RangeTo<Tr>> for TrRange {
    fn from(r: RangeTo<Tr>) -> TrRange {
        TrRange {
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
    pub struct NodeSpan(Into<TrRange>);

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



#[derive(Default, Debug, Clone, Copy)]
pub struct WidgetHints {
    pub size_bounds: SizeBounds,
    pub node_span: NodeSpan,
    pub place_in_cell: PlaceInCell,
    pub margins: Margins
}

impl WidgetHints {
    pub fn new(size_bounds: SizeBounds, node_span: NodeSpan, place_in_cell: PlaceInCell, margins: Margins) -> WidgetHints {
        WidgetHints {
            size_bounds: size_bounds,
            node_span: node_span,
            place_in_cell: place_in_cell,
            margins: margins
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackHints {
    /// Track-level minimum size. If the child minimum size is less than this, this is used instead.
    pub min_size: Px,
    /// Track-level maximum size. If this is less than the minimum size, minimum size takes priority
    /// and overrides this.
    pub max_size: Px,
    /// The proportion of free space this track takes up. This value represents a portion of the total
    /// "fractional space" available in the column or row - the layout engine attempts to set the pixel
    /// value to `total_free_space * fr_size / total_fr_size`.
    pub fr_size: Fr
}

impl Default for TrackHints {
    fn default() -> TrackHints {
        TrackHints {
            min_size: 0,
            max_size: Px::max_value(),
            fr_size: 1.0
        }
    }
}
