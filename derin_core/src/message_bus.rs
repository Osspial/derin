// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::widget::WidgetId;
use fnv::{FnvHashMap, FnvHashSet};
use std::{
    any::{Any, TypeId},
    sync::mpsc::{self, Sender, Receiver},
};

pub type Message = Box<Any>;
pub type WidgetMessageFn = Box<FnMut(&mut Any, &Any)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetMessageKey {
    widget_type: TypeId,
    message_type: TypeId,
}

impl WidgetMessageKey {
    pub fn new<W, A>() -> WidgetMessageKey
        where W: 'static + ?Sized,
              A: 'static + ?Sized
    {
        WidgetMessageKey {
            widget_type: TypeId::of::<W>(),
            message_type: TypeId::of::<A>(),
        }
    }

    pub fn from_dyn_message<W>(message: &Any) -> WidgetMessageKey
        where W: 'static + ?Sized
    {
        WidgetMessageKey {
            widget_type: TypeId::of::<W>(),
            message_type: message.type_id(),
        }
    }

    pub fn message_type(&self) -> TypeId {
        self.message_type
    }
}

pub struct MessageBus {
    /// Maps message types to widget IDs.
    type_map: FnvHashMap<TypeId, FnvHashSet<WidgetId>>,
    messages_recv: Receiver<MessageTargeted>,
    messages_send: Sender<MessageTargeted>,
}

#[derive(Debug)]
pub struct MessageTargeted {
    pub message: Message,
    pub target: Option<MessageTarget>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageTarget {
    Widget(WidgetId),
    ParentOf(WidgetId),
    ChildrenOf(WidgetId),
}

impl MessageBus {
    pub fn new() -> MessageBus {
        let (messages_send, messages_recv) = mpsc::channel();
        MessageBus {
            type_map: FnvHashMap::default(),
            messages_recv, messages_send,
        }
    }

    pub fn sender(&self) -> Sender<MessageTargeted> {
        self.messages_send.clone()
    }

    pub fn next_message(&mut self) -> Option<(Message, impl '_ + Iterator<Item=MessageTarget>)> {
        while let Ok(MessageTargeted{message, target}) = self.messages_recv.try_recv() {
            // We have to dereference `message` here because otherwise it would get the TypeId of
            // `Box<Any>`, not the inner `Any`.
            let type_id = (*message).type_id();

            let untargeted_widget_ids = self.type_map.get(&type_id)
                .filter(|wids| wids.len() > 0)
                .filter(|_| target.is_none());

            return Some((
                message,
                target.into_iter().chain(
                    untargeted_widget_ids
                        .into_iter()
                        .flatten()
                        .cloned()
                        .map(|id| MessageTarget::Widget(id))
                )
            ))
        }

        None
    }

    pub fn register_widget_message_type(&mut self, message_type: TypeId, widget_id: WidgetId) {
        self.type_map.entry(message_type).or_default().insert(widget_id);
    }

    pub fn remove_widget(&mut self, widget_id: WidgetId) {
        for wid_vec in self.type_map.values_mut() {
            wid_vec.retain(|id| *id != widget_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MessageA;
    struct MessageB;

    #[test]
    fn next_message() {
        let (a, b, c, d) = (WidgetId::new(), WidgetId::new(), WidgetId::new(), WidgetId::new());

        let mut message_bus = MessageBus::new();

        macro_rules! assert_recv {
            ($($target:expr,)*) => {{
                let mut targets = FnvHashSet::default();
                let mut recieved_targets = FnvHashSet::default();
                $(targets.insert($target);)*

                for target in message_bus.next_message().into_iter().flat_map(|(_, target_iter)| target_iter) {
                    assert!(recieved_targets.insert(target));
                }
                assert_eq!(targets, recieved_targets);
            }}
        }

        assert_eq!(TypeId::of::<MessageA>(), (*(Box::new(MessageA) as Message)).type_id());

        message_bus.register_widget_message_type(TypeId::of::<MessageA>(), a);
        message_bus.register_widget_message_type(TypeId::of::<MessageA>(), b);
        message_bus.register_widget_message_type(TypeId::of::<MessageA>(), c);
        message_bus.register_widget_message_type(TypeId::of::<MessageA>(), d);

        message_bus.register_widget_message_type(TypeId::of::<MessageB>(), a);
        message_bus.register_widget_message_type(TypeId::of::<MessageB>(), b);
        message_bus.register_widget_message_type(TypeId::of::<MessageB>(), c);

        message_bus.messages_send.send(MessageTargeted {
            message: Box::new(MessageA),
            target: None
        });
        assert_recv!(
            MessageTarget::Widget(a),
            MessageTarget::Widget(b),
            MessageTarget::Widget(c),
            MessageTarget::Widget(d),
        );

        message_bus.messages_send.send(MessageTargeted {
            message: Box::new(MessageB),
            target: None
        });
        assert_recv!(
            MessageTarget::Widget(a),
            MessageTarget::Widget(b),
            MessageTarget::Widget(c),
        );

        message_bus.messages_send.send(MessageTargeted {
            message: Box::new(MessageA),
            target: Some(MessageTarget::Widget(a))
        });
        assert_recv!(
            MessageTarget::Widget(a),
        );

        message_bus.messages_send.send(MessageTargeted {
            message: Box::new(MessageA),
            target: Some(MessageTarget::ChildrenOf(a))
        });
        assert_recv!(
            MessageTarget::ChildrenOf(a),
        );
    }
}
