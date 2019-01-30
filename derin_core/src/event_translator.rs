// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod dispatcher;

use crate::{
    WindowEvent, InputState, LoopFlow,
    cgmath::{Vector2},
    event::{EventOps, FocusChange, FocusSource, MouseHoverChange, WidgetEvent, WidgetEventSourced},
    render::RenderFrame,
    widget_traverser::{Relation, WidgetTraverser, OffsetWidgetScanPath},
    update_state::{UpdateStateCell},
    offset_widget::OffsetWidget,
};
use self::dispatcher::{EventDispatcher, EventDestination, DispatchableEvent};
use cgmath_geometry::rect::{GeoBox, BoundBox};
use std::rc::Rc;

pub(crate) struct EventTranslator
{
    inner: TranslatorInner
}

pub(crate) struct TranslatorActive<'a, 'b, F>
    where F: RenderFrame + 'static
{
    widget_traverser: &'a mut WidgetTraverser<'b, F>,
    inner: &'a mut TranslatorInner,
    input_state: &'a mut InputState,
    update_state: Rc<UpdateStateCell>,
}

struct TranslatorInner {
    event_dispatcher: EventDispatcher,
}

impl EventTranslator
{
    pub fn new() -> EventTranslator {
        EventTranslator {
            inner: TranslatorInner {
                event_dispatcher: EventDispatcher::new(),
            },
        }
    }

    pub fn with_data<'a, 'b, F: RenderFrame>(
        &'a mut self,
        widget_traverser: &'a mut WidgetTraverser<'b, F>,
        input_state: &'a mut InputState,
        update_state: Rc<UpdateStateCell>,
    ) -> TranslatorActive<'a, 'b, F> {
        TranslatorActive {
            widget_traverser,
            inner: &mut self.inner,
            input_state,
            update_state,
        }
    }
}

