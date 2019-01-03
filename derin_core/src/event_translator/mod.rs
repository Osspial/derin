mod dispatcher;

use crate::{
    WindowEvent, InputState, LoopFlow,
    cgmath::Vector2,
    event::{FocusSource, MouseHoverChange, WidgetEvent},
    tree::*,
    timer::TimerList,
    render::RenderFrame,
    widget_traverser::{Relation, WidgetTraverser, OffsetWidgetPath},
    update_state::{UpdateStateCell},
    offset_widget::{OffsetWidget, OffsetWidgetTrait, OffsetWidgetTraitAs},
};
use self::dispatcher::{EventDispatcher, EventDestination, DispatchableEvent};
use cgmath_geometry::rect::GeoBox;
use std::{
    rc::Rc,
    iter::{ExactSizeIterator, DoubleEndedIterator}
};

pub(crate) struct EventTranslator<A>
    where A: 'static,
{
    inner: TranslatorInner<A>
}

pub(crate) struct TranslatorActive<'a, 'b, A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    widget_traverser: &'a mut WidgetTraverser<'b, A, F>,
    inner: &'a mut TranslatorInner<A>,
    timer_list: &'a mut TimerList,
    input_state: &'a mut InputState,
    update_state: Rc<UpdateStateCell>,
}

struct TranslatorInner<A: 'static> {
    actions: Vec<A>,
    event_dispatcher: EventDispatcher,
}

impl<A> EventTranslator<A>
    where A: 'static,
{
    pub fn new() -> EventTranslator<A> {
        EventTranslator {
            inner: TranslatorInner {
                actions: Vec::new(),
                event_dispatcher: EventDispatcher::new(),
            },
        }
    }

    pub fn with_data<'a, 'b, F: RenderFrame>(
        &'a mut self,
        timer_list: &'a mut TimerList,
        widget_traverser: &'a mut WidgetTraverser<'b, A, F>,
        input_state: &'a mut InputState,
        update_state: Rc<UpdateStateCell>,
    ) -> TranslatorActive<'a, 'b, A, F> {
        TranslatorActive {
            widget_traverser,
            inner: &mut self.inner,
            timer_list,
            input_state,
            update_state,
        }
    }

    pub fn drain_actions(&mut self) -> impl '_ + Iterator<Item=A> + ExactSizeIterator + DoubleEndedIterator {
        self.inner.actions.drain(..)
    }
}

impl<A, F> TranslatorActive<'_, '_, A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    pub fn translate_window_event(&mut self, window_event: WindowEvent) {
        use self::WindowEvent::*;

        let TranslatorActive {
            ref mut widget_traverser,
            ref mut inner,
            timer_list,
            input_state,
            ref update_state,
        } = self;
        let TranslatorInner {
            ref mut actions,
            ref mut event_dispatcher,
        } = inner;

        let mut root_widget_rect = || {
            let root_id = widget_traverser.root_id();
            widget_traverser.get_widget(root_id).unwrap().widget.rect()
        };
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

        let mouse_down_widget_iter = input_state.mouse_buttons_down.clone().into_iter().map(|d| d.widget_id);

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

                for widget_id in mouse_down_widget_iter.filter(|id| *id != hover_widget_id) {
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
            MouseExit => try {
                let old_pos = input_state.mouse_pos?;
                let new_pos = project_to_outside_root(old_pos);

                self.translate_window_event(WindowEvent::MouseMove(new_pos));
                if self.input_state.mouse_buttons_down.len() == 0 {
                    self.input_state.mouse_pos = None;
                }

                return;
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

                for widget_id in mouse_down_widget_iter.filter(|id| *id != hover_widget_id) {
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

                for widget_id in mouse_down_widget_iter.filter(|id| *id != hover_widget_id) {
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
                    WidgetEvent::MouseScrollLines(dir),
                );
            },
            MouseScrollPx(dir) => try {
                let hover_widget_id = input_state.mouse_hover_widget?;
                event_dispatcher.queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseScrollPx(dir),
                );
            },
            WindowResize(_) => unimplemented!(),
            KeyDown(key) => try {
                if !input_state.keys_down.contains(&key) {
                    input_state.keys_down.push(key);
                    match input_state.focused_widget {
                        Some(widget) => event_dispatcher.queue_direct_event(
                            widget,
                            WidgetEvent::KeyDown(key, input_state.modifiers),
                        ),
                        None => unimplemented!("dispatch to universal fallthrough")
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
                        None => unimplemented!("dispatch to universal fallthrough")
                    }
                }
            },
            Char(c) => try {
                match input_state.focused_widget {
                    Some(widget) => event_dispatcher.queue_direct_event(
                        widget,
                        WidgetEvent::Char(c),
                    ),
                    None => unimplemented!("dispatch to universal fallthrough")
                }
            },
            Timer => unimplemented!(),
            Redraw => try {
                update_state.borrow_mut().queue_global_update();
            },
        };

