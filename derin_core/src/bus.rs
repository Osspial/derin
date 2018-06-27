use std::{
    rc::{Rc, Weak},
    cell::{RefCell, UnsafeCell},
    collections::VecDeque,
};

pub struct BusTerminal<T, ID: Copy=()> {
    state: UnsafeCell<BusTerminalEnum<T, ID>>,
    id: ID
}

enum BusTerminalEnum<T, ID> {
    Unattached(Vec<T>),
    Attached(Weak<Shared<T, ID>>)
}

pub struct BusHub<T, ID=()>(Rc<Shared<T, ID>>);

type Shared<T, ID> = RefCell<BusHubShared<T, ID>>;
struct BusHubShared<T, ID> {
    items: VecDeque<(T, ID)>,
    bus_events: VecDeque<BusEvent<ID>>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BusEvent<ID> {
    Attach(ID),
    Detach(ID)
}


impl<T, ID: Copy> BusTerminal<T, ID> {
    pub fn new(id: ID) -> BusTerminal<T, ID> {
        BusTerminal {
            state: UnsafeCell::new(BusTerminalEnum::Unattached(vec![])),
            id
        }
    }

    #[inline(always)]
    pub fn id(&self) -> ID {
        self.id
    }

    pub fn send(&self, item: T) {
        let state = unsafe{ &mut *self.state.get() };

        match *state {
            BusTerminalEnum::Attached(ref mut shared_weak) => match shared_weak.upgrade() {
                Some(shared) => shared.borrow_mut().items.push_back((item, self.id)),
                None => *state = BusTerminalEnum::Unattached(vec![item])
            },
            BusTerminalEnum::Unattached(ref mut vec) => vec.push(item),
        }
    }

    pub fn ask<Q>(&self, _query: &mut Q) {
        unimplemented!()
    }

    pub fn attach(&self, receiver: &mut BusHub<T, ID>) {
        let state = unsafe{ &mut *self.state.get() };

        match state {
            BusTerminalEnum::Unattached(ref mut vec) => {
                let self_id = self.id;

                let mut shared = receiver.0.borrow_mut();
                shared.items.extend(vec.drain(..).map(|item| (item, self_id)));
                shared.bus_events.push_back(BusEvent::Attach(self.id));
            },
            BusTerminalEnum::Attached(ref shared_weak) => {
                match shared_weak.upgrade() {
                    Some(shared) => {
                        let cur_shared_ptr = &*shared as *const Shared<_, _>;
                        let new_shared_ptr = &*receiver.0 as *const Shared<_, _>;

                        // Only send the detach/attach events if we're switching to a new receiver. We check
                        // if we're switching to a new receiver by comparing memory locations; because the
                        // shared data is stored in a Rc, the memory location of the shared data will be
                        // different if we're using a different receiver.
                        if cur_shared_ptr != new_shared_ptr {
                            shared.borrow_mut().bus_events.push_back(BusEvent::Detach(self.id));
                            receiver.0.borrow_mut().bus_events.push_back(BusEvent::Attach(self.id));
                        }
                    },
                    None => receiver.0.borrow_mut().bus_events.push_back(BusEvent::Attach(self.id))
                }
            }
        }

        *state = BusTerminalEnum::Attached(Rc::downgrade(&receiver.0));
    }
}

impl<T, ID: Copy> Drop for BusTerminal<T, ID> {
    fn drop(&mut self) {
        let state = unsafe{ &mut *self.state.get() };

        // Notify the receiver that we're detaching from it, if the receiver still exists.
        if let BusTerminalEnum::Attached(ref shared_weak) = state {
            if let Some(shared) = shared_weak.upgrade() {
                shared.borrow_mut().bus_events.push_back(BusEvent::Detach(self.id));
            }
        }
    }
}

impl<T, ID: Copy> BusHub<T, ID> {
    pub fn new() -> BusHub<T, ID> {
        BusHub(Rc::new(RefCell::new(BusHubShared {
            items: VecDeque::new(),
            bus_events: VecDeque::new()
        })))
    }

    #[must_use]
    #[inline(always)]
    pub fn recv_item(&mut self) -> Option<(T, ID)> {
        self.0.borrow_mut().items.pop_front()
    }

    #[must_use]
    #[inline(always)]
    pub fn recv_attachment(&mut self) -> Option<BusEvent<ID>> {
        self.0.borrow_mut().bus_events.pop_front()
    }
}
