use crate::{
    message_bus::{Message, MessageTarget, MessageTargeted, MessageBus},
    cgmath::Point2,
    tree::WidgetID,
};
use derin_common_types::cursor::CursorIcon;
use fnv::FnvHashSet;
use std::{
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
    buffered_messages: Vec<MessageTargeted>,
}

pub(crate) type UpdateStateCell = RefCell<UpdateState>;

#[derive(Debug)]
pub(crate) struct UpdateState {
    pub redraw: FnvHashSet<WidgetID>,
    pub relayout: FnvHashSet<WidgetID>,
    pub update_timers: FnvHashSet<WidgetID>,
    pub update_messages: FnvHashSet<WidgetID>,
    pub remove_from_tree: FnvHashSet<WidgetID>,
    pub set_cursor_icon: Option<CursorIcon>,
    pub set_cursor_pos: Option<(WidgetID, Point2<i32>)>,
    pub message_sender: Sender<MessageTargeted>,
    pub global_update: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateError {
    NoRootWidget,
}

impl UpdateState {
    pub fn new(message_bus: &MessageBus) -> Rc<UpdateStateCell> {
        Rc::new(
            RefCell::new(UpdateState {
                redraw: FnvHashSet::default(),
                relayout: FnvHashSet::default(),
                update_timers: FnvHashSet::default(),
                update_messages: FnvHashSet::default(),
                remove_from_tree: FnvHashSet::default(),
                set_cursor_icon: None,
                set_cursor_pos: None,
                message_sender: message_bus.sender(),
                global_update: true,
            })
        )
    }

    fn queue_insert_id(&mut self, id: WidgetID) {
        self.redraw.insert(id);
        self.relayout.insert(id);
        self.update_timers.insert(id);
        self.update_messages.insert(id);
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
            buffered_messages: Vec::new(),
        })
    }

    /// Try to upgrade the `Weak` reference to a full `Rc`. If the `Weak` points to something that
    /// no longer exists (because the primary `UpdateState` was dropped), change self to `Vacant`
    /// and return `Vacant`.
    fn upgrade<R>(&mut self, f: impl FnOnce(&mut UpdateStateShared<Rc<UpdateStateCell>>) -> R) -> R {
        let ret: R;

        match self {
            UpdateStateShared::Vacant(ref mut v) => {
                let mut swap_vacant = UpdateStateVacant::default();
                mem::swap(v, &mut swap_vacant);
                let mut uss = UpdateStateShared::Vacant(swap_vacant);
                ret = f(&mut uss);

                *self = match uss {
                    UpdateStateShared::Occupied(state) => UpdateStateShared::Occupied(Rc::downgrade(&state)),
                    UpdateStateShared::Vacant(v) => UpdateStateShared::Vacant(v)
                };
            },
            UpdateStateShared::Occupied(weak) => match weak.upgrade() {
                Some(rc) => ret = f(&mut UpdateStateShared::Occupied(rc)),
                None => {
                    *self = UpdateStateShared::new();
                    ret = self.upgrade(f);
                }
            }
        }

        ret
    }

    pub fn set_owning_update_state(&mut self, id: WidgetID, parent_state: &Rc<UpdateStateCell>) {
        self.upgrade(|this| match this {
            UpdateStateShared::Vacant(vacant) => {
                {
                    let mut parent_state = parent_state.borrow_mut();
                    parent_state.queue_insert_id(id);
                    for message in vacant.buffered_messages.drain(..) {
                        parent_state.message_sender.send(message).ok();
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

    pub fn request_update_messages(&mut self, id: WidgetID) {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.update_messages.insert(id);
            },
            // Ditto.
            UpdateStateShared::Vacant(_) => ()
        });
    }

    pub fn send_message<A: 'static>(&mut self, message: A, target: Option<MessageTarget>) {
        let message = MessageTargeted {
            message: Box::new(message) as Message,
            target,
        };
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let update_state = update_state.borrow();
                update_state.message_sender.send(message).ok();
            },
            UpdateStateShared::Vacant(vacant) => {
                vacant.buffered_messages.push(message);
            }
        });
    }

    pub fn request_set_cursor_pos(&mut self, id: WidgetID, pos: Point2<i32>) -> Result<(), UpdateError> {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.set_cursor_pos = Some((id, pos));
                Ok(())
            },
            UpdateStateShared::Vacant(_) => Err(UpdateError::NoRootWidget)
        })
    }

    pub fn request_set_cursor_icon(&mut self, icon: CursorIcon) -> Result<(), UpdateError> {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.set_cursor_icon = Some(icon);
                Ok(())
            },
            UpdateStateShared::Vacant(_) => Err(UpdateError::NoRootWidget)
        })
    }

    pub fn remove_from_tree(&mut self, id: WidgetID) {
        self.upgrade(|this| match this {
            UpdateStateShared::Occupied(update_state) => {
                let mut update_state = update_state.borrow_mut();
                update_state.redraw.remove(&id);
                update_state.relayout.remove(&id);
                update_state.update_timers.remove(&id);
                update_state.update_messages.remove(&id);
                update_state.remove_from_tree.insert(id);
            },
            UpdateStateShared::Vacant(_) => ()
        });
    }
}
