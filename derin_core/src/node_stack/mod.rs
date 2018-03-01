mod inner;

use LoopFlow;
use std::cmp::{Ordering, Ord};
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use render::RenderFrame;
use tree::{Node, NodeSummary, Parent, NodeIdent, ChildEventRecv, UpdateTag, NodeSubtraitMut, RootID, NodeID};

use self::inner::{NRAllocCache, NRVec};
pub use self::inner::NodePath;

use cgmath::{Point2, Vector2, EuclideanSpace};
use cgmath_geometry::{BoundBox, GeoBox};

pub(crate) struct NodeStackBase<A, F: RenderFrame> {
    stack: NRAllocCache<A, F>
}

pub struct NodeStack<'a, A: 'a, F: 'a + RenderFrame, Root: 'a + ?Sized> {
    stack: NRVec<'a, A, F>,
    root: *mut Root
}

impl<A, F: RenderFrame> NodeStackBase<A, F> {
    pub fn new() -> NodeStackBase<A, F> {
        NodeStackBase {
            stack: NRAllocCache::new()
        }
    }

    // pub fn use_stack<'a, Root: Node<A, F>>(&'a mut self, node: &'a mut Root, root_id: RootID) -> NodeStack<'a, A, F, Root> {
    //     NodeStack {
    //         root: node,
    //         stack: self.stack.use_cache(node, root_id)
    //     }
    // }

    pub fn use_stack_dyn<'a>(&'a mut self, node: &'a mut Node<A, F>, root_id: RootID) -> NodeStack<'a, A, F, Node<A, F>> {
        NodeStack {
            root: node,
            stack: self.stack.use_cache(node, root_id)
        }
    }
}

impl<'a, A, F: RenderFrame, Root: Node<A, F> + ?Sized> NodeStack<'a, A, F, Root> {
    pub fn drain_to_root<G>(&mut self, mut for_each: G) -> NodePath<Root>
        where G: FnMut(&mut Node<A, F>, &[NodeIdent], Vector2<i32>)
    {
        self.drain_to_root_while(|node, ident, offset| {for_each(node, ident, offset); true}).unwrap()
    }

    pub fn drain_to_root_while<G>(&mut self, mut for_each: G) -> Option<NodePath<Root>>
        where G: FnMut(&mut Node<A, F>, &[NodeIdent], Vector2<i32>) -> bool
    {
        let mut continue_drain = true;
        while self.stack.len() > 1 && continue_drain {
            {
                let NodePath{ node, path, top_parent_offset } = self.stack.top_mut();
                continue_drain = for_each(node, path, top_parent_offset);
            }
            self.stack.pop();
        }

        if !continue_drain {
            return None;
        }

        let top_mut = self.stack.top_mut();
        for_each(top_mut.node, top_mut.path, Vector2::new(0, 0));

        assert_eq!(self.root as *mut () as usize, top_mut.node as *mut _ as *mut () as usize);
        Some(NodePath {
            node: unsafe{ &mut *self.root },
            path: top_mut.path,
            top_parent_offset: Vector2::new(0, 0)
        })
    }

    #[inline]
    pub fn move_to_root(&mut self) -> NodePath<Root> {
        self.stack.truncate(1);

        let top_mut = self.stack.top_mut();
        assert_eq!(self.root as *mut () as usize, top_mut.node as *mut _ as *mut () as usize);
        NodePath {
            node: unsafe{ &mut *self.root },
            path: top_mut.path,
            top_parent_offset: Vector2::new(0, 0)
        }
    }

    pub fn move_to_sibling_delta(&mut self, sibling_dist: isize) -> Result<NodePath<Node<A, F> + 'a>, Ordering> {
        if sibling_dist == 0 {
            return Ok(self.stack.top_mut());
        }

        let top_index = self.stack.top_index();
        let sibling_index = top_index as isize + sibling_dist;
        let left_cmp = sibling_index.cmp(&0);
        self.stack.pop().ok_or(left_cmp)?;

        let parent = self.stack.top().subtrait().as_parent().unwrap();
        let right_cmp = sibling_index.cmp(&(parent.num_children() as isize));

