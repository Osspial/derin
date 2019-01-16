use crate::tree::WidgetID;
use fnv::{FnvHashMap, FnvHashSet};
use std::{
    any::{Any, TypeId},
    sync::mpsc::{self, Sender, Receiver},
};

pub type WidgetActionFn = Box<FnMut(&mut Any, &Any)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetActionKey {
    widget_type: TypeId,
    action_type: TypeId,
}

impl WidgetActionKey {
    pub fn new<W, A>() -> WidgetActionKey
        where W: 'static + ?Sized,
              A: 'static + ?Sized
    {
        WidgetActionKey {
            widget_type: TypeId::of::<W>(),
            action_type: TypeId::of::<A>(),
        }
    }

    pub fn from_dyn_action<W>(action: &Any) -> WidgetActionKey
        where W: 'static + ?Sized
    {
        WidgetActionKey {
            widget_type: TypeId::of::<W>(),
            action_type: action.get_type_id(),
        }
    }

    pub fn action_type(&self) -> TypeId {
        self.action_type
    }
}

pub struct ActionBus {
    /// Maps action types to widget IDs.
    type_map: FnvHashMap<TypeId, FnvHashSet<WidgetID>>,
    actions_recv: Receiver<Box<Any>>,
    actions_send: Sender<Box<Any>>,
}

impl ActionBus {
    pub fn new() -> ActionBus {
        let (actions_send, actions_recv) = mpsc::channel();
        ActionBus {
            type_map: FnvHashMap::default(),
            actions_recv, actions_send,
        }
    }

    pub fn sender(&self) -> Sender<Box<Any>> {
        self.actions_send.clone()
    }

    pub fn next_action(&mut self) -> Option<(Box<Any>, impl '_ + Iterator<Item=WidgetID>)> {
        while let Ok(action) = self.actions_recv.try_recv() {
            let type_id = action.get_type_id();
            let widget_ids = match self.type_map.get(&type_id).filter(|wids| wids.len() > 0) {
                Some(wids) => wids,
                None => continue
            };

            return Some((action, widget_ids.iter().cloned()))
        }

        None
    }

    pub fn remove_widget(&mut self, widget_id: WidgetID) {
        for wid_vec in self.type_map.values_mut() {
            wid_vec.retain(|id| *id != widget_id);
        }
    }
}
