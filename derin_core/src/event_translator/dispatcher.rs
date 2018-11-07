use crate::{
    event::{WidgetEvent, FocusChange},
    tree::{Widget, WidgetID, WidgetIdent},
    render::RenderFrame,
    offset_widget::{OffsetWidget, OffsetWidgetTrait},
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
    SiblingWrapping {
        of: WidgetID,
        delta: isize
    },
    Child {
        of: WidgetID,
        ident: WidgetIdent
    },
    Widget(WidgetID),
}

#[derive(Debug, Clone)]
pub(crate) enum DispatchableEvent {
    WidgetEvent {
        bubble_source: Option<WidgetID>,
        event: WidgetEvent
    },
    FocusEvent {
        source: WidgetID,
        focus_change: FocusChange
    }
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
                None => continue
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
            SiblingWrapping{of, delta} => widget_tree.sibling_wrapping(*of, *delta)?,
            Child{of, ident} => widget_tree.children(*of)?.find(|&(_, data)| ident == &data.ident)?.0,
            Widget(id) => *id
        };

        widget_stack.move_to_widget_with_tree(target_id, widget_tree)
    }
}
