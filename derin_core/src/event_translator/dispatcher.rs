// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    cgmath::Point2,
    event::{FocusChange, FocusSource, WidgetEvent},
    render::Renderer,
    widget::{WidgetId, WidgetIdent},
    widget_traverser::{Relation, WidgetTraverser, OffsetWidgetScanPath},
};
use std::collections::VecDeque;

pub(crate) struct EventDispatcher {
    events: VecDeque<(EventDestination, DispatchableEvent)>
}

#[derive(Debug, Clone)]
pub(crate) enum EventDestination {
    Widget(WidgetId),
    Relation(WidgetId, Relation)
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
        bubble_source: Option<WidgetId>,
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

    pub fn queue_direct_event(&mut self, widget_id: WidgetId, event: WidgetEvent) {
        self.queue_event(
            EventDestination::Widget(widget_id),
            DispatchableEvent::Direct {
                bubble_source: None,
                event,
            }
        )
    }

    pub fn dispatch_events<R>(
        &mut self,
        widget_traverser: &mut WidgetTraverser<R>,
        mut f: impl FnMut(&mut Self, OffsetWidgetScanPath<R>, DispatchableEvent)
    )
        where R: Renderer
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