        match (left_cmp, right_cmp) {
            (Ordering::Greater, Ordering::Less) |
            (Ordering::Equal, Ordering::Less) => {
                let child = self.stack.try_push(|node, _|
                    node.subtrait_mut().as_parent().unwrap().child_by_index_mut(sibling_index as usize)
                ).unwrap();
                Ok(NodePath {
                    node: child.node,
                    path: self.stack.ident(),
                    top_parent_offset: self.top_parent_offset()
                })
            },
            _ => {
                self.stack.try_push(|node, _|
                    node.subtrait_mut().as_parent().unwrap().child_by_index_mut(top_index)
                ).unwrap();
                Err(left_cmp)
            }
        }
    }

    pub fn move_to_sibling_index(&mut self, sibling_index: usize) -> Result<NodePath<Node<A, F> + 'a>, Ordering> {
        let top_index = self.stack.top_index();
        if self.stack.pop().is_none() {
            return match sibling_index {
                0 => Ok(self.stack.top_mut()),
                _ => Err(Ordering::Greater)
            };
        }
        let child = self.stack.try_push(|node, _|
            node.subtrait_mut().as_parent().unwrap().child_by_index_mut(sibling_index)
        );
        match child {
            Some(child) => Ok(NodePath {
                node: child.node,
                path: self.stack.ident(),
                top_parent_offset: self.top_parent_offset()
            }),
            None => {
                self.stack.try_push(|node, _|
                    node.subtrait_mut().as_parent().unwrap().child_by_index_mut(top_index)
                ).unwrap();
                Err(Ordering::Greater)
            }
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
    pub fn ident(&self) -> &[NodeIdent] {
        self.stack.ident()
    }

    pub fn parent(&self) -> Option<&Parent<A, F>> {
        self.stack.nodes().rev().skip(1).next().map(|n| n.subtrait().as_parent().unwrap())
    }

    #[inline]
    pub fn nodes<'b>(&'b self) -> impl 'b + Iterator<Item=&'a Node<A, F>> + DoubleEndedIterator + ExactSizeIterator {
        self.stack.nodes()
    }

    pub fn try_push<G>(&mut self, with_top: G) -> Option<NodeSummary<&'a mut Node<A, F>>>
        where G: FnOnce(&'a mut Node<A, F>, &[NodeIdent]) -> Option<NodeSummary<&'a mut Node<A, F>>>
    {
        self.stack.try_push(with_top)
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
    pub fn top_parent_offset(&self) -> Vector2<i32> {
        self.stack.top_parent_offset()
    }

    #[inline]
    pub fn top_bounds_offset(&self) -> BoundBox<Point2<i32>> {
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
            // While the next item in the ident path and the active path are equal, increment the
            // diverge depth.
            while active_path_iter.next().and_then(|ident| ident_path_iter.peek().map(|i| i == ident)).unwrap_or(false) {
                diverge_depth += 1;
                ident_path_iter.next();
            }
        }
        if diverge_depth == 0 {
            return None;
        }
        self.stack.truncate(diverge_depth);

        let mut valid_path = true;
        for ident in ident_path_iter {
            valid_path = self.stack.try_push(|node, _| {
                if let NodeSubtraitMut::Parent(node_as_parent) = node.subtrait_mut() {
                    node_as_parent.child_mut(ident)
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
        where for<'b> G: FnMut(&'b mut Node<A, F>, &[NodeIdent], Vector2<i32>) -> &'b UpdateTag
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
            self.stack.try_push(|top_node, path| {
                top_node_offset = top_node.rect().min().to_vec();
                match top_node.subtrait_mut() {
                    NodeSubtraitMut::Node(top_node) => {
                        let node_tags = ChildEventRecv::from(top_node.update_tag());
                        if node_tags & flag_trail_flags != ChildEventRecv::empty() {
                            let update_tag = for_each_flag(top_node, path, top_parent_offset);
                            let flags_removed = flag_trail_flags - ChildEventRecv::from(update_tag);
                            nodes_visited += 1;
                            remove_flags |= flags_removed;
                        }

                        flags &= !flag_trail_flags;
                        on_flag_trail = None;

                        None
                    },
                    NodeSubtraitMut::Parent(top_node_as_parent) => {
                        let mut child_ident = None;

                        top_node_as_parent.children(&mut |children_summaries| {
                            for child_summary in children_summaries.iter() {
                                let child_flags = get_update_flags(&child_summary.update_tag);
                                if child_flags & flag_trail_flags != ChildEventRecv::empty() {
                                    on_flag_trail = Some(child_flags & flag_trail_flags);
                                    child_ident = Some(child_summary.ident);
                                    return LoopFlow::Break(());
                                }
                            }

                            LoopFlow::Continue
                        });

                        if child_ident.is_none() {
                            flags &= !flag_trail_flags;
                            on_flag_trail = None;
                        }

                        match child_ident {
                            Some(i) => top_node_as_parent.child_mut(i),
                            None => None
                        }
                    }
                }
            });

            flags &= !remove_flags;
        }

        nodes_visited
    }

    pub(crate) fn move_over_nodes<I, G>(&mut self, node_ids: I, mut for_each_node: G)
        where I: IntoIterator<Item=NodeID> + ExactSizeIterator + Clone,
              G: FnMut(&mut Node<A, F>, &[NodeIdent], usize, Vector2<i32>)
    {
        self.move_to_root();
        let mut nodes_left = node_ids.len();
        let mut child_index = 0;
        while nodes_left > 0 {
            let top_parent_offset = self.stack.top_parent_offset();
            let valid_child = self.stack.try_push(|top_node, top_path| {
                let top_id = top_node.update_tag().node_id;

                if let Some((iter_index, _)) = node_ids.clone().into_iter().enumerate().find(|&(_, id)| id == top_id) {
                    for_each_node(top_node, top_path, iter_index, top_parent_offset);
                    nodes_left -= 1;
                }

                if let Some(top_node_as_parent) = top_node.subtrait_mut().as_parent() {
                    return top_node_as_parent.child_by_index_mut(child_index);
                }

                None
            }).is_some();

            match valid_child {
                true => child_index = 0,
                false => {
                    child_index = self.stack.top_index() + 1;
                    if self.stack.pop().is_none() {
                        break
                    }
                }
            }
        }
    }
}