use crate::tree::WidgetID;
use fnv::{FnvHashMap, FnvHashSet};
use std::{
    any::{Any, TypeId},
    sync::mpsc::{self, Sender, Receiver},
};

pub type Action = Box<Any>;
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
    actions_recv: Receiver<ActionTargeted>,
    actions_send: Sender<ActionTargeted>,
}

#[derive(Debug)]
pub struct ActionTargeted {
    pub action: Action,
    pub target: Option<ActionTarget>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionTarget {
    Widget(WidgetID),
    ParentOf(WidgetID),
    ChildrenOf(WidgetID),
}

impl ActionBus {
    pub fn new() -> ActionBus {
        let (actions_send, actions_recv) = mpsc::channel();
        ActionBus {
            type_map: FnvHashMap::default(),
            actions_recv, actions_send,
        }
    }

    pub fn sender(&self) -> Sender<ActionTargeted> {
        self.actions_send.clone()
    }

    pub fn next_action(&mut self) -> Option<(Action, impl '_ + Iterator<Item=ActionTarget>)> {
        while let Ok(ActionTargeted{action, target}) = self.actions_recv.try_recv() {
            // We have to dereference `action` here because otherwise it would get the TypeId of
            // `Box<Any>`, not the inner `Any`.
            let type_id = (*action).get_type_id();

            let untargeted_widget_ids = self.type_map.get(&type_id)
                .filter(|wids| wids.len() > 0)
                .filter(|_| target.is_none());

            return Some((
                action,
                target.into_iter().chain(
                    untargeted_widget_ids
                        .into_iter()
                        .flatten()
                        .cloned()
                        .map(|id| ActionTarget::Widget(id))
                )
            ))
        }

        None
    }

    pub fn register_widget_action_type(&mut self, action_type: TypeId, widget_id: WidgetID) {
        self.type_map.entry(action_type).or_default().insert(widget_id);
    }

    pub fn remove_widget(&mut self, widget_id: WidgetID) {
        for wid_vec in self.type_map.values_mut() {
            wid_vec.retain(|id| *id != widget_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ActionA;
    struct ActionB;

    #[test]
    fn next_action() {
        let (a, b, c, d) = (WidgetID::new(), WidgetID::new(), WidgetID::new(), WidgetID::new());

        let mut action_bus = ActionBus::new();

        macro_rules! assert_recv {
            ($($target:expr,)*) => {{
                let mut targets = FnvHashSet::default();
                let mut recieved_targets = FnvHashSet::default();
                $(targets.insert($target);)*

                for target in action_bus.next_action().into_iter().flat_map(|(_, target_iter)| target_iter) {
                    assert!(recieved_targets.insert(target));
                }
                assert_eq!(targets, recieved_targets);
            }}
        }

        assert_eq!(TypeId::of::<ActionA>(), (*(Box::new(ActionA) as Action)).get_type_id());

        action_bus.register_widget_action_type(TypeId::of::<ActionA>(), a);
        action_bus.register_widget_action_type(TypeId::of::<ActionA>(), b);
        action_bus.register_widget_action_type(TypeId::of::<ActionA>(), c);
        action_bus.register_widget_action_type(TypeId::of::<ActionA>(), d);

        action_bus.register_widget_action_type(TypeId::of::<ActionB>(), a);
        action_bus.register_widget_action_type(TypeId::of::<ActionB>(), b);
        action_bus.register_widget_action_type(TypeId::of::<ActionB>(), c);

        action_bus.actions_send.send(ActionTargeted {
            action: Box::new(ActionA),
            target: None
        });
        assert_recv!(
            ActionTarget::Widget(a),
            ActionTarget::Widget(b),
            ActionTarget::Widget(c),
            ActionTarget::Widget(d),
        );

        action_bus.actions_send.send(ActionTargeted {
            action: Box::new(ActionB),
            target: None
        });
        assert_recv!(
            ActionTarget::Widget(a),
            ActionTarget::Widget(b),
            ActionTarget::Widget(c),
        );

        action_bus.actions_send.send(ActionTargeted {
            action: Box::new(ActionA),
            target: Some(ActionTarget::Widget(a))
        });
        assert_recv!(
            ActionTarget::Widget(a),
        );

        action_bus.actions_send.send(ActionTargeted {
            action: Box::new(ActionA),
            target: Some(ActionTarget::ChildrenOf(a))
        });
        assert_recv!(
            ActionTarget::ChildrenOf(a),
        );
    }
}
