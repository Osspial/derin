#![feature(const_fn, conservative_impl_trait)]

pub mod geometry;
pub mod layout;

use geometry::{OriginRect, OffsetRect};
use layout::{NodeSpan, PlaceInCell};

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct SizeBounds {
    pub min: OriginRect,
    pub max: OriginRect
}

impl Default for SizeBounds {
    fn default() -> SizeBounds {
        SizeBounds {
            min: OriginRect::min(),
            max: OriginRect::max()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WidgetLayoutInfo {
    pub size_bounds: SizeBounds,
    pub node_span: NodeSpan,
    pub placement: PlaceInCell
}

pub trait Widget {
    fn set_rect(&mut self, rect: OffsetRect);
}

pub trait Container
        where for<'a> &'a Self: ContainerRef<'a, Widget = Self::Widget> {
    type Widget: Widget;
    type Key;

    fn get(&self, Self::Key) -> Option<&(Self::Widget, WidgetLayoutInfo)>;
    fn get_mut(&mut self, Self::Key) -> Option<&mut (Self::Widget, WidgetLayoutInfo)>;

    fn insert(&mut self, key: Self::Key, widget: Self::Widget);
    fn remove(&mut self, key: Self::Key) -> Self::Widget;

    fn widgets_in_span(&self, span: NodeSpan) -> <&Self as ContainerRef>::SpanIter;
    fn widgets_in_span_mut(&mut self, span: NodeSpan) -> <&Self as ContainerRef>::SpanIterMut;
}

pub trait ContainerRef<'a> {
    type Widget: Widget + 'a;
    type SpanIter: Iterator<Item = &'a Self::Widget>;
    type SpanIterMut: Iterator<Item = &'a mut Self::Widget>;
}


pub struct LayoutEngine<C: Container>
        where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
{
    container: C,
    id: u32
}

impl<C: Container> LayoutEngine<C>
        where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
{
    pub fn new(container: C) -> LayoutEngine<C> {
        static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

        LayoutEngine {
            container: container,
            id: ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32
        }
    }

    pub fn get_widget(&self, key: C::Key) -> Option<&C::Widget> {
        self.container.get(key).map(|w| &w.0)
    }

    pub fn get_widget_mut(&mut self, key: C::Key) -> Option<&mut C::Widget> {
        self.container.get_mut(key).map(|w| &mut w.0)
    }
}
