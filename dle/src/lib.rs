pub mod geometry;
pub mod layout;

use geometry::{OriginRect, OffsetRect};
use layout::NodeSpan;

pub trait Widget {
    fn span(&self) -> NodeSpan;
    fn min_rect(&self) -> Option<OriginRect>;
    fn max_rect(&self) -> Option<OriginRect>;
    fn set_rect(&mut self, rect: OffsetRect);
}

pub trait Container
        where for<'a> &'a Self: ContainerRef<'a, Widget = Self::Widget> {
    type Widget: Widget;
    type Key;

    fn get(&self, Self::Key) -> &Self::Widget;
    fn get_mut(&mut self, Self::Key) -> &mut Self::Widget;

    fn insert(&mut self, key: Self::Key, widget: Self::Widget);
    fn remove(&mut self, key: Self::Key) -> Self::Widget;
}

pub trait ContainerRef<'a> {
    type Widget: Widget + 'a;
    type SpanIter: Iterator<Item = &'a Self::Widget>;
    type SpanIterMut: Iterator<Item = &'a mut Self::Widget>;

    fn widgets_in_span(&self, span: NodeSpan) -> Self::SpanIter;
    fn widgets_in_span_mut(&mut self, span: NodeSpan) -> Self::SpanIterMut;
}

pub struct LayoutEngine<C: Container>
        where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget> {
    container: C
}
