enum FocusMove {
    Child(NodeIdent),
    Parent,
    Sibling(isize)
}

struct NodeSelectorReceives {}
impl NodeSelectorReceives {
    fn receive_status(&self, selector: Selector) -> ReceiveStatus {
        unimplemented!()
    }

    fn set_self_receive(&mut self, selector: Selector) {
        unimplemented!()
    }

    fn set_child_receive(&mut self, selector: Selector) {
        unimplemented!()
    }
}
enum ReceiveStatus {
    Self,
    Child,
    None
}

trait SelectorType {
    fn enter_fn(&self, node: &mut Node, delta: SelectorDelta);
    fn run_fn(&self, node: &mut Node, delta: SelectorDelta);
    fn exit_fn(&self, node: &mut node, delta: SelectorDelta)
    fn to_enum(&self) -> Selector;
}

enum Selector {
    Keyboard,
    Mouse(MouseSelector),
}

enum MouseSelector {
    Move,
    Button(MouseButton),
}

struct SelectorDelta {
    selector_ops: &mut SelectorOps,
    triggered_move: &mut bool
}
impl SelectorDelta {
    fn node_ops(&mut self) -> Option<NodeOps> {
        unimplemented!()
    }
    fn move_to_child<F>(self, choose_child: F)
        where F: FnMut(&mut Node) -> bool
    {
        self.selector_ops.move_to_child(|node| {
            let child = node.children_mut(.., |node_list| {
                match choose_child(node) {
                    true => LoopFlow::Break(node),
                    false => LoopFlow::Continue
                }
            });
            *self.triggered_move = child.is_some();
            child
        });
    }

    fn move_to_sibling_forward<F>(self, choose_sibling: F)
        where F: FnMut(&mut Node) -> bool
    {
        let cur_index = self.selector_ops.node_index();
        self.selector_ops.move_to_sibling(|parent| {
            let num_children = parent.num_children();
            let sibling = parent.children_mut(cur_index..num_children, |node_list| {
                match choose_sibling(node) {
                    true => LoopFlow::Break(node),
                    false => LoopFlow::Continue
                }
            });
            *self.triggered_move = sibling.map(|s| s.index != cur_index).unwrap_or(false);
            sibling
        });
    }

    fn move_to_sibling_reverse<F>(self, _: F)
        where F: FnMut(&mut Node) -> bool
    {
        let cur_index = self.selector_ops.node_index();
        self.selector_ops.move_to_sibling(|parent| {
            let num_children = parent.num_children();
            let sibling = parent.children_mut((0..cur_index).rev(), |node_list| {
                match choose_sibling(node) {
                    true => LoopFlow::Break(node),
                    false => LoopFlow::Continue
                }
            });
            *self.triggered_move = sibling.map(|s| s.index != cur_index).unwrap_or(false);
            sibling
        });
    }

    fn move_to_parent(self)
    {
        *self.triggered_move = self.selector_ops.move_to_parent();
    }
}

struct NodeOps {
    event: &mut Option<NodeEvent>,
    selector_ops: &mut SelectorOps,
}
impl NodeOps {
    fn take_selector(&mut self, selector: Selector) -> &mut Self {
        let primary_selector = self.selector_ops.selector_enum();
        {
            let mut stack = self.selector_ops.pause_selection();
            let mut new_ops = stack.move_to_selector(selector);
            if new_ops.selectors_at_node().fold(false, |s| s == primary_selector) {
                return;
            }

            new_ops.unselect();
        }

        self.selector_ops.set_to_selector(selector);
    }
    fn send_event(self, event: NodeEvent) {
        *self.event = Some(event);
    }
}

struct NodeStack {}
impl NodeStack {
    fn move_to_selector(&mut self, _: Selector) -> SelectorOps {unimplemented!()}
}

struct SelectorOps {}
impl SelectorOps {
    fn selector_enum(&self) -> Selector {
        unimplemented!()
    }
    fn node(&mut self) -> &mut Node {
        unimplemented!()
    }
    fn node_index(&self) -> usize {
        unimplemented!()
    }
    fn move_to_parent(&mut self) -> bool {
        unimplemented!()
    }
    fn move_to_child<F>(&mut self, child_fn: F)
        where F: FnOnce(&mut Parent) -> Option<&mut Node>
    {
        unimplemented!()
    }
    fn move_to_sibling<F>(&mut self, sibling_fn: F)
        where F: FnOnce(&mut Parent, NodeIdent) -> Option<&mut Node>
    {
        unimplemented!()
    }
    fn bubble(&mut self, event: NodeEvent) {
        unimplemented!()
    }
    fn unselect(&mut self) {
        unimplemented!()
    }

    fn selectors_at_node(&self) -> impl Iterator<Item=Selector> {
        unimplemented!();
        None
    }

    fn pause_selection(&mut self) -> NodeStack {
        unimplemented!()
    }
    fn set_to_selector(&mut self, _: Selector) {
        unimplemented!()
    }
}

struct SelectorDispatch {
    node_stack: NodeStack
}

impl SelectorDispatch {
    fn run_fn<S: Selector>(&mut self, selector: S) {
        let selector_ops = self.node_stack.move_to_selector(selector.to_enum());

        let mut triggered_move = true;
        while triggered_move {
            selector.run_fn(
                selector_ops.node(),
                SelectorDelta{ selector_ops: &mut selector_ops, triggered_move: &mut triggered_move },
                self.fns
            );
        }
    }
}
