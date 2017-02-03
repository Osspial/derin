pub use dle::hints::*;

use std::iter;
use std::iter::{Empty, Once};

/// An iterator adapter that turns `LayoutHint`s into specific `NodeSpan`es
pub trait GridLayout {
    type WidgetHintsIter: Iterator<Item = WidgetHints>;
    type ColHintsIter: Iterator<Item = TrackHints>;
    type RowHintsIter: Iterator<Item = TrackHints>;

    fn grid_size(&self) -> GridSize;
    fn widget_hints(&self) -> Self::WidgetHintsIter;
    fn col_hints(&self) -> Self::ColHintsIter;
    fn row_hints(&self) -> Self::RowHintsIter;
}

#[derive(Clone, Copy)]
pub struct SingleNodeLayout(pub WidgetHints);

impl GridLayout for SingleNodeLayout {
    type WidgetHintsIter = Once<WidgetHints>;
    type ColHintsIter = Once<TrackHints>;
    type RowHintsIter = Once<TrackHints>;

    fn grid_size(&self) -> GridSize {
        GridSize::new(1, 1)
    }

    fn widget_hints(&self) -> Once<WidgetHints> {
        iter::once(self.0)
    }

    fn col_hints(&self) -> Once<TrackHints> {iter::once(TrackHints::default())}
    fn row_hints(&self) -> Once<TrackHints> {iter::once(TrackHints::default())}
}

impl Default for SingleNodeLayout {
    fn default() -> SingleNodeLayout {
        SingleNodeLayout(WidgetHints {
            node_span: NodeSpan::new(0, 0),
            ..WidgetHints::default()
        })
    }
}

#[derive(Default, Clone, Copy)]
pub struct EmptyNodeLayout;

impl GridLayout for EmptyNodeLayout {
    type WidgetHintsIter = Empty<WidgetHints>;
    type ColHintsIter = Empty<TrackHints>;
    type RowHintsIter = Empty<TrackHints>;

    fn grid_size(&self) -> GridSize {
        GridSize::new(0, 0)
    }

    fn widget_hints(&self) -> Empty<WidgetHints> {iter::empty()}
    fn col_hints(&self) -> Empty<TrackHints> {iter::empty()}
    fn row_hints(&self) -> Empty<TrackHints> {iter::empty()}
}

#[derive(Default, Clone)]
pub struct VerticalLayout {
    len: u32
}

impl VerticalLayout {
    pub fn new(len: u32) -> VerticalLayout {
        VerticalLayout {
            len: len
        }
    }
}

impl GridLayout for VerticalLayout {
    type WidgetHintsIter = VLWidgetHintsIter;
    type ColHintsIter = Empty<TrackHints>;
    type RowHintsIter = Once<TrackHints>;

    fn grid_size(&self) -> GridSize {
        GridSize::new(1, self.len)
    }

    fn widget_hints(&self) -> VLWidgetHintsIter {
        VLWidgetHintsIter {
            len: self.len,
            cur: 0
        }
    }

    fn col_hints(&self) -> Empty<TrackHints> {
        iter::empty()
    }

    fn row_hints(&self) -> Once<TrackHints> {
        iter::once(TrackHints {
            fr_size: 0.0,
            ..TrackHints::default()
        })
    }
}

pub struct VLWidgetHintsIter {
    len: u32,
    cur: u32
}

impl Iterator for VLWidgetHintsIter {
    type Item = WidgetHints;

    fn next(&mut self) -> Option<WidgetHints> {
        if self.cur < self.len {
            let slot = WidgetHints {
                size_bounds: SizeBounds::default(),
                node_span: NodeSpan::new(0..1, self.cur..self.cur+1),
                place_in_cell: PlaceInCell::default()
            };
            self.cur += 1;
            Some(slot)
        } else {
            None
        }
    }
}
