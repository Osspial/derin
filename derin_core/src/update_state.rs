use crate::{
    tree::WidgetID,
};
use fnv::FnvHashSet;
use std::{
    rc::{Rc, Weak},
    cell::RefCell,
};

/// `UpdateState` handle that gets stored in a `WidgetTag`. Used to access and modify global update
/// state.
#[derive(Debug, Clone)]
pub(crate) enum UpdateStateShared<R = Weak<UpdateStateCell>> {
    Occupied(R),
    Vacant,
}

pub(crate) type UpdateStateCell = RefCell<UpdateState>;

#[derive(Debug)]
pub(crate) struct UpdateState {
    pub redraw: FnvHashSet<WidgetID>,
    pub relayout: FnvHashSet<WidgetID>,
    pub remove_from_tree: FnvHashSet<WidgetID>,
    pub global_update: bool,
}

impl UpdateState {
    pub fn new() -> Rc<UpdateStateCell> {
        Rc::new(
            RefCell::new(UpdateState {
                redraw: FnvHashSet::default(),
                relayout: FnvHashSet::default(),
                remove_from_tree: FnvHashSet::default(),
                global_update: true,
            })
        )
    }

    fn queue_insert_id(&mut self, id: WidgetID) {
        self.redraw.insert(id);
        self.relayout.insert(id);
    }

    pub fn queue_global_update(&mut self) {
        self.global_update = true;
    }

    pub fn reset_global_update(&mut self) {
        self.global_update = false;
    }
}

impl UpdateStateShared {
    pub fn new() -> UpdateStateShared {
        UpdateStateShared::Vacant
    }

    /// Try to upgrade the `Weak` reference to a full `Rc`. If the `Weak` points to something that
    /// no longer exists (because the primary `UpdateState` was dropped), change self to `Vacant`
    /// and return `Vacant`.
    fn upgrade(&mut self) -> UpdateStateShared<Rc<UpdateStateCell>> {
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

    pub fn set_owning_update_state(&mut self, id: WidgetID, parent_state: &Rc<UpdateStateCell>) {
        match self.upgrade() {
            UpdateStateShared::Vacant => {
                parent_state.borrow_mut().queue_insert_id(id);
                *self = UpdateStateShared::Occupied(Rc::downgrade(parent_state))
            },
            UpdateStateShared::Occupied(old_state) => {
                if !Rc::ptr_eq(&old_state, &parent_state) {
                    let mut old_state = old_state.borrow_mut();
                    old_state.redraw.remove(&id);
                    old_state.relayout.remove(&id);
                    old_state.remove_from_tree.insert(id);

                    parent_state.borrow_mut().queue_insert_id(id);
                    *self = UpdateStateShared::Occupied(Rc::downgrade(parent_state));
                }
            }
        }
    }

    pub fn request_redraw(&mut self, id: WidgetID) {
        match self.upgrade() {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.redraw.insert(id);
            },
            // All updates are automatically performed on a fresh insert so we don't need to log that
            // an update was requested.
            UpdateStateShared::Vacant => ()
        }
    }

    pub fn request_relayout(&mut self, id: WidgetID) {
        match self.upgrade() {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.relayout.insert(id);
            },
            // Ditto.
            UpdateStateShared::Vacant => ()
        }
    }

    pub fn remove_from_tree(&mut self, id: WidgetID) {
        match self.upgrade() {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.redraw.remove(&id);
                update_state.relayout.remove(&id);
                update_state.remove_from_tree.insert(id);
            },
            UpdateStateShared::Vacant => ()
        }
    }
}