        event_dispatcher.dispatch_events(
            widget_traverser,
            |event_dispatcher, OffsetWidgetPath{mut widget, path, widget_id, index}, event| {
                let widget_ident = path.last().unwrap();

                // Helper function that takes the `EventOps` generated by `on_widget_event`, updates
                // the input state, and queues more events as necessary.
                let mut perform_event_ops = |ops| {
                    use crate::event::{EventOps, FocusChange};
                    let EventOps {
                        action,
                        focus,
                        bubble,
                        cursor_pos,
                        cursor_icon,
                        popup
                    } = ops;
                    if let Some(action) = action {
                        actions.push(action);
                    }
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

                        let is_focused = input_state.focused_widget == Some(widget_id);
                        if !(is_focused && focus == FocusChange::Take) {
                            if let Some(focused_widget) = input_state.focused_widget {
                                event_dispatcher.queue_event(
                                    EventDestination::Widget(focused_widget),
                                    DispatchableEvent::Direct {
                                        bubble_source: None,
                                        event: WidgetEvent::LoseFocus
                                    }
                                );
                            }
                            if let Some((destination, source)) = destination_source_opt {
                                event_dispatcher.queue_event(
                                    destination,
                                    DispatchableEvent::Direct {
                                        bubble_source: None,
                                        event: WidgetEvent::GainFocus(source)
                                    }
                                );
                            }
                        }
                    }
                };

                match event {
                    // We handle `MouseMove` events differently than all other events because
                    // `MouseMove` can trigger other `MouseMove`s if the mouse moves into a child
                    // or parent widget.
                    DispatchableEvent::MouseMove{old_pos, new_pos, exiting_from_child} => {
                        let widget_rect = widget.rect();
                        let (contains_new, contains_old) = (widget_rect.contains(new_pos), widget_rect.contains(old_pos));

                        let mut send_exiting_from_child = |widget: &mut OffsetWidget<'_, dyn Widget<A, F>>, in_widget| {
                            if let Some(child_ident) = exiting_from_child.clone() {
                                perform_event_ops(widget.on_widget_event(
                                    WidgetEvent::MouseMove {
                                        old_pos, new_pos,
                                        in_widget,
                                        hover_change: Some(MouseHoverChange::ExitChild(child_ident)),
                                    },
                                    input_state,
                                    None, // TODO: POPUPS
                                    &[]
                                ));
                            }
                        };

                        match contains_new {
                            true => {
                                let mut enter_child_opt = None;
                                if let Some(mut widget_as_parent) = widget.as_parent_mut() {
                                    widget_as_parent.children_mut(|child_summary| {
                                        if child_summary.widget.rect().contains(new_pos) {
                                            enter_child_opt = Some((child_summary.widget.widget_tag().widget_id, child_summary.ident));
                                            LoopFlow::Break(())
                                        } else {
                                            LoopFlow::Continue
                                        }
                                    });
                                }

                                send_exiting_from_child(&mut widget, contains_new && enter_child_opt.is_none());

                                if !contains_old {
                                    perform_event_ops(widget.on_widget_event(
                                        WidgetEvent::MouseMove {
                                            old_pos, new_pos,
                                            in_widget: enter_child_opt.is_none(),
                                            hover_change: Some(MouseHoverChange::Enter)
                                        },
                                        input_state,
                                        None, // TODO: POPUPS
                                        &[]
                                    ));
                                }

                                match enter_child_opt {
                                    Some((enter_child_id, enter_child_ident)) => {
                                        perform_event_ops(widget.on_widget_event(
                                            WidgetEvent::MouseMove {
                                                old_pos, new_pos,
                                                in_widget: false,
                                                hover_change: Some(MouseHoverChange::EnterChild(enter_child_ident))
                                            },
                                            input_state,
                                            None, // TODO: POPUPS
                                            &[]
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
                                                WidgetEvent::MouseMove {
                                                    old_pos, new_pos,
                                                    in_widget: enter_child_opt.is_none(),
                                                    hover_change: None
                                                },
                                                input_state,
                                                None, // TODO: POPUPS
                                                &[]
                                            ));
                                        }
                                        input_state.mouse_hover_widget = Some(widget_id);
                                    }
                                }
                            },
                            false => {
                                send_exiting_from_child(&mut widget, contains_new);

                                perform_event_ops(widget.on_widget_event(
                                    WidgetEvent::MouseMove {
                                        old_pos, new_pos,
                                        in_widget: false,
                                        hover_change: Some(MouseHoverChange::Exit),
                                    },
                                    input_state,
                                    None,
                                    &[]
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
                    DispatchableEvent::Direct{bubble_source, event} => {
                        if bubble_source.is_some() {
                            unimplemented!()
                        }
                        perform_event_ops(widget.on_widget_event(
                            event,
                            input_state,
                            None,
                            &[]
                        ))
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
        cgmath::Point2,
        test_helpers::TestEvent,
    };
    use derin_common_types::buttons::MouseButton;

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

        let mut translator = EventTranslator::new(a);
        let mut translator = translator.with_widget(&mut tree);
        let mut input_state = InputState::new();

        translator.translate_window_event(WindowEvent::MouseEnter, &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(1, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(2, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(15, 15)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(25, 25)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(1, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseExit, &mut input_state);
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

        let mut translator = EventTranslator::new(root);
        let mut translator = translator.with_widget(&mut tree);
        let mut input_state = InputState::new();

        translator.translate_window_event(WindowEvent::MouseEnter, &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(5, 10)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(20, 10)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(45, 10)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(35, 10)), &mut input_state);
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
        ]);

        let mut translator = EventTranslator::new(root);
        let mut translator = translator.with_widget(&mut tree);
        let mut input_state = InputState::new();

        translator.translate_window_event(WindowEvent::MouseEnter, &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(0, 5)), &mut input_state);


        // Move into widget `a` and press the left mouse button. Future mouse moves should send move
        // events to widget `a`, regardless of whether or not the mouse is over the widget.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(15, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Left), &mut input_state);

        // Test sending move events to `a`.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(25, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(26, 5)), &mut input_state);

        // Press the MMB in the root widget.
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Middle), &mut input_state);

        // Press the RMB in the right widget.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(35, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseDown(MouseButton::Right), &mut input_state);

        // This should send mouse movement events to all three widgets.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(36, 5)), &mut input_state);

        // Sends mouse up events to all widgets.
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Middle), &mut input_state);
        // Root widget should stop tracking mouse movement.
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(35, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Left), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseUp(MouseButton::Right), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(36, 5)), &mut input_state);
    }
}
