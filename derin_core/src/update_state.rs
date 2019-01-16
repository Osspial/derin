use crate::{
    action_bus::ActionBus,
    tree::WidgetID,
};
use fnv::FnvHashSet;
use std::{
    any::Any,
    mem,
    rc::{Rc, Weak},
    sync::mpsc::Sender,
    cell::RefCell,
};

/// `UpdateState` handle that gets stored in a `WidgetTag`. Used to access and modify global update
/// state.
#[derive(Debug)]
pub(crate) enum UpdateStateShared<R = Weak<UpdateStateCell>> {
    Occupied(R),
    Vacant(UpdateStateVacant)
}

#[derive(Debug, Default)]
pub(crate) struct UpdateStateVacant {
    buffered_actions: Vec<Box<Any>>,
}

pub(crate) type UpdateStateCell = RefCell<UpdateState>;

#[derive(Debug)]
pub(crate) struct UpdateState {
    pub redraw: FnvHashSet<WidgetID>,
    pub relayout: FnvHashSet<WidgetID>,
    pub update_timers: FnvHashSet<WidgetID>,
    pub update_actions: FnvHashSet<WidgetID>,
    pub remove_from_tree: FnvHashSet<WidgetID>,
    pub action_sender: Sender<Box<Any>>,
    pub global_update: bool,
}

impl UpdateState {
    pub fn new(action_bus: &ActionBus) -> Rc<UpdateStateCell> {
        Rc::new(
            RefCell::new(UpdateState {
                redraw: FnvHashSet::default(),
                relayout: FnvHashSet::default(),
                update_timers: FnvHashSet::default(),
                update_actions: FnvHashSet::default(),
                remove_from_tree: FnvHashSet::default(),
                action_sender: action_bus.sender(),
                global_update: true,
            })
        )
    }

    fn queue_insert_id(&mut self, id: WidgetID) {
        self.redraw.insert(id);
        self.relayout.insert(id);
        self.update_timers.insert(id);
        self.update_actions.insert(id);
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
        UpdateStateShared::Vacant(UpdateStateVacant {
            buffered_actions: Vec::new(),
        })
    }

    /// Try to upgrade the `Weak` reference to a full `Rc`. If the `Weak` points to something that
    /// no longer exists (because the primary `UpdateState` was dropped), change self to `Vacant`
    /// and return `Vacant`.
    fn upgrade(&mut self, f: impl FnOnce(&mut UpdateStateShared<Rc<UpdateStateCell>>)) {
        match self {
            UpdateStateShared::Vacant(ref mut v) => {
                let mut swap_vacant = UpdateStateVacant::default();
                mem::swap(v, &mut swap_vacant);
                let mut uss = UpdateStateShared::Vacant(swap_vacant);
                f(&mut uss);

                *self = match uss {
                    UpdateStateShared::Occupied(state) => UpdateStateShared::Occupied(Rc::downgrade(&state)),
                    UpdateStateShared::Vacant(v) => UpdateStateShared::Vacant(v)
                };
            },
            UpdateStateShared::Occupied(weak) => match weak.upgrade() {
                Some(rc) => f(&mut UpdateStateShared::Occupied(rc)),
                None => {
                    *self = UpdateStateShared::new();
                    self.upgrade(f);
                }
            }
        }
    }

    pub fn set_owning_update_state(&mut self, id: WidgetID, parent_state: &Rc<UpdateStateCell>) {
        self.upgrade(|this| match this {
            UpdateStateShared::Vacant(vacant) => {
                {
                    let mut parent_state = parent_state.borrow_mut();
                    parent_state.queue_insert_id(id);
                    for action in vacant.buffered_actions.drain(..) {
                        parent_state.action_sender.send(action).ok();
                    }
                }

                *this = UpdateStateShared::Occupied(parent_state.clone())
            },
            UpdateStateShared::Occupied(old_state) => {
                if !Rc::ptr_eq(&old_state, &parent_state) {
                    {
                        let mut old_state = old_state.borrow_mut();
                        old_state.redraw.remove(&id);
                        old_state.relayout.remove(&id);
                        old_state.remove_from_tree.insert(id);
                    }

                    parent_state.borrow_mut().queue_insert_id(id);
                    *this = UpdateStateShared::Occupied(parent_state.clone());
                }
            }
        });
    }

    pub fn request_redraw(&mut self, id: WidgetID) {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.redraw.insert(id);
            },
            // All updates are automatically performed on a fresh insert so we don't need to log that
            // an update was requested.
            UpdateStateShared::Vacant(_) => ()
        });
    }

    pub fn request_relayout(&mut self, id: WidgetID) {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.relayout.insert(id);
            },
            // Ditto.
            UpdateStateShared::Vacant(_) => ()
        });
    }

    pub fn request_update_timers(&mut self, id: WidgetID) {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.update_timers.insert(id);
            },
            // Ditto.
            UpdateStateShared::Vacant(_) => ()
        });
    }

    pub fn request_update_actions(&mut self, id: WidgetID) {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.update_actions.insert(id);
            },
            // Ditto.
            UpdateStateShared::Vacant(_) => ()
        });
    }

    pub fn broadcast_action<A: 'static>(&mut self, action: A) {
        let action = Box::new(action) as Box<Any>;
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let update_state = update_state.borrow();
                update_state.action_sender.send(action).ok();
            },
            UpdateStateShared::Vacant(vacant) => {
                vacant.buffered_actions.push(action);
            }
        });
    }

    pub fn remove_from_tree(&mut self, id: WidgetID) {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.redraw.remove(&id);
                update_state.relayout.remove(&id);
                update_state.update_timers.remove(&id);
                update_state.update_actions.remove(&id);
                update_state.remove_from_tree.insert(id);
            },
            UpdateStateShared::Vacant(_) => ()
        });
    }
}
