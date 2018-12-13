mod dispatcher;

use crate::{
    WindowEvent, InputState, LoopFlow,
    event::{FocusSource, MouseHoverChange, WidgetEvent},
    tree::*,
    timer::TimerList,
    render::RenderFrame,
    widget_stack::{WidgetStack, WidgetStackBase, WidgetPath},
    update_state::{UpdateStateBuffered},
    offset_widget::{OffsetWidgetTrait, OffsetWidgetTraitAs},
    virtual_widget_tree::VirtualWidgetTree
};
use self::dispatcher::{EventDispatcher, EventDestination, DispatchableEvent};
use cgmath_geometry::rect::GeoBox;
use std::{
    rc::Rc,
    iter::{ExactSizeIterator, DoubleEndedIterator}
};

pub(crate) struct EventTranslator<A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    widget_stack_base: WidgetStackBase<A, F>,
    inner: TranslatorInner<A>
}

pub(crate) struct TranslatorActive<'a, A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    widget_stack: WidgetStack<'a, A, F>,
    inner: &'a mut TranslatorInner<A>
}

struct TranslatorInner<A: 'static> {
    actions: Vec<A>,
    timer_list: TimerList,
    event_dispatcher: EventDispatcher,
    virtual_widget_tree: VirtualWidgetTree,
    update_state: Rc<UpdateStateBuffered>,
}

impl<A, F> EventTranslator<A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    pub fn new(root_id: WidgetID) -> EventTranslator<A, F> {
        EventTranslator {
            widget_stack_base: WidgetStackBase::new(),
            inner: TranslatorInner {
                actions: Vec::new(),
                timer_list: TimerList::new(None),
                event_dispatcher: EventDispatcher::new(),
                virtual_widget_tree: VirtualWidgetTree::new(root_id),
                update_state: UpdateStateBuffered::new(),
            },
        }
    }

    pub fn with_widget<'a>(&'a mut self, root: &'a mut Widget<A, F>) -> TranslatorActive<'a, A, F> {
        TranslatorActive {
            widget_stack: self.widget_stack_base.use_stack_dyn(root),
            inner: &mut self.inner
        }
    }

    pub fn drain_actions(&mut self) -> impl '_ + Iterator<Item=A> + ExactSizeIterator + DoubleEndedIterator {
        self.inner.actions.drain(..)
    }
}

impl<'a, A, F> TranslatorActive<'a, A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    pub fn translate_window_event(&mut self, window_event: WindowEvent, input_state: &mut InputState) {
        use self::WindowEvent::*;

        let TranslatorActive {
            ref mut widget_stack,
            ref mut inner
        } = self;
        let TranslatorInner {
            ref mut actions,
            ref mut timer_list,
            ref mut event_dispatcher,
            ref mut virtual_widget_tree,
            ref update_state
        } = inner;

        let mut queue_direct_event = |widget_id, event| {
            event_dispatcher.queue_event(
                EventDestination::Widget(widget_id),
                DispatchableEvent::Direct {
                    bubble_source: None,
                    event,
                }
            )
        };

        let mut root_widget_rect = || widget_stack.move_to_path(Some(ROOT_IDENT)).unwrap().widget.rect();
        let mut project_to_outside_root = |point| {
            let border_point = root_widget_rect().nearest_points(point).next().unwrap();
            let diff = (border_point - point).map(|i| i.signum());
            border_point + diff
        };

