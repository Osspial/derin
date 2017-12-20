mod inner;

use LoopFlow;
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use render::RenderFrame;
use tree::{Node, NodeIdent, ChildEventRecv, UpdateTag, NodeSubtrait, NodeSubtraitMut};

use self::inner::{NRAllocCache, NRVec};

use cgmath::{Vector2, EuclideanSpace};
use cgmath_geometry::{BoundRect, Rectangle};

pub struct NodeStackBase<A, F: RenderFrame> {
    stack: NRAllocCache<A, F>
}

pub struct NodeStack<'a, A: 'a, F: 'a + RenderFrame, Root: 'a + > {
    stack: NRVec<'a, A, F>,
    root: *mut Root
}

impl<A, F: RenderFrame> NodeStackBase<A, F> {
    pub fn new() -> NodeStackBase<A, F> {
        NodeStackBase {
            stack: NRAllocCache::new()
        }
    }

    pub fn use_stack<'a, Root: Node<A, F>>(&'a mut self, node: &'a mut Root) -> NodeStack<'a, A, F, Root> {
        NodeStack {
            root: node,
            stack: self.stack.use_cache(node)
        }
    }
}

impl<'a, A, F: RenderFrame, Root: Node<A, F>> NodeStack<'a, A, F, Root> {
    #[inline]
    pub fn drain_to_root<G>(&mut self, mut for_each: G) -> &mut Root
        where G: FnMut(&mut Node<A, F>, Vector2<u32>)
    {
        loop {
            let offset = self.stack.top_parent_offset();
            match self.stack.pop() {
                Some(node) => for_each(node, offset),
                None       => break
            }
        }

        for_each(self.stack.top_mut(), Vector2::new(0, 0));

        assert_eq!(self.root, self.stack.top_mut() as *mut _ as *mut Root);
        unsafe{ &mut *self.root }
    }

    #[inline]
    pub fn move_to_root(&mut self) -> &mut Root {
        self.stack.truncate(1);

        assert_eq!(self.root, self.stack.top_mut() as *mut _ as *mut Root);
        unsafe{ &mut *self.root }
    }

    #[inline]
    pub fn top(&self) -> &Node<A, F> {
        self.stack.top()
    }

    #[inline]
    pub fn top_mut(&mut self) -> &mut Node<A, F> {
        self.stack.top_mut()
    }

    #[inline]
    pub fn top_ident(&self) -> NodeIdent {
        self.stack.top_ident()
    }

    #[inline]
    pub fn nodes<'b>(&'b self) -> impl 'b + Iterator<Item=&'a Node<A, F>> + DoubleEndedIterator + ExactSizeIterator {
        self.stack.nodes()
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&'a mut Node<A, F>> {
        self.stack.pop()
    }

    #[inline]
    pub fn depth(&self) -> usize {
        // The len is always going to be >= 1, so when it's 1 we're at the root node (dpeth 0)
        self.stack.len() - 1
    }

    #[inline]
    pub fn move_to_hover(&mut self) {
        // Move to the MOUSE_HOVER tagged node.
        self.move_over_flags(ChildEventRecv::MOUSE_HOVER, |node, _| node.update_tag());
    }

    #[inline]
    pub fn top_parent_offset(&self) -> Vector2<u32> {
        self.stack.top_parent_offset()
    }

    #[inline]
    pub fn top_bounds_offset(&self) -> BoundRect<u32> {
        self.stack.top_bounds_offset()
    }

    /// Returns number of nodes visited. `for_each_flag` takes node at flag and Vector2 giving offset from
    /// root of the node's parent.
    pub fn move_over_flags<G>(&mut self, mut flags: ChildEventRecv, mut for_each_flag: G) -> usize
        where G: FnMut(&mut Node<A, F>, Vector2<u32>) -> &UpdateTag
    {
        assert_ne!(self.stack.nodes().len(), 0);

        let get_update_flags = |update: &UpdateTag| update.child_event_recv.get() | ChildEventRecv::from(update);
        // Remove flags from the search set that aren't found at the root of the tree.
        flags &= {
            let root_update = self.stack.nodes().next().unwrap().update_tag();
            get_update_flags(root_update)
        };

        let mut nodes_visited = 0;
        let mut on_flag_trail = None;

        while !flags.is_empty() {
            if on_flag_trail.is_none() {
                // The index and update tag of the closest flagged parent
                let (cfp_index, cfp_update_tag) =
                    self.stack.nodes().map(|n| n.update_tag()).enumerate().rev()
                        .find(|&(_, u)| flags & (get_update_flags(u)) != ChildEventRecv::empty())
                        .unwrap();

                self.stack.truncate(cfp_index + 1);
                on_flag_trail = Some(get_update_flags(cfp_update_tag) & flags);
            }
            let flag_trail_flags = on_flag_trail.unwrap();
            let mut remove_flags = ChildEventRecv::empty();
            let top_parent_offset = self.stack.top_parent_offset();

            self.stack.try_push(|top_node| {
                macro_rules! call_fn {
                    ($node:expr, $offset:expr) => {{
                        let update_tag = for_each_flag($node, $offset);
                        let flags_removed = flag_trail_flags - ChildEventRecv::from(update_tag);
                        nodes_visited += 1;
                        remove_flags |= flags_removed;
                    }}
                }

                match top_node.subtrait_mut() {
                    NodeSubtraitMut::Node(top_node) => {
                        let node_tags = ChildEventRecv::from(top_node.update_tag());
                        if node_tags & flag_trail_flags != ChildEventRecv::empty() {
                            call_fn!(top_node, top_parent_offset);
                        }

                        flags &= !flag_trail_flags;
                        on_flag_trail = None;

                        None
                    },
                    NodeSubtraitMut::Parent(top_node_as_parent) => {
                        let mut child_ident = None;

                        let top_node_offset = top_node_as_parent.bounds().min().to_vec();
                        top_node_as_parent.children_mut(&mut |children_summaries| {
                            let mut run_on_index = None;

                            for (index, child_summary) in children_summaries.iter_mut().enumerate() {
                                let node_tags = ChildEventRecv::from(&child_summary.update_tag);
                                if node_tags & flag_trail_flags != ChildEventRecv::empty() {
                                    flags &= !flag_trail_flags;
                                    on_flag_trail = None;
                                    child_ident = Some(child_summary.ident);

                                    run_on_index = Some(index);
                                    break;
                                }

                                match child_summary.node.subtrait() {
                                    NodeSubtrait::Parent(_) => {
                                        let child_flags = child_summary.update_tag.child_event_recv.get();
                                        if child_flags & flag_trail_flags != ChildEventRecv::empty() {
                                            on_flag_trail = Some(child_flags & flag_trail_flags);
                                            child_ident = Some(child_summary.ident);

                                            break;
                                        }
                                    },
                                    NodeSubtrait::Node(_) => ()
                                }
                            }

                            match run_on_index {
                                None => LoopFlow::Continue,
                                Some(index) => {
                                    call_fn!(children_summaries[index].node, top_parent_offset + top_node_offset);
                                    LoopFlow::Break(())
                                }
                            }
                        });

                        if child_ident.is_none() {
                            flags &= !flag_trail_flags;
                            on_flag_trail = None;
                        }

                        match child_ident {
                            Some(i) => Some((top_node_as_parent.child_mut(i).expect("Unexpected node removal").node, i)),
                            None => None
                        }
                    }
                }
            });

            flags &= !remove_flags;
        }

        nodes_visited
    }
}
