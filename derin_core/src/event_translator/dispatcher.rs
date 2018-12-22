use crate::{
    cgmath::Point2,
    event::{WidgetEvent, FocusChange},
    tree::{WidgetID, WidgetIdent},
    render::RenderFrame,
    widget_stack::{WidgetStack, WidgetPath},
    virtual_widget_tree::VirtualWidgetTree
};
use std::collections::VecDeque;

pub(crate) struct EventDispatcher {
    events: VecDeque<(EventDestination, DispatchableEvent)>
}

#[derive(Debug, Clone)]
pub(crate) enum EventDestination {
    Parent{of: WidgetID},
    Sibling {
        of: WidgetID,
        delta: isize
    },
    ChildIdent {
        of: WidgetID,
        ident: WidgetIdent
    },
    ChildIndex {
        of: WidgetID,
        index: usize
    },
    Widget(WidgetID),
}

#[derive(Debug, Clone)]
pub(crate) enum DispatchableEvent {
    MouseMove {
        old_pos: Point2<i32>,
        new_pos: Point2<i32>,
        exiting_from_child: Option<WidgetIdent>,
    },
    Direct {
        bubble_source: Option<WidgetID>,
        event: WidgetEvent,
    },
}

impl EventDispatcher {
    pub fn new() -> EventDispatcher {
        EventDispatcher {
            events: VecDeque::new()
        }
    }

    pub fn queue_event(&mut self, destination: EventDestination, event: DispatchableEvent) {
        self.events.push_back((destination, event));
    }

    pub fn queue_direct_event(&mut self, widget_id: WidgetID, event: WidgetEvent) {
        self.queue_event(
            EventDestination::Widget(widget_id),
            DispatchableEvent::Direct {
                bubble_source: None,
                event,
            }
        )
    }

    pub fn dispatch_events<A, F>(
        &mut self,
        widget_stack: &mut WidgetStack<A, F>,
        widget_tree: &mut VirtualWidgetTree,
        mut f: impl FnMut(&mut Self, WidgetPath<A, F>, DispatchableEvent)
    )
        where A: 'static,
              F: RenderFrame
    {
        while let Some((destination, event)) = self.events.pop_front() {
            let widget = match destination.get_widget(widget_stack, widget_tree) {
                Some(w) => w,
                None => continue //TODO: LOG WARNING
            };
            f(self, widget, event);
        }
    }
}

impl EventDestination {
    pub fn get_widget<'a, A, F>(&self, widget_stack: &'a mut WidgetStack<A, F>, widget_tree: &mut VirtualWidgetTree) -> Option<WidgetPath<'a, A, F>>
        where A: 'static,
              F: RenderFrame
    {
        use self::EventDestination::*;
        let target_id = match self {
            Parent{of} => widget_tree.parent(*of).ok()?,
            Sibling{of, delta} => widget_tree.sibling(*of, *delta).ok()?,
            ChildIdent{of, ident} => widget_tree.children(*of)?.find(|&(_, data)| ident == &data.ident)?.0,
            ChildIndex{of, index} => widget_tree.child_from_start(*of, *index).ok()?,
            Widget(id) => *id
        };

        widget_stack.move_to_widget_with_tree(target_id, widget_tree)
    }
}
