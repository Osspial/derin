use std::marker::PhantomData;

use core::LoopFlow;
use core::render::RenderFrame;
use core::tree::{NodeIdent, NodeSummary, Node};

pub trait WidgetContainer<F: RenderFrame> {
    type Action;

    fn num_children(&self) -> usize;

    fn children<'a, G, R>(&'a self, for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a;

    fn children_mut<'a, G, R>(&'a mut self, for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a mut Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a;

    fn child(&self, node_ident: NodeIdent) -> Option<NodeSummary<&Node<Self::Action, F>>> {
        self.children(|summary| {
            if summary.ident == node_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    fn child_mut(&mut self, node_ident: NodeIdent) -> Option<NodeSummary<&mut Node<Self::Action, F>>> {
        self.children_mut(|summary| {
            if summary.ident == node_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    fn child_by_index(&self, mut index: usize) -> Option<NodeSummary<&Node<Self::Action, F>>> {
        self.children(|summary| {
            if index == 0 {
                LoopFlow::Break(summary)
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        })
    }
    fn child_by_index_mut(&mut self, mut index: usize) -> Option<NodeSummary<&mut Node<Self::Action, F>>> {
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
pub struct SingleContainer<A, F: RenderFrame, N: Node<A, F>> {
    pub node: N,
    _marker: PhantomData<(A, F)>
}

impl<A, F: RenderFrame, N: Node<A, F>> SingleContainer<A, F, N> {
    #[inline(always)]
    pub fn new(node: N) -> SingleContainer<A, F, N> {
        SingleContainer{ node, _marker: PhantomData }
    }
}

impl<A, F: RenderFrame, N: Node<A, F>> WidgetContainer<F> for SingleContainer<A, F, N> {
    type Action = A;

    #[inline(always)]
    fn num_children(&self) -> usize {1}

    fn children<'a, G, R>(&'a self, mut for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a
    {
        let self_summary = NodeSummary {
            node: &self.node as &Node<A, F>,
            ident: NodeIdent::Num(0),
            rect: self.node.rect(),
            size_bounds: self.node.size_bounds(),
            update_tag: self.node.update_tag().clone(),
            index: 0
        };
        match for_each_child(self_summary) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a mut Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a
    {
        let self_summary = NodeSummary {
            rect: self.node.rect(),
            size_bounds: self.node.size_bounds(),
            update_tag: self.node.update_tag().clone(),
            node: &mut self.node as &mut Node<A, F>,
            ident: NodeIdent::Num(0),
            index: 0
        };
        match for_each_child(self_summary) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }
}
