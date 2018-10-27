use crate::{
    event::{WidgetEvent, FocusChange},
    tree::{Widget, WidgetID, WidgetIdent},
    render::RenderFrame,
    offset_widget::{OffsetWidget, OffsetWidgetTrait},
    widget_stack::{WidgetStack, WidgetPath},
    widget_tree::WidgetTree
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
    pub fn queue_event(&mut self, destination: EventDestination, event: DispatchableEvent) {
        self.events.push_back((destination, event));
    }

    pub fn dispatch_events<Fun, A, F, Root>(
        &mut self,
        widget_stack: &mut WidgetStack<A, F>,
        widget_tree: &mut WidgetTree,
        mut fun: Fun
    )
        where Fun: FnMut(&mut Self, WidgetPath<A, F>, DispatchableEvent),
              A: 'static,
              F: RenderFrame
    {
        while let Some((destination, event)) = self.events.pop_front() {
            let widget = match destination.get_widget(widget_stack, widget_tree) {
                Some(w) => w,
                None => continue
            };
            fun(self, widget, event);
        }
    }
}

impl EventDestination {
    pub fn get_widget<'a, A, F>(&self, widget_stack: &'a mut WidgetStack<A, F>, widget_tree: &mut WidgetTree) -> Option<WidgetPath<'a, A, F>>
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

        let mut active_ident_rev = widget_tree.ident_chain_reversed(target_id)?.cloned().collect::<Vec<_>>();
        // If the widget is where the tree says it is, return it.
        if widget_stack.move_to_path(active_ident_rev.drain(..).rev()).is_some() {
            return Some(widget_stack.top());
        }

        // If the widget wasn't found where the tree said it would be found, update the tree
        // and store the new location.
        let (widget_index, widget_ident);
        match widget_stack.search_for_widget(target_id) {
            Some(widget) => {
                widget_index = widget.index;
                widget_ident = widget.path.last().unwrap().clone();
            },
            // If the widget isn't anywhere to be found, return none.
            None => {
                widget_tree.remove(target_id);
                return None
            }
        }

        if let Some(parent) = widget_stack.parent() {
            widget_tree.insert(parent.widget_tag().widget_id, target_id, widget_index, widget_ident).ok();
        }

        Some(widget_stack.top())
    }
}