impl<F> TranslatorActive<'_, '_, F>
    where F: RenderFrame + 'static
{
    pub fn translate_window_event(&mut self, window_event: WindowEvent) {
        use self::WindowEvent::*;

        let TranslatorActive {
            ref mut widget_traverser,
            ref mut inner,
            input_state,
            ref update_state,
        } = self;
        let TranslatorInner {
            ref mut event_dispatcher,
        } = inner;

        let root_id = widget_traverser.root_id();
        let mut root_widget_rect = || widget_traverser.get_widget(root_id).unwrap().widget.rect();
        let mut project_to_outside_root = |point| {
            let root_rect = root_widget_rect();
            let border_point = root_rect.nearest_points(point).next().unwrap();

            let mut diff = Vector2::new(0, 0);
            if border_point != point {
                diff = (border_point - point).map(|i| i.signum());
            } else {
                if border_point.x == root_rect.min.x {
                    diff.x = -1;
                }
                if border_point.x == root_rect.max.x {
                    diff.x = 1;
                }
                if border_point.y == root_rect.min.y {
                    diff.y = -1;
                }
                if border_point.y == root_rect.max.y {
                    diff.y = 1;
                }
            }

            border_point + diff
        };

        let mouse_event_widget_iter =
            input_state.mouse_buttons_down
                .clone().into_iter()
                .map(|d| d.widget_id)
                .chain(input_state.focused_widget);

        let _: Option<()> =
        match window_event {
            MouseMove(new_pos) => try {
                let old_pos = input_state.mouse_pos
                    .unwrap_or_else(|| project_to_outside_root(new_pos));
                input_state.mouse_pos = Some(new_pos);

                let hover_widget_id = input_state.mouse_hover_widget
                    .unwrap_or(widget_traverser.root_id());

                event_dispatcher.queue_event(
                    EventDestination::Widget(hover_widget_id),
                    DispatchableEvent::MouseMove {
                        old_pos, new_pos,
                        exiting_from_child: None,
                    }
                );

                for widget_id in mouse_event_widget_iter.filter(|id| *id != hover_widget_id) {
                    event_dispatcher.queue_direct_event(
                        widget_id,
                        WidgetEvent::MouseMove {
                            old_pos, new_pos,
                            in_widget: false,
                            hover_change: None,
                        },
                    );
                }
            },
            MouseEnter => None,
            // We convert `MouseExit` events to `MouseMove` events so that we don't have to duplicate
            // code.
            MouseExit => {
                if let Some(old_pos) = input_state.mouse_pos {
                    let new_pos = project_to_outside_root(old_pos);

                    self.translate_window_event(WindowEvent::MouseMove(new_pos));
                    if self.input_state.mouse_buttons_down.len() == 0 {
                        self.input_state.mouse_pos = None;
                    }

                    return;
                }

                None
            }
            MouseDown(mouse_button) => try {
                let mouse_pos = input_state.mouse_pos?;
                let hover_widget_id = input_state.mouse_hover_widget?;

                event_dispatcher.queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseDown {
                        pos: mouse_pos,
                        in_widget: true,
                        button: mouse_button
                    },
                );
                input_state.mouse_buttons_down.push_button(mouse_button, mouse_pos, hover_widget_id);

                for widget_id in mouse_event_widget_iter.filter(|id| *id != hover_widget_id) {
                    event_dispatcher.queue_direct_event(
                        widget_id,
                        WidgetEvent::MouseDown {
                            pos: mouse_pos,
                            in_widget: false,
                            button: mouse_button
                        },
                    );
                }
            },
            MouseUp(mouse_button) => try {
                let mouse_pos = input_state.mouse_pos?;
                let mouse_down = input_state.mouse_buttons_down.contains(mouse_button)?;
                let hover_widget_id = input_state.mouse_hover_widget
                    .unwrap_or(widget_traverser.root_id());

                event_dispatcher.queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseUp {
                        pos: mouse_pos,
                        down_pos: mouse_down.mouse_down.down_pos,
                        pressed_in_widget: mouse_down.widget_id == hover_widget_id,
                        in_widget: true,
                        button: mouse_button
                    },
                );
                input_state.mouse_buttons_down.release_button(mouse_button);

                for widget_id in mouse_event_widget_iter.filter(|id| *id != hover_widget_id) {
                    event_dispatcher.queue_direct_event(
                        widget_id,
                        WidgetEvent::MouseUp {
                            pos: mouse_pos,
                            down_pos: mouse_down.mouse_down.down_pos,
                            pressed_in_widget: mouse_down.widget_id == widget_id,
                            in_widget: false,
                            button: mouse_button
                        },
                    );
                }
            },
            MouseScrollLines(dir) => try {
                let hover_widget_id = input_state.mouse_hover_widget?;
                event_dispatcher.queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseScrollLines{dir, in_widget: true},
                );

                for widget_id in mouse_event_widget_iter.filter(|id| *id != hover_widget_id) {
                    event_dispatcher.queue_direct_event(
                        widget_id,
                        WidgetEvent::MouseScrollLines {dir, in_widget: false},
                    );
                }
            },
            MouseScrollPx(dir) => try {
                let hover_widget_id = input_state.mouse_hover_widget?;
                event_dispatcher.queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseScrollPx{dir, in_widget: true},
                );

                for widget_id in mouse_event_widget_iter.filter(|id| *id != hover_widget_id) {
                    event_dispatcher.queue_direct_event(
                        widget_id,
                        WidgetEvent::MouseScrollPx {dir, in_widget: false},
                    );
                }
            },
            WindowResize(size) => try {
                widget_traverser.get_widget(root_id).unwrap().widget.set_rect(BoundBox::new2(0, 0, size.dims.x as i32, size.dims.y as i32));
                update_state.borrow_mut().queue_global_update();
            },
            KeyDown(key) => try {
                if !input_state.keys_down.contains(&key) {
                    input_state.keys_down.push(key);
                    match input_state.focused_widget {
                        Some(widget) => event_dispatcher.queue_direct_event(
                            widget,
                            WidgetEvent::KeyDown(key, input_state.modifiers),
                        ),
                        None => println!("dispatch to universal fallthrough")
                    }
                }
            },
            KeyUp(key) => try {
                if crate::vec_remove_element(&mut input_state.keys_down, &key).is_some() {
                    match input_state.focused_widget {
                        Some(widget) => event_dispatcher.queue_direct_event(
                            widget,
                            WidgetEvent::KeyUp(key, input_state.modifiers),
                        ),
                        None => println!("dispatch to universal fallthrough")
                    }
                }
            },
            Char(c) => try {
                match input_state.focused_widget {
                    Some(widget) => event_dispatcher.queue_direct_event(
                        widget,
                        WidgetEvent::Char(c),
                    ),
                    None => println!("dispatch to universal fallthrough")
                }
            },
            Timer => None, // The timers will be handled in FrameEventProcessor::finish
            Redraw => try {
                update_state.borrow_mut().queue_global_update();
            },
        };

        event_dispatcher.dispatch_events(
            widget_traverser,
            |event_dispatcher, OffsetWidgetScanPath{mut widget, path, widget_id, index}, event| {
                let widget_ident = path.last().unwrap();

                // Helper function that takes the `EventOps` generated by `on_widget_event`, updates
                // the input state, and queues more events as necessary.
                let mut perform_event_ops = |ops| {
                    let EventOps {
                        focus,
                        bubble,
                    } = ops;
                    if let Some(focus) = focus {
                        let of = widget_id;
                        let ident = widget_ident.clone();
                        let destination_source_opt = {
                            match focus.clone() {
                                FocusChange::Next => Some((
                                    EventDestination::Relation(of, Relation::Sibling(1)),
                                    FocusSource::Sibling{ident, delta: -1}
                                )),
                                FocusChange::Prev => Some((
                                    EventDestination::Relation(of, Relation::Sibling(-1)),
                                    FocusSource::Sibling{ident, delta: 1}
                                )),
                                FocusChange::Parent => Some((
                                    EventDestination::Relation(of, Relation::Parent),
                                    FocusSource::Child{ident, index}
                                )),
                                FocusChange::ChildIdent(ident) => Some((
                                    EventDestination::Relation(of, Relation::ChildIdent(ident)),
                                    FocusSource::Parent
                                )),
                                FocusChange::ChildIndex(index) => Some((
                                    EventDestination::Relation(of, Relation::ChildIndex(index)),
                                    FocusSource::Parent
                                )),
                                FocusChange::Take => Some((
                                    EventDestination::Widget(widget_id),
                                    FocusSource::This
                                )),
                                FocusChange::Remove => None
                            }
                        };

                        if let Some((destination, source)) = destination_source_opt {
                            event_dispatcher.queue_event(
                                destination,
                                DispatchableEvent::GainFocus{source, change: focus}
                            );
                        } else if focus == FocusChange::Remove {
                            event_dispatcher.queue_direct_event(
                                widget_id,
                                WidgetEvent::LoseFocus
                            );
                        }
                    }
                };

                match event {
                    // We handle `MouseMove` events differently than all other events because
                    // `MouseMove` can trigger other `MouseMove`s if the mouse moves into a child
                    // or parent widget.
                    DispatchableEvent::MouseMove{old_pos, new_pos, exiting_from_child} => {
                        let widget_rect = match widget.rect_clipped() {
                            Some(rect) => rect,
                            None => return
                        };
                        let (contains_new, contains_old) = (widget_rect.contains(new_pos), widget_rect.contains(old_pos));

                        let mut send_exiting_from_child = |widget: &mut OffsetWidget<'_, F>, in_widget| {
                            if let Some(child_ident) = exiting_from_child.clone() {
                                perform_event_ops(widget.on_widget_event(
                                    WidgetEventSourced::This(WidgetEvent::MouseMove {
                                        old_pos, new_pos,
                                        in_widget,
                                        hover_change: Some(MouseHoverChange::ExitChild(child_ident)),
                                    }),
                                    input_state,
                                ));
                            }
                        };

                        match contains_new {
                            true => {
                                let mut enter_child_opt = None;
                                widget.children_mut(|child_summary| {
                                    if child_summary.widget.rect_clipped().map(|r| r.contains(new_pos)).unwrap_or(false) {
                                        enter_child_opt = Some((child_summary.widget.widget_id(), child_summary.ident));
                                        LoopFlow::Break
                                    } else {
                                        LoopFlow::Continue
                                    }
                                });

                                send_exiting_from_child(&mut widget, contains_new && enter_child_opt.is_none());

                                if !contains_old {
                                    perform_event_ops(widget.on_widget_event(
                                        WidgetEventSourced::This(WidgetEvent::MouseMove {
                                            old_pos, new_pos,
                                            in_widget: enter_child_opt.is_none(),
                                            hover_change: Some(MouseHoverChange::Enter)
                                        }),
                                        input_state,
                                    ));
                                }

                                match enter_child_opt {
                                    Some((enter_child_id, enter_child_ident)) => {
                                        perform_event_ops(widget.on_widget_event(
                                            WidgetEventSourced::This(WidgetEvent::MouseMove {
                                                old_pos, new_pos,
                                                in_widget: false,
                                                hover_change: Some(MouseHoverChange::EnterChild(enter_child_ident))
                                            }),
                                            input_state,
                                        ));
                                        event_dispatcher.queue_event(
                                            EventDestination::Widget(enter_child_id),
                                            DispatchableEvent::MouseMove {
                                                old_pos, new_pos,
                                                exiting_from_child: None,
                                            }
                                        );
                                    },
                                    None => {
                                        if contains_old && exiting_from_child.is_none() {
                                            perform_event_ops(widget.on_widget_event(
                                                WidgetEventSourced::This(WidgetEvent::MouseMove {
                                                    old_pos, new_pos,
                                                    in_widget: enter_child_opt.is_none(),
                                                    hover_change: None
                                                }),
                                                input_state,
                                            ));
                                        }
                                        input_state.mouse_hover_widget = Some(widget_id);
                                    }
                                }
                            },
                            false => {
                                send_exiting_from_child(&mut widget, contains_new);

                                perform_event_ops(widget.on_widget_event(
                                    WidgetEventSourced::This(WidgetEvent::MouseMove {
                                        old_pos, new_pos,
                                        in_widget: false,
                                        hover_change: Some(MouseHoverChange::Exit),
                                    }),
                                    input_state,
                                ));
                                event_dispatcher.queue_event(
                                    EventDestination::Relation(widget_id, Relation::Parent),
                                    DispatchableEvent::MouseMove {
                                        old_pos, new_pos,
                                        exiting_from_child: Some(path.last().cloned().unwrap()),
                                    }
                                );
                            }
                        }
                    },
                    DispatchableEvent::GainFocus{source, change} => if input_state.focused_widget != Some(widget_id) {
                        if let Some(focused_widget_id) = input_state.focused_widget {
                            event_dispatcher.queue_direct_event(
                                focused_widget_id,
                                WidgetEvent::LoseFocus
                            );
                        }
                        event_dispatcher.queue_direct_event(
                            widget_id,
                            WidgetEvent::GainFocus(source, change)
                        );
                    },
                    DispatchableEvent::Direct{bubble_source, event} => {
                        if bubble_source.is_some() {
                            unimplemented!()
                        } else {
                            match event {
                                WidgetEvent::LoseFocus =>
                                    input_state.focused_widget = None,
                                WidgetEvent::GainFocus(..) =>
                                    input_state.focused_widget= Some(widget_id),
                                _ => ()
                            }
                        }
                        perform_event_ops(widget.on_widget_event(
                            WidgetEventSourced::This(event),
                            input_state,
                        ));
                    }
                }
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        message_bus::MessageBus,
        cgmath::Point2,
        test_helpers::{TestEvent, TestRenderFrame},
        update_state::UpdateState,
        widget::WidgetIdent,
        widget_traverser::WidgetTraverserBase,
    };
    use derin_common_types::buttons::{Key, ModifierKeys, MouseButton};

    macro_rules! create_translator {
        ($translator:pat, $tree:expr, $root_id:expr) => {
            let message_bus = MessageBus::new();
            let mut traverser_base: WidgetTraverserBase<TestRenderFrame> = WidgetTraverserBase::new($root_id);
            let update_state = UpdateState::new(&message_bus);
            let mut traverser = traverser_base.with_root_ref($tree, update_state.clone());
            let mut input_state = InputState::new();

            let mut translator = EventTranslator::new();
            let $translator = translator.with_data(
                &mut traverser,
                &mut input_state,
                update_state
            );
        }
    }

    #[test]
    fn mouse_move_enter_child() {
        test_widget_tree!{
            let event_list = crate::test_helpers::EventList::new();
            let mut tree = a {
                rect: (0, 0, 40, 40);
                b {
                    rect: (10, 10, 30, 30);
                    c {
                        rect: (10, 10, 20, 20)
                    }
                }
            };
        }
        // Rough diagram:
        // a---------------------+
        // |                     |
        // |                     |
        // |    b----------------+
        // |    |                |
        // |    |                |
        // |    |    c-----------+
        // |    |    |           |
        // |    |    |           |
        // |    |    |           |
        // |    |    |           |
        // |    |    |           |
        // +----+----+-----------+


        event_list.set_events(vec![
            // WindowEvent::MouseMove(Point2::new(1, 5))
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-1, 5),
                    new_pos: Point2::new(1, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                }
            },

            // WindowEvent::MouseMove(Point2::new(2, 5))
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(1, 5),
                    new_pos: Point2::new(2, 5),
                    in_widget: true,
                    hover_change: None,
                }
            },

            // WindowEvent::MouseMove(Point2::new(15, 15)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(2, 5),
                    new_pos: Point2::new(15, 15),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(WidgetIdent::new_str("b"))),
                }
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-8, -5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                }
            },

            // WindowEvent::MouseMove(Point2::new(25, 25)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 5),
                    new_pos: Point2::new(15, 15),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(WidgetIdent::new_str("c"))),
                }
            },
            TestEvent {
                widget: c,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-5, -5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                }
            },

            // WindowEvent::MouseMove(Point2::new(1, 5)
            TestEvent {
                widget: c,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 5),
                    new_pos: Point2::new(-19, -15),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                }
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(15, 15),
                    new_pos: Point2::new(-9, -5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::ExitChild(WidgetIdent::new_str("c"))),
                }
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(15, 15),
                    new_pos: Point2::new(-9, -5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                }
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(25, 25),
                    new_pos: Point2::new(1, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::ExitChild(WidgetIdent::new_str("b"))),
                }
            },

            // WindowEvent::MouseExit
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(1, 5),
                    new_pos: Point2::new(-1, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                }
            },
        ]);

        create_translator!(mut translator, &mut tree, a);

        translator.translate_window_event(WindowEvent::MouseEnter);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(1, 5)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(2, 5)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(15, 15)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(25, 25)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(1, 5)));
        translator.translate_window_event(WindowEvent::MouseExit);
    }

    #[test]
    fn mouse_move_though_multiple() {
        test_widget_tree!{
            let event_list = crate::test_helpers::EventList::new();
            let mut tree = root {
                rect: (0, 0, 60, 20);
                left {
                    rect: (10, 1, 30, 19);
                    left_inner {rect: (5, 1, 15, 18)}
                },
                right {
                    rect: (40, 1, 50, 19)
                }
            };
        }
        // Rough diagram:
        // root---------------------------------------------------+
        // |    left-------------+         right------------+     |
        // |    |  left_inner-+  |         |                |     |
        // |    |  |          |  |         |                |     |
        // |    |  |          |  |         |                |     |
        // |    |  |          |  |         |                |     |
        // |    |  +----------+  |         |                |     |
        // |    +----------------+         +----------------+     |
        // +------------------------------------------------------+


        event_list.set_events(vec![
            // WindowEvent::MouseMove(Point2::new(5, 10))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-1, 10),
                    new_pos: Point2::new(5, 10),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                }
            },

            // WindowEvent::MouseMove(Point2::new(20, 10))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 10),
                    new_pos: Point2::new(20, 10),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(WidgetIdent::new_str("left"))),
                }
            },
            TestEvent {
                widget: left,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-5, 9),
                    new_pos: Point2::new(10, 9),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Enter),
                }
            },
            TestEvent {
                widget: left,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-5, 9),
                    new_pos: Point2::new(10, 9),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(WidgetIdent::new_str("left_inner"))),
                },
            },
            TestEvent {
                widget: left_inner,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-10, 8),
                    new_pos: Point2::new(5, 8),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseMove(Point2::new(45, 10)
            TestEvent {
                widget: left_inner,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 8),
                    new_pos: Point2::new(30, 8),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                },
            },
            TestEvent {
                widget: left,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(10, 9),
                    new_pos: Point2::new(35, 9),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::ExitChild(WidgetIdent::new_str("left_inner"))),
                },
            },
            TestEvent {
                widget: left,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(10, 9),
                    new_pos: Point2::new(35, 9),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(20, 10),
                    new_pos: Point2::new(45, 10),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::ExitChild(WidgetIdent::new_str("left"))),
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(20, 10),
                    new_pos: Point2::new(45, 10),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(WidgetIdent::new_str("right"))),
                },
            },
            TestEvent {
                widget: right,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-20, 9),
                    new_pos: Point2::new(5, 9),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            TestEvent {
                widget: right,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 9),
                    new_pos: Point2::new(-5, 9),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(45, 10),
                    new_pos: Point2::new(35, 10),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::ExitChild(WidgetIdent::new_str("right"))),
                },
            },
        ]);

        create_translator!(mut translator, &mut tree, root);

        translator.translate_window_event(WindowEvent::MouseEnter);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(5, 10)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(20, 10)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(45, 10)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(35, 10)));
    }

    #[test]
    fn mouse_down() {
        test_widget_tree!{
            let event_list = crate::test_helpers::EventList::new();
            let mut tree = root {
                rect: (0, 0, 50, 10);
                a { rect: (10, 0, 20, 10) },
                b { rect: (30, 0, 40, 10) }
            };
        }
        // rough diagram:
        // root----a--------+-------b--------+-------+
        // |       |        |       |        |       |
        // |       |        |       |        |       |
        // | root  |   a    | root  |   b    | root  |
        // |       |        |       |        |       |
        // |       |        |       |        |       |
        // +-------+--------+-------+--------+-------+

        let a_ident = WidgetIdent::new_str("a");
        let b_ident = WidgetIdent::new_str("b");

        event_list.set_events(vec![
            // WindowEvent::MouseEnter
            // WindowEvent::MouseMove(Point2::new(0, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-1, 5),
                    new_pos: Point2::new(0, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseMove(Point2::new(15, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(0, 5),
                    new_pos: Point2::new(15, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(a_ident.clone())),
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-10, 5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseDown(MouseButton::Left)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(5, 5),
                    in_widget: true,
                    button: MouseButton::Left,
                },
            },

            // WindowEvent::MouseMove(Point2::new(25, 5))
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 5),
                    new_pos: Point2::new(15, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(15, 5),
                    new_pos: Point2::new(25, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::ExitChild(a_ident.clone())),
                },
            },

            // WindowEvent::MouseMove(Point2::new(26, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(25, 5),
                    new_pos: Point2::new(26, 5),
                    in_widget: true,
                    hover_change: None,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(15, 5),
                    new_pos: Point2::new(16, 5),
                    in_widget: false,
                    hover_change: None,
                },
            },


            // WindowEvent::MouseScrollLines(Vector2::new(0, 1))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseScrollLines {
                    dir: Vector2::new(0, 1),
                    in_widget: true,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseScrollLines {
                    dir: Vector2::new(0, 1),
                    in_widget: false,
                },
            },

            // WindowEvent::MouseScrollPx(Vector2::new(0, 1))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseScrollPx {
                    dir: Vector2::new(0, 1),
                    in_widget: true,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseScrollPx {
                    dir: Vector2::new(0, 1),
                    in_widget: false,
                },
            },

            // WindowEvent::MouseDown(MouseButton::Middle)
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(26, 5),
                    in_widget: true,
                    button: MouseButton::Middle,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(16, 5),
                    in_widget: false,
                    button: MouseButton::Middle,
                },
            },

            // WindowEvent::MouseMove(Point2::new(35, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(26, 5),
                    new_pos: Point2::new(35, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(b_ident.clone())),
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(16, 5),
                    new_pos: Point2::new(25, 5),
                    in_widget: false,
                    hover_change: None,
                },
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-4, 5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseDown(MouseButton::Right)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(5, 5),
                    in_widget: true,
                    button: MouseButton::Right,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(25, 5),
                    in_widget: false,
                    button: MouseButton::Right,
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(35, 5),
                    in_widget: false,
                    button: MouseButton::Right,
                },
            },

            // WindowEvent::MouseMove(Point2::new(36, 5))
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 5),
                    new_pos: Point2::new(6, 5),
                    in_widget: true,
                    hover_change: None,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(25, 5),
                    new_pos: Point2::new(26, 5),
                    in_widget: false,
                    hover_change: None,
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(35, 5),
                    new_pos: Point2::new(36, 5),
                    in_widget: false,
                    hover_change: None,
                },
            },

            // WindowEvent::MouseUp(MouseButton::Middle)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    down_pos: Point2::new(-4, 5),
                    pos: Point2::new(6, 5),
                    in_widget: true,
                    pressed_in_widget: false,
                    button: MouseButton::Middle,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    down_pos: Point2::new(16, 5),
                    pos: Point2::new(26, 5),
                    in_widget: false,
                    pressed_in_widget: false,
                    button: MouseButton::Middle,
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    down_pos: Point2::new(26, 5),
                    pos: Point2::new(36, 5),
                    in_widget: false,
                    pressed_in_widget: true,
                    button: MouseButton::Middle,
                },
            },

            // WindowEvent::MouseMove(Point2::new(35, 5))
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(6, 5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: None,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(26, 5),
                    new_pos: Point2::new(25, 5),
                    in_widget: false,
                    hover_change: None,
                },
            },

            // WindowEvent::MouseUp(MouseButton::Left)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    down_pos: Point2::new(-15, 5),
                    pos: Point2::new(5, 5),
                    in_widget: true,
                    pressed_in_widget: false,
                    button: MouseButton::Left,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    down_pos: Point2::new(5, 5),
                    pos: Point2::new(25, 5),
                    in_widget: false,
                    pressed_in_widget: true,
                    button: MouseButton::Left,
                },
            },

            // WindowEvent::MouseUp(MouseButton::Right)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    down_pos: Point2::new(5, 5),
                    pos: Point2::new(5, 5),
                    in_widget: true,
                    pressed_in_widget: true,
                    button: MouseButton::Right,
                },
            },

            // WindowEvent::MouseMove(Point2::new(36, 5))
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 5),
                    new_pos: Point2::new(6, 5),
                    in_widget: true,
                    hover_change: None,
                },
            },

            // WindowEvent::MouseScrollLines(Vector2::new(0, 1))
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseScrollLines {
                    dir: Vector2::new(0, 1),
                    in_widget: true,
                },
            },

            // WindowEvent::MouseScrollPx(Vector2::new(0, 1))
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseScrollPx {
                    dir: Vector2::new(0, 1),
                    in_widget: true,
                },
            },
        ]);

        create_translator!(mut translator, &mut tree, root);

        translator.translate_window_event(WindowEvent::MouseEnter);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(0, 5)));


        // Move into widget `a` and press the left mouse button. Future mouse moves should send move
        // events to widget `a`, regardless of whether or not the mouse is over the widget.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(15, 5)));
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Left));

        // Test sending move events to `a`.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(25, 5)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(26, 5)));

        // Mouse scroll events should be delivered to all mouse widgets.
        translator.translate_window_event(WindowEvent::MouseScrollLines(Vector2::new(0, 1)));
        translator.translate_window_event(WindowEvent::MouseScrollPx(Vector2::new(0, 1)));

        // Press the MMB in the root widget.
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Middle));

        // Press the RMB in the right widget.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(35, 5)));
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Right));

        // This should send mouse movement events to all three widgets.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(36, 5)));

        // Sends mouse up events to all widgets.
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Middle));
        // Root widget should stop tracking mouse movement.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(35, 5)));
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Left));
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Right));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(36, 5)));
        translator.translate_window_event(WindowEvent::MouseScrollLines(Vector2::new(0, 1)));
        translator.translate_window_event(WindowEvent::MouseScrollPx(Vector2::new(0, 1)));
    }

    #[test]
    fn keyboard_focus() {
        test_widget_tree!{
            let event_list = crate::test_helpers::EventList::new();
            let mut tree = root {
                rect: (0, 0, 70, 10);
                a { rect: (10, 0, 20, 10), focus_controls: true },
                b { rect: (30, 0, 40, 10), focus_controls: true },
                c { rect: (50, 0, 60, 10) }
            };
        }
        // rough diagram:
        // root----a--------+-------b--------+-------c--------+-------+
        // |       |        |       |        |       |        |       |
        // |       |        |       |        |       |        |       |
        // | root  |   a    | root  |   b    | root  |   c    | root  |
        // |       |        |       |        |       |        |       |
        // |       |        |       |        |       |        |       |
        // +-------+--------+-------+--------+-------+--------+-------+

        let a_ident = WidgetIdent::new_str("a");
        let b_ident = WidgetIdent::new_str("b");
        let c_ident = WidgetIdent::new_str("c");

        event_list.set_events(vec![
            // WindowEvent::MouseEnter
            // WindowEvent::MouseMove(Point2::new(0, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-1, 5),
                    new_pos: Point2::new(0, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseMove(Point2::new(15, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(0, 5),
                    new_pos: Point2::new(15, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(a_ident.clone())),
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-10, 5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseDown(MouseButton::Left)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(5, 5),
                    in_widget: true,
                    button: MouseButton::Left,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::GainFocus(FocusSource::This, FocusChange::Take),
            },

            // WindowEvent::MouseUp(MouseButton::Left)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    pos: Point2::new(5, 5),
                    down_pos: Point2::new(5, 5),
                    in_widget: true,
                    pressed_in_widget: true,
                    button: MouseButton::Left,
                },
            },

            // WindowEvent::MouseMove(Point2::new(25, 5))
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 5),
                    new_pos: Point2::new(15, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(15, 5),
                    new_pos: Point2::new(25, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::ExitChild(a_ident.clone())),
                },
            },

            // WindowEvent::MouseMove(Point2::new(26, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(25, 5),
                    new_pos: Point2::new(26, 5),
                    in_widget: true,
                    hover_change: None,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(15, 5),
                    new_pos: Point2::new(16, 5),
                    in_widget: false,
                    hover_change: None,
                },
            },

            // WindowEvent::KeyDown(Key::A)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::KeyDown(Key::A, ModifierKeys::empty()),
            },
            // WindowEvent::KeyUp(Key::A)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::KeyUp(Key::A, ModifierKeys::empty()),
            },

            // WindowEvent::MouseMove(Point2::new(35, 5))
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(26, 5),
                    new_pos: Point2::new(35, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(b_ident.clone())),
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(16, 5),
                    new_pos: Point2::new(25, 5),
                    in_widget: false,
                    hover_change: None,
                },
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-4, 5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseDown(MouseButton::Left)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(5, 5),
                    in_widget: true,
                    button: MouseButton::Left,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(25, 5),
                    in_widget: false,
                    button: MouseButton::Left,
                },
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::LoseFocus,
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::GainFocus(FocusSource::This, FocusChange::Take),
            },

            // WindowEvent::MouseUp(MouseButton::Left)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    pos: Point2::new(5, 5),
                    down_pos: Point2::new(5, 5),
                    in_widget: true,
                    pressed_in_widget: true,
                    button: MouseButton::Left,
                },
            },

            // WindowEvent::KeyDown(Key::A)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::KeyDown(Key::A, ModifierKeys::empty()),
            },
            // WindowEvent::KeyUp(Key::A)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::KeyUp(Key::A, ModifierKeys::empty()),
            },

            // WindowEvent::MouseMove(Point2::new(55, 5))
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(5, 5),
                    new_pos: Point2::new(25, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::Exit),
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(35, 5),
                    new_pos: Point2::new(55, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::ExitChild(b_ident.clone())),
                },
            },
            TestEvent {
                widget: root,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(35, 5),
                    new_pos: Point2::new(55, 5),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(c_ident.clone())),
                },
            },
            TestEvent {
                widget: c,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-15, 5),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                },
            },

            // WindowEvent::MouseDown(MouseButton::Left)
            TestEvent {
                widget: c,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(5, 5),
                    in_widget: true,
                    button: MouseButton::Left,
                },
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseDown {
                    pos: Point2::new(25, 5),
                    in_widget: false,
                    button: MouseButton::Left,
                },
            },

            // WindowEvent::MouseUp(MouseButton::Left)
            TestEvent {
                widget: c,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    pos: Point2::new(5, 5),
                    down_pos: Point2::new(5, 5),
                    in_widget: true,
                    pressed_in_widget: true,
                    button: MouseButton::Left,
                },
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseUp {
                    pos: Point2::new(25, 5),
                    down_pos: Point2::new(25, 5),
                    in_widget: false,
                    pressed_in_widget: false,
                    button: MouseButton::Left,
                },
            },

            // WindowEvent::KeyDown(Key::LArrow)
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::KeyDown(Key::LArrow, ModifierKeys::empty()),
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::LoseFocus,
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::GainFocus(
                    FocusSource::Sibling {
                        ident: b_ident.clone(),
                        delta: 1,
                    },
                    FocusChange::Prev,
                ),
            },

            // WindowEvent::KeyUp(Key::LArrow)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::KeyUp(Key::LArrow, ModifierKeys::empty()),
            },
            // TODO: ALWAYS DELIVER KEYUP FOR EVERY KEYDOWN

            // WindowEvent::KeyDown(Key::Escape)
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::KeyDown(Key::Escape, ModifierKeys::empty()),
            },
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::LoseFocus,
            },

            // WindowEvent::KeyUp(Key::Escape)
        ]);

        create_translator!(mut translator, &mut tree, root);

        // Because no widget has keyboard focus, these events shouldn't get delivered to a widget.
        //
        // There should be *some* mechanism for delivering these events to the user (in the past
        // we used a universal event fallthrough, which may be worth looking at again). This test
        // will change when that mechanism gets implemented again.
        //
        // TODO: UPDATE TEST FOR UNFOCUSED KEYBOARD EVENTS
        translator.translate_window_event(WindowEvent::KeyDown(Key::A));
        translator.translate_window_event(WindowEvent::KeyUp(Key::A));

        // Move the mouse into the window.
        translator.translate_window_event(WindowEvent::MouseEnter);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(0, 5)));

        // Move into widget `a` and click the left mouse button, delivering focus to `a`. Future
        // mouse moves should send move events to widget `a`, regardless of whether or not the
        // mouse is over the widget.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(15, 5)));
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Left));
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Left));

        // Test sending mouse move events to `a`.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(25, 5)));
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(26, 5)));

        // Test sending keyboard events to `a`.
        translator.translate_window_event(WindowEvent::KeyDown(Key::A));
        translator.translate_window_event(WindowEvent::KeyUp(Key::A));


        // This should unfocus `a` and focus `b`.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(35, 5)));
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Left));
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Left));

        // Test sending keyboard events to `b`.
        translator.translate_window_event(WindowEvent::KeyDown(Key::A));
        translator.translate_window_event(WindowEvent::KeyUp(Key::A));


        // Because `c` doesn't take focus, clicking on it should NOT deliver focus to it.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(55, 5)));
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Left));
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Left));


        // Test focusing sibling widget. Should focus `a`.
        translator.translate_window_event(WindowEvent::KeyDown(Key::LArrow));
        translator.translate_window_event(WindowEvent::KeyUp(Key::LArrow));

        // Test removing keyboard focus. Should unfocus `a`.
        translator.translate_window_event(WindowEvent::KeyDown(Key::Escape));
        translator.translate_window_event(WindowEvent::KeyUp(Key::Escape));
    }
}
