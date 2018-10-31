use crate::{
    LoopFlow,
    tree::WidgetID,
    render::RenderFrame,
    widget_tree::WidgetTree,
    widget_stack::WidgetStack,
    offset_widget::{OffsetWidgetTrait, OffsetWidgetTraitAs},
};
use fnv::FnvHashSet;
use std::{
    rc::Rc,
    cell::RefCell
};

#[derive(Debug, Clone)]
pub(crate) enum UpdateStateShared {
    Occupied(Rc<RefCell<UpdateState>>),
    Vacant
}

#[derive(Debug)]
pub(crate) struct UpdateState {
    pub redraw: FnvHashSet<WidgetID>,
    pub relayout: FnvHashSet<WidgetID>,
    pub child_updated: FnvHashSet<WidgetID>,
    pub widget_tree: WidgetTree
}

impl UpdateState {
    pub fn update_virtual_tree<A, F: RenderFrame>(&mut self, stack: &mut WidgetStack<A, F>) {
        let UpdateState {
            ref mut child_updated,
            ref mut widget_tree,
            ..
        } = self;
        for parent_id in child_updated.drain() {
            let mut parent_widget = stack.move_to_widget_with_tree(parent_id, widget_tree);
            let parent_opt = parent_widget.as_mut().and_then(|w| w.widget.as_parent_mut());

            let parent = match parent_opt {
                Some(parent) => parent,
                None => continue
            };
            parent.children(|summary| {
                widget_tree.insert(parent_id, summary.widget.widget_tag().widget_id, summary.index, summary.ident);
                LoopFlow::Continue
            });
        }
    }
}

impl UpdateStateShared {
    pub fn new() -> UpdateStateShared {
        UpdateStateShared::Vacant
    }

    pub fn set_parent_state(&mut self, id: WidgetID, parent_state: Rc<RefCell<UpdateState>>) {
        match self {
            UpdateStateShared::Vacant{..} => {
                {
                    let mut parent_state = parent_state.borrow_mut();
                    parent_state.redraw.insert(id);
                    parent_state.relayout.insert(id);
                    parent_state.child_updated.insert(id);
                }
                *self = UpdateStateShared::Occupied(parent_state)
            },
            UpdateStateShared::Occupied(old_state) => {
                {
                    let mut old_state = old_state.borrow_mut();
                    old_state.redraw.remove(&id);
                    old_state.relayout.remove(&id);
                    old_state.child_updated.remove(&id);
                }
                {
                    let mut parent_state = parent_state.borrow_mut();
                    parent_state.redraw.insert(id);
                    parent_state.relayout.insert(id);
                    parent_state.child_updated.insert(id);
                }
                *old_state = parent_state;
            }
        }
    }

    pub fn request_redraw(&mut self, id: WidgetID) {
        match self {
            UpdateStateShared::Occupied(parent_state) => {
                let mut parent_state = parent_state.borrow_mut();
                parent_state.redraw.insert(id);
            },
            UpdateStateShared::Vacant => ()
        }
    }

    pub fn request_relayout(&mut self, id: WidgetID) {
        match self {
            UpdateStateShared::Occupied(parent_state) => {
                let mut parent_state = parent_state.borrow_mut();
                parent_state.relayout.insert(id);
            },
            UpdateStateShared::Vacant => ()
        }
    }

    pub fn mark_child_updated(&mut self, id: WidgetID) {
        match self {
            UpdateStateShared::Occupied(parent_state) => {
                let mut parent_state = parent_state.borrow_mut();
                parent_state.child_updated.insert(id);
            },
            UpdateStateShared::Vacant => ()
        }
    }
}