        let _: Option<()> =
        match window_event {
            MouseMove(new_pos) => try {
                let old_pos = input_state.mouse_pos
                    .unwrap_or_else(|| project_to_outside_root(new_pos));
                input_state.mouse_pos = Some(new_pos);

                let hover_widget_id = input_state.mouse_hover_widget
                    .unwrap_or(virtual_widget_tree.root_id());

                event_dispatcher.queue_event(
                    EventDestination::Widget(hover_widget_id),
                    DispatchableEvent::MouseMove {
                        old_pos, new_pos,
                        exiting_from_child: None,
                    }
                );
            },
            MouseEnter => None,
            MouseExit => try {
                let old_pos = input_state.mouse_pos?;
                let new_pos = input_state.mouse_pos
                    .unwrap_or_else(|| project_to_outside_root(old_pos));

                let hover_widget_id = input_state.mouse_hover_widget
                .unwrap_or(virtual_widget_tree.root_id());

                event_dispatcher.queue_event(
                    EventDestination::Widget(hover_widget_id),
                    DispatchableEvent::MouseMove {
                        old_pos, new_pos,
                        exiting_from_child: None,
                    }
                );
                if input_state.mouse_buttons_down.len() == 0 {
                    input_state.mouse_pos = None;
                }
            }
            MouseDown(mouse_button) => try {
                let mouse_pos = input_state.mouse_pos?;
                let hover_widget_id = input_state.mouse_hover_widget?;

                queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseDown {
                        pos: mouse_pos,
                        in_widget: true,
                        button: mouse_button
                    },
                );
                input_state.mouse_buttons_down.push_button(mouse_button, mouse_pos);
            },
            MouseUp(mouse_button) => try {
                let mouse_pos = input_state.mouse_pos?;
                let mouse_down = input_state.mouse_buttons_down.contains(mouse_button)?;
                let hover_widget_id = input_state.mouse_hover_widget
                    .unwrap_or(virtual_widget_tree.root_id());

                queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseUp {
                        pos: mouse_pos,
                        down_pos: mouse_down.down_pos,
                        pressed_in_widget: unimplemented!(),
                        in_widget: true,
                        button: mouse_button
                    },
                );
                input_state.mouse_buttons_down.release_button(mouse_button);
            },
            MouseScrollLines(dir) => try {
                let hover_widget_id = input_state.mouse_hover_widget?;
                queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseScrollLines(dir),
                );
            },
            MouseScrollPx(dir) => try {
                let hover_widget_id = input_state.mouse_hover_widget?;
                queue_direct_event(
                    hover_widget_id,
                    WidgetEvent::MouseScrollPx(dir),
                );
            },
            WindowResize(_) => unimplemented!(),
            KeyDown(key) => try {
                if !input_state.keys_down.contains(&key) {
                    input_state.keys_down.push(key);
                    match input_state.focused_widget {
                        Some(widget) => queue_direct_event(
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
                        Some(widget) => queue_direct_event(
                            widget,
                            WidgetEvent::KeyUp(key, input_state.modifiers),
                        ),
                        None => unimplemented!("dispatch to universal fallthrough")
                    }
                }
            },
            Char(c) => try {
                match input_state.focused_widget {
                    Some(widget) => queue_direct_event(
                        widget,
                        WidgetEvent::Char(c),
                    ),
                    None => unimplemented!("dispatch to universal fallthrough")
                }
            },
            Timer => unimplemented!(),
            Redraw => unimplemented!()
        };

        event_dispatcher.dispatch_events(
            widget_stack,
            virtual_widget_tree,
            |event_dispatcher, WidgetPath{mut widget, path, widget_id, index}, event| {
                let widget_ident = path.last().unwrap();
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
                                    EventDestination::Sibling{of, delta: 1},
                                    FocusSource::Sibling{ident, delta: -1}
                                )),
                                FocusChange::Prev => Some((
                                    EventDestination::Sibling{of, delta: -1},
                                    FocusSource::Sibling{ident, delta: 1}
                                )),
                                FocusChange::Parent => Some((
                                    EventDestination::Parent{of},
                                    FocusSource::Child{ident, index}
                                )),
                                FocusChange::ChildIdent(ident) => Some((
                                    EventDestination::ChildIdent{of, ident},
                                    FocusSource::Parent
                                )),
                                FocusChange::ChildIndex(index) => Some((
                                    EventDestination::ChildIndex{of, index},
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
                    DispatchableEvent::MouseMove{old_pos, new_pos, exiting_from_child} => {
                        let widget_rect = widget.rect();
                        let (contains_new, contains_old) = (widget_rect.contains(new_pos), widget_rect.contains(old_pos));
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
                                        perform_event_ops(widget.on_widget_event(
                                            WidgetEvent::MouseMove {
                                                old_pos, new_pos,
                                                in_widget: true,
                                                hover_change: match contains_old {
                                                    true => exiting_from_child.map(|ident| MouseHoverChange::ExitChild(ident)),
                                                    false => Some(MouseHoverChange::Enter)
                                                }
                                            },
                                            input_state,
                                            None, // TODO: POPUPS
                                            &[]
                                        ));
                                        input_state.mouse_hover_widget = Some(widget_id);
                                    }
                                }
                            },
                            false => {
                                // TODO: HANDLE EXIT/EXIT CHILD IN ONE MOVE
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
                                    EventDestination::Parent{of: widget_id},
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
    use std::sync::mpsc;
    use crate::{
        cgmath::Point2,
        test_helpers::TestEvent,
    };

    #[test]
    fn mouse_move() {
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

        event_list.set_events(vec![
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
            TestEvent {
                widget: a,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(1, 5),
                    new_pos: Point2::new(15, 15),
                    in_widget: false,
                    hover_change: Some(MouseHoverChange::EnterChild(WidgetIdent::new_str("b"))),
                }
            },
            TestEvent {
                widget: b,
                source_child: vec![],
                event: WidgetEvent::MouseMove {
                    old_pos: Point2::new(-9, -4),
                    new_pos: Point2::new(5, 5),
                    in_widget: true,
                    hover_change: Some(MouseHoverChange::Enter),
                }
            },
        ]);

        let mut translator = EventTranslator::new(a);
        let mut translator = translator.with_widget(&mut tree);
        let mut input_state = InputState::new();

        translator.translate_window_event(WindowEvent::MouseEnter, &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(1, 5)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(15, 15)), &mut input_state);
        translator.translate_window_event(WindowEvent::MouseMove(Point2::new(25, 25)), &mut input_state);
    }

    // #[test]
    // fn mouse_move() {
    //     let (tx, rx) = mpsc::channel();
    //     test_widget_tree!{
    //         let sender = tx;
    //         let mut tree = root {
    //             rect: (0, 0, 500, 500);
    //             left {
    //                 rect: (10, 10, 240, 490);
    //                 tl {rect: (10, 10, 220, 230)},
    //                 bl {rect: (10, 250, 220, 470)}
    //             },
    //             right {rect: (260, 10, 490, 490)}
    //         };
    //     }

    //     let mut translator = EventTranslator::new(root);
    //     let mut translator = translator.with_widget(&mut tree);
    //     let mut input_state = InputState::new();

    //     translator.translate_window_event(WindowEvent::MouseEnter, &mut input_state);
    //     translator.translate_window_event(WindowEvent::MouseMove(Point2::new(250, 200)), &mut input_state);
    //     translator.translate_window_event(WindowEvent::MouseMove(Point2::new(100, 250)), &mut input_state);
    //     translator.translate_window_event(WindowEvent::MouseMove(Point2::new(50, 250)), &mut input_state);
    //     translator.translate_window_event(WindowEvent::MouseMove(Point2::new(150, 250)), &mut input_state);
    //     translator.translate_window_event(WindowEvent::MouseMove(Point2::new(250, 250)), &mut input_state);
    //     translator.translate_window_event(WindowEvent::MouseMove(Point2::new(150, 250)), &mut input_state);
    //     translator.translate_window_event(WindowEvent::MouseMove(Point2::new(300, 50)), &mut input_state);

    //     while let Ok(e) = rx.try_recv() {
    //         println!("{:#?}", e);
    //     }

    //     panic!();
    // }
}
