use crate::{
    cgmath::Point2,
    event::{FocusChange, FocusSource, WidgetEvent},
    widget::{WidgetID, WidgetIdent},
    render::RenderFrame,
    widget_traverser::{Relation, WidgetTraverser, OffsetWidgetScanPath},
};
use std::collections::VecDeque;

pub(crate) struct EventDispatcher {
    events: VecDeque<(EventDestination, DispatchableEvent)>
}

#[derive(Debug, Clone)]
pub(crate) enum EventDestination {
    Widget(WidgetID),
    Relation(WidgetID, Relation)
}

#[derive(Debug, Clone)]
pub(crate) enum DispatchableEvent {
    MouseMove {
        old_pos: Point2<i32>,
        new_pos: Point2<i32>,
        exiting_from_child: Option<WidgetIdent>,
    },
    GainFocus {
        source: FocusSource,
        change: FocusChange,
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

    pub fn dispatch_events<F>(
        &mut self,
        widget_traverser: &mut WidgetTraverser<F>,
        mut f: impl FnMut(&mut Self, OffsetWidgetScanPath<F>, DispatchableEvent)
    )
        where F: RenderFrame
    {
        while let Some((destination, event)) = self.events.pop_front() {
            let widget_opt = {
                use self::EventDestination::*;
                match destination {
                    Relation(id, relation) => widget_traverser.get_widget_relation(id, relation),
                    Widget(id) => widget_traverser.get_widget(id)
                }
            };

            let widget = match widget_opt {
                Some(w) => w,
                None => continue //TODO: LOG WARNING
            };
            f(self, widget, event);
        }
    }
}
