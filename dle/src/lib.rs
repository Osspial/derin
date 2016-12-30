#![feature(const_fn)]

pub mod geometry;
pub mod layout;
mod grid;

use geometry::{OriginRect, OffsetRect};
use layout::{NodeSpan, PlaceInCell};
use grid::TrackVec;

use std::sync::atomic::{AtomicUsize, Ordering};

pub type Tr = u32;
pub type Px = u32;

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

pub struct WidgetData<W: Widget> {
    pub widget: W,
    pub layout_info: WidgetLayoutInfo
}

#[derive(Default, Debug, Clone, Copy)]
pub struct WidgetLayoutInfo {
    size_bounds: SizeBounds,
    node_span: NodeSpan,
    placement: PlaceInCell
}

impl WidgetLayoutInfo {
    pub fn new() -> WidgetLayoutInfo {
        WidgetLayoutInfo::default()
    }

    pub fn size_bounds(self) -> SizeBounds {
        self.size_bounds
    }

    pub fn node_span(self) -> NodeSpan {
        self.node_span
    }

    pub fn placement(self) -> PlaceInCell {
        self.placement
    }
}

pub trait Widget {
    fn set_rect(&mut self, rect: OffsetRect);
}

pub trait Container
        where for<'a> &'a Self: ContainerRef<'a, Widget = Self::Widget> {
    type Widget: Widget;
    type Key: Clone + Copy;

    fn get(&self, Self::Key) -> Option<&WidgetData<Self::Widget>>;
    fn get_mut(&mut self, Self::Key) -> Option<&mut WidgetData<Self::Widget>>;

    fn insert(&mut self, key: Self::Key, widget: Self::Widget) -> Option<WidgetData<Self::Widget>>;
    fn remove(&mut self, key: Self::Key) -> Option<WidgetData<Self::Widget>>;

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
    grid: TrackVec,
    id: u32
}

impl<C: Container> LayoutEngine<C>
        where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
{
    pub fn new(container: C) -> LayoutEngine<C> {
        static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

        LayoutEngine {
            container: container,
            grid: TrackVec::new(),
            id: ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32
        }
    }

    pub fn get_widget(&self, key: C::Key) -> Option<&C::Widget> {
        self.container.get(key).map(|w| &w.widget)
    }

    pub fn get_widget_mut(&mut self, key: C::Key) -> Option<&mut C::Widget> {
        self.container.get_mut(key).map(|w| &mut w.widget)
    }
}
