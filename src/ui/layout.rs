pub use dle::hints::*;

/// An iterator adapter that turns `LayoutHint`s into specific `NodeSpan`es
pub trait GridLayout: Iterator<Item = WidgetLayoutInfo> {
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
    type Item = WidgetLayoutInfo;

    fn next(&mut self) -> Option<WidgetLayoutInfo> {
        match self.consumed {
            true => None,
            false => {
                self.consumed = true;
                Some(WidgetLayoutInfo {
                    size_bounds: SizeBounds::default(),
                    node_span: NodeSpan::new(0, 0),
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
    type Item = WidgetLayoutInfo;

    fn next(&mut self) -> Option<WidgetLayoutInfo> {None}

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
    type Item = WidgetLayoutInfo;

    fn next(&mut self) -> Option<WidgetLayoutInfo> {
        if self.cur < self.len {
            let slot = WidgetLayoutInfo {
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
