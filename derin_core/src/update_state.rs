use crate::{
    LoopFlow,
    tree::WidgetID,
    render::RenderFrame,
    offset_widget::{OffsetWidgetTrait, OffsetWidgetTraitAs},
};
use fnv::FnvHashSet;
use std::{
    rc::{Rc, Weak},
    cell::RefCell,
};

#[derive(Debug, Clone)]
pub(crate) enum UpdateStateShared<R = Weak<UpdateStateBuffered>> {
    Occupied(R),
    Vacant
}

#[derive(Debug)]
pub(crate) struct UpdateStateBuffered {
    pub update_state: RefCell<UpdateState>,
    /// This exists so we can call `set_owning_update_state` inside the child_update loop of
    /// `update_virtual_tree`. See, `set_owning_update_state` needs to modify `update_state` to
    /// insert the new redraw/relayout/child update stuff. However, it can't since `update_state`
    /// was borrowed to get the widget IDs for the loop! The solution is to, when `update_state` is
    /// borrowed, push to this back buffer than process any new events pushed to the back buffer.
    back_buffer: RefCell<Vec<BufferedUpdate>>
}

#[derive(Debug, Clone)]
enum BufferedUpdate {
    Redraw(WidgetID),
    Relayout(WidgetID),
    ChildUpdated(WidgetID),
    Remove(WidgetID)
}

#[derive(Debug)]
pub(crate) struct UpdateState {
    pub redraw: FnvHashSet<WidgetID>,
    pub relayout: FnvHashSet<WidgetID>,
    pub child_updated: FnvHashSet<WidgetID>,
}

impl UpdateStateBuffered {
    pub fn new() -> Rc<UpdateStateBuffered> {
        Rc::new(UpdateStateBuffered {
            update_state: RefCell::new(UpdateState {
                redraw: FnvHashSet::default(),
                relayout: FnvHashSet::default(),
                child_updated: FnvHashSet::default(),
            }),
            back_buffer: RefCell::new(vec![])
        })
    }

    fn queue_insert_id(&self, id: WidgetID) {
        if let Ok(mut parent_state) = self.update_state.try_borrow_mut() {
            parent_state.redraw.insert(id);
            parent_state.relayout.insert(id);
            parent_state.child_updated.insert(id);
        } else {
            let mut buffer = self.back_buffer.borrow_mut();
            buffer.push(BufferedUpdate::Redraw(id));
            buffer.push(BufferedUpdate::Relayout(id));
            buffer.push(BufferedUpdate::ChildUpdated(id));
        }
    }
}

impl UpdateStateShared {
    pub fn new() -> UpdateStateShared {
        UpdateStateShared::Vacant
    }

    /// Try to upgrade the `Weak` reference to a full `Rc`. If the `Weak` points to something that
    /// no longer exists (because the primary `UpdateState` was dropped), change self to `Vacant`
    /// and return `Vacant`.
    fn upgrade(&mut self) -> UpdateStateShared<Rc<UpdateStateBuffered>> {
        match self {
            UpdateStateShared::Vacant => UpdateStateShared::Vacant,
            UpdateStateShared::Occupied(weak) => match weak.upgrade() {
                Some(rc) => UpdateStateShared::Occupied(rc),
                None => {
                    *self = UpdateStateShared::Vacant;
                    UpdateStateShared::Vacant
                }
            }
        }
    }

    pub fn set_owning_update_state(&mut self, id: WidgetID, parent_state: &Rc<UpdateStateBuffered>) {
        match self.upgrade() {
            UpdateStateShared::Vacant => {
                parent_state.queue_insert_id(id);
                *self = UpdateStateShared::Occupied(Rc::downgrade(parent_state))
            },
            UpdateStateShared::Occupied(old_state) => {
                if !Rc::ptr_eq(&old_state, &parent_state) {
                    if let Ok(mut old_state) = old_state.update_state.try_borrow_mut() {
                        old_state.redraw.remove(&id);
                        old_state.relayout.remove(&id);
                        old_state.child_updated.remove(&id);
                    } else {
                        old_state.back_buffer.borrow_mut().push(BufferedUpdate::Remove(id));
                    }

                    parent_state.queue_insert_id(id);
                    *self = UpdateStateShared::Occupied(Rc::downgrade(parent_state));
                }
            }
        }
    }

    pub fn request_redraw(&mut self, id: WidgetID) {
        match self.upgrade() {
            UpdateStateShared::Occupied(parent_state) => {
                if let Ok(mut parent_state) = parent_state.update_state.try_borrow_mut() {
                    parent_state.redraw.insert(id);
                } else {
                    parent_state.back_buffer.borrow_mut().push(BufferedUpdate::Redraw(id));
                }
            },
            // All updates are automatically performed on a fresh insert so we don't need to log that
            // an update was requested.
            UpdateStateShared::Vacant => ()
        }
    }

    pub fn request_relayout(&mut self, id: WidgetID) {
        match self.upgrade() {
            UpdateStateShared::Occupied(parent_state) => {
                if let Ok(mut parent_state) = parent_state.update_state.try_borrow_mut() {
                    parent_state.relayout.insert(id);
                } else {
                    parent_state.back_buffer.borrow_mut().push(BufferedUpdate::Relayout(id));
                }
            },
            // Ditto.
            UpdateStateShared::Vacant => ()
        }
    }

    pub fn mark_child_updated(&mut self, id: WidgetID) {
        match self.upgrade() {
            UpdateStateShared::Occupied(parent_state) => {
                if let Ok(mut parent_state) = parent_state.update_state.try_borrow_mut() {
                    parent_state.child_updated.insert(id);
                } else {
                    parent_state.back_buffer.borrow_mut().push(BufferedUpdate::ChildUpdated(id));
                }
            },
            // Ditto.
            UpdateStateShared::Vacant => ()
        }
    }
}
