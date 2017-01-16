pub use dle::widget_hints::*;

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
        if self.cur < self.len {
            let slot = GridSlot {
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
