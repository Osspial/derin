mod inner;

use LoopFlow;
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use render::RenderFrame;
use tree::{Node, NodeIdent, ChildEventRecv, UpdateTag, NodeSubtrait, NodeSubtraitMut};

use self::inner::{NRAllocCache, NRVec};
pub use self::inner::NodePath;

use cgmath::{Point2, Vector2, EuclideanSpace};
use cgmath_geometry::{BoundBox, GeoBox};

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
    pub fn drain_to_root<G>(&mut self, mut for_each: G) -> NodePath<Root>
        where G: FnMut(&mut Node<A, F>, &[NodeIdent], Vector2<u32>)
    {
        while self.stack.len() > 1 {
            {
                let offset = self.stack.top_parent_offset();
                let NodePath{ node, path } = self.stack.top_mut();
                for_each(node, path, offset);
            }
            self.stack.pop();
        }

        let top_mut = self.stack.top_mut();
        for_each(top_mut.node, top_mut.path, Vector2::new(0, 0));

        assert_eq!(self.root, top_mut.node as *mut _ as *mut Root);
        NodePath {
            node: unsafe{ &mut *self.root },
            path: top_mut.path
        }
    }

    #[inline]
    pub fn move_to_root(&mut self) -> NodePath<Root> {
        self.stack.truncate(1);

        let top_mut = self.stack.top_mut();
        assert_eq!(self.root, top_mut.node as *mut _ as *mut Root);
        NodePath {
            node: unsafe{ &mut *self.root },
            path: top_mut.path
        }
    }

    #[inline]
    pub fn top(&self) -> &Node<A, F> {
        self.stack.top()
    }

    #[inline]
    pub fn top_mut(&mut self) -> NodePath<Node<A, F> + 'a> {
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
    pub fn ident(&self) -> &[NodeIdent] {
        self.stack.ident()
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
        self.move_over_flags(ChildEventRecv::MOUSE_HOVER, |node, _, _| node.update_tag());
    }

    #[inline]
    pub fn move_to_keyboard_focus(&mut self) -> Option<NodePath<Node<A, F> + 'a>> {
        let mut found_node = false;
        self.move_over_flags(ChildEventRecv::KEYBOARD, |node, _, _| {
            found_node = true;
            node.update_tag()
        });
        match found_node {
            false => None,
            true => Some(self.top_mut())
        }
    }

    #[inline]
    pub fn top_parent_offset(&self) -> Vector2<u32> {
        self.stack.top_parent_offset()
    }

    #[inline]
    pub fn top_bounds_offset(&self) -> BoundBox<Point2<u32>> {
        self.stack.top_bounds_offset()
    }

    pub fn move_to_path<I>(&mut self, ident_path: I) -> Option<NodePath<Node<A, F> + 'a>>
        where I: IntoIterator<Item=NodeIdent>
    {
        let mut ident_path_iter = ident_path.into_iter().peekable();

        // Find the depth at which the given path and the current path diverge, and move the stack
        // to that depth.
        let mut diverge_depth = 0;
        {
            let mut active_path_iter = self.stack.ident().iter();
            while active_path_iter.next() == ident_path_iter.peek() {
                diverge_depth += 1;
                ident_path_iter.next();
            }
        }
        self.stack.truncate(diverge_depth);

        let mut valid_path = true;
        for ident in ident_path_iter {
            valid_path = self.stack.try_push(|node, _| {
                if let NodeSubtraitMut::Parent(node_as_parent) = node.subtrait_mut() {
                    node_as_parent.child_mut(ident).map(|child_summary| (child_summary.node, ident))
                } else {
                    None
                }
            }).is_some();

            if !valid_path {
                break;
            }
        }

        match valid_path {
            true => Some(self.stack.top_mut()),
            false => None
        }
    }

    /// Returns number of nodes visited. `for_each_flag` takes node at flag, ident path of node,
    /// and Vector2 giving offset from root of the node's parent.
    pub fn move_over_flags<G>(&mut self, mut flags: ChildEventRecv, mut for_each_flag: G) -> usize
        where for<'b> G: FnMut(&'b mut Node<A, F>, &[NodeIdent], Vector2<u32>) -> &'b UpdateTag
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

            let mut top_node_offset = Vector2::new(0, 0);
            macro_rules! call_fn {
                ($node:expr, $path:expr, $offset:expr) => {{
                    let update_tag = for_each_flag($node, $path, $offset);
                    let flags_removed = flag_trail_flags - ChildEventRecv::from(update_tag);
                    nodes_visited += 1;
                    remove_flags |= flags_removed;
                }}
            }
            let child_pushed = self.stack.try_push(|top_node, path| {
                top_node_offset = top_node.bounds().min().to_vec();
                match top_node.subtrait_mut() {
                    NodeSubtraitMut::Node(top_node) => {
                        let node_tags = ChildEventRecv::from(top_node.update_tag());
                        if node_tags & flag_trail_flags != ChildEventRecv::empty() {
                            call_fn!(top_node, path, top_parent_offset);
                        }

                        flags &= !flag_trail_flags;
                        on_flag_trail = None;

                        None
                    },
                    NodeSubtraitMut::Parent(top_node_as_parent) => {
                        let mut child_ident = None;

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
                                    // We call the function after the loop has exited and the ident
                                    // has been pushed to the stack, so we can call the function with
                                    // the ident.
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
            }).is_some();
            if child_pushed {
                let NodePath{ node: child, path: child_path } = self.stack.top_mut();
                call_fn!(child, child_path, top_parent_offset + top_node_offset);
            }

            flags &= !remove_flags;
        }

        nodes_visited
    }
}
