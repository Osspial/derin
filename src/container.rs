use std::marker::PhantomData;

use core::LoopFlow;
use core::render::RenderFrame;
use core::tree::{WidgetIdent, WidgetSummary, Widget};

pub trait WidgetContainer<F: RenderFrame> {
    type Action;

    /// Get the number of children stored within the container.
    fn num_children(&self) -> usize;

    /// Perform internal, immutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn children<'a, G, R>(&'a self, for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a Widget<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a;

    /// Perform internal, mutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn children_mut<'a, G, R>(&'a mut self, for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a mut Widget<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a;

    /// Get the child with the specified name.
    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<Self::Action, F>>> {
        self.children(|summary| {
            if summary.ident == widget_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    /// Mutably get the child with the specified name.
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<Self::Action, F>>> {
        self.children_mut(|summary| {
            if summary.ident == widget_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    /// Get the child at the specified index.
    ///
    /// The index of a child is generally assumed to correspond with the order in which the children
    /// are defined within the container.
    fn child_by_index(&self, mut index: usize) -> Option<WidgetSummary<&Widget<Self::Action, F>>> {
        self.children(|summary| {
            if index == 0 {
                LoopFlow::Break(summary)
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        })
    }
    /// Mutably get the child at the specified index.
    ///
    /// The index of a child is generally assumed to correspond with the order in which the children
    /// are defined within the container.
    fn child_by_index_mut(&mut self, mut index: usize) -> Option<WidgetSummary<&mut Widget<Self::Action, F>>> {
        self.children_mut(|summary| {
            if index == 0 {
                LoopFlow::Break(summary)
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SingleContainer<A, F: RenderFrame, N: Widget<A, F>> {
    pub widget: N,
    _marker: PhantomData<(A, F)>
}

impl<A, F: RenderFrame, N: Widget<A, F>> SingleContainer<A, F, N> {
    #[inline(always)]
    pub fn new(widget: N) -> SingleContainer<A, F, N> {
        SingleContainer{ widget, _marker: PhantomData }
    }
}

impl<A, F: RenderFrame, N: Widget<A, F>> WidgetContainer<F> for SingleContainer<A, F, N> {
    type Action = A;

    #[inline(always)]
    fn num_children(&self) -> usize {1}

    fn children<'a, G, R>(&'a self, mut for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a Widget<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a
    {
        let self_summary = WidgetSummary {
            widget: &self.widget as &Widget<A, F>,
            ident: WidgetIdent::Num(0),
            rect: self.widget.rect(),
            size_bounds: self.widget.size_bounds(),
            update_tag: self.widget.update_tag().clone(),
            index: 0
        };
        match for_each_child(self_summary) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a mut Widget<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a
    {
        let self_summary = WidgetSummary {
            rect: self.widget.rect(),
            size_bounds: self.widget.size_bounds(),
            update_tag: self.widget.update_tag().clone(),
            widget: &mut self.widget as &mut Widget<A, F>,
            ident: WidgetIdent::Num(0),
            index: 0
        };
        match for_each_child(self_summary) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }
}
