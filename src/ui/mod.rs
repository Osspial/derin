pub use dct::{geometry, buttons, hints};
use self::hints::{WidgetHints, GridSize, TrackHints};
use std::cmp::PartialEq;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildId {
    Str(&'static str),
    Num(u32),
    StrCollection(&'static str, u32),
    NumCollection(u32, u32)
}

pub enum RawEvent {}

pub trait Node {
    type Action;
    type RenderState: PartialEq;

    fn type_name(&self) -> &'static str;
    fn render_state(&self) -> Self::RenderState;
    fn on_raw_event(&self, event: RawEvent) -> Option<Self::Action>;
}

pub trait ParentMut {
    type ChildAction;

    fn children(&self, _: !);
    fn children_mut(&mut self, _: !);
}

pub trait GridLayout<'a> {
    type ColHints: 'a + Iterator<Item = TrackHints>;
    type RowHints: 'a + Iterator<Item = TrackHints>;

    fn grid_size(&self) -> GridSize;
    fn col_hints(&'a self) -> Self::ColHints;
    fn row_hints(&'a self) -> Self::RowHints;

    fn get_hints(&self, ChildId) -> Option<WidgetHints>;
}
