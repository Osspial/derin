use arrayvec::ArrayVec;

use cgmath::{EuclideanSpace, Point2, Vector2, Array, Bounded};
use cgmath_geometry::{GeoBox, DimsBox, Segment};

use std::cmp::Ordering;

use {WindowEvent, LoopFlow, Root};
use tree::*;
use timer::Timer;
use popup::{PopupSummary, PopupID};
use event::{WidgetEvent, InputState, MouseDown, FocusChange};
use render::{Renderer, RenderFrame, FrameRectStack};
use widget_stack::{WidgetPath, WidgetStack};
use meta_tracker::{MetaDrain, MetaEvent, MetaEventVariant};
use dct::buttons::ModifierKeys;

use std::time::Duration;

pub struct EventLoopOps<'a, A: 'static, N: 'static, F: 'a, R: 'a, G: 'a>
    where N: Widget<A, F>,
          F: RenderFrame,
          R: Renderer<Frame=F>
{
    pub(crate) root: &'a mut Root<A, N, F>,
    pub(crate) on_action: &'a mut FnMut(A, &mut N, &mut F::Theme) -> LoopFlow<G>,
    pub(crate) bubble_fallthrough: &'a mut FnMut(WidgetEvent, &[WidgetIdent]) -> Option<A>,
    pub(crate) with_renderer: &'a mut FnMut(Option<PopupID>, &mut FnMut(&mut R))
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLoopResult<R> {
    pub flow: LoopFlow<R>,
    pub wait_until_call_timer: Option<Duration>,
    pub popup_deltas: Vec<PopupDelta>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupDelta {
    Create(PopupSummary),
    Remove(PopupID)
}

impl<'a, A, N, F, R, G> EventLoopOps<'a, A, N, F, R, G>
    where N: Widget<A, F>,
          F: RenderFrame,
          R: Renderer<Frame=F>
{
    pub fn set_modifiers(&mut self, modifiers: ModifierKeys) {
        self.root.modifiers = modifiers;
    }
    pub fn process_event(&mut self, event: WindowEvent) -> EventLoopResult<G> {
        self.process_event_inner(None, event)
    }
    pub fn process_popup_event(&mut self, popup_id: PopupID, event: WindowEvent) -> EventLoopResult<G> {
        self.process_event_inner(Some(popup_id), event)
    }
    pub fn remove_popup(&mut self, popup_id: PopupID) {
        self.root.popup_widgets.remove(popup_id);
    }

    fn process_event_inner(&mut self, event_popup_id: Option<PopupID>, event: WindowEvent) -> EventLoopResult<G> {
        let EventLoopOps {
            root: &mut Root {
                id: root_id,
                mouse_pos: ref mut root_mouse_pos,
                ref mut mouse_buttons_down,
                ref mut actions,
                ref mut widget_stack_base,
                ref mut force_full_redraw,
                ref mut event_stamp,
                ref mut widget_ident_stack,
                ref mut meta_tracker,
                ref mut timer_list,
                ref mut root_widget,
                ref mut theme,
                ref mut popup_widgets,
                ref mut modifiers,
                ref mut cursor_icon,
                ..
            },
            ref mut on_action,
            ref mut bubble_fallthrough,
            ref mut with_renderer
        } = *self;

        // let mut cur_popup_id = None;
        let mut popup_map_insert = Vec::new();
        let mut popup_deltas = Vec::new();

        let mut set_cursor_pos = None;
        let mut set_cursor_icon = None;
        let mouse_pos: &mut Point2<i32>;

        // If we're performing events on a popup, we remove that popup from the popup map so that
        // operations can be performed on other popups during event processing. We re-insert it
        // after all operations have been performed.
        let mut taken_popup_widget = None;

        let widget_stack_root = match event_popup_id {
            Some(id) => {
                taken_popup_widget = Some(popup_widgets.take(id).expect("Bad Popup ID"));
                let popup_widget = taken_popup_widget.as_mut().unwrap();

                mouse_pos = &mut popup_widget.mouse_pos;
                &mut *popup_widget.widget
            },
            None => {
                mouse_pos = root_mouse_pos;
                &mut *root_widget
            }
        };
        let mut widget_stack = widget_stack_base.use_stack_dyn(widget_stack_root, root_id);

        macro_rules! mouse_button_arrays {
            ($update_tag:expr, $root_offset:expr) => {{
                let shift_button = move |mut down: MouseDown| {
                    down.down_pos -= $root_offset;
                    down
                };
                let mbd_array: ArrayVec<[_; 5]> = mouse_buttons_down.clone().into_iter().map(&shift_button).collect();
                let mbdin_array: ArrayVec<[_; 5]> = $update_tag.mouse_state.get().mouse_button_sequence()
                    .into_iter().filter_map(|b| mouse_buttons_down.contains(b)).map(shift_button).collect();
                (mbd_array, mbdin_array)
            }}
        }

        macro_rules! try_push_action {
            (
                $widget:expr, $path:expr $(=> ($meta_tracker:expr))*,
                ($mbd_array:tt, $mbdin_array:tt) => $event:expr, to_rootspace: $widget_offset:expr $(, bubble ($bubble:expr, $bubble_store:expr, $bubble_path:expr))*
            ) => {{
                let widget_update_tag = $widget.update_tag();
                let widget_offset = $widget_offset;
                let (mbd_array, mbdin_array) = mouse_button_arrays!(widget_update_tag, widget_offset);
                let widget_id = widget_update_tag.widget_id;

                let input_state = InputState {
                    mouse_pos: *mouse_pos - widget_offset,
                    modifiers: *modifiers,
                    mouse_buttons_down: &mbd_array,
                    mouse_buttons_down_in_widget: &mbdin_array
                };
                let ($mbd_array, $mbdin_array) = (&mbd_array, &mbdin_array);
                let event = $event;
                let event_ops = $widget.on_widget_event(
                    event,
                    input_state,
                    popup_widgets.popups_owned_by_mut(widget_id),
                    if_tokens!(($($bubble_path)*) {
                        $($bubble_path)*
                    } else {
                        &[]
                    })
                );

                let widget_update_tag = $widget.update_tag();

                let ref mut meta_tracker = if_tokens!(($($meta_tracker)*) {
                    $($meta_tracker)*
                } else {*meta_tracker});
                if let Some(action) = event_ops.action {
                    actions.push_back(action);
                }
                if let Some(focus) = event_ops.focus {
                    meta_tracker.push_focus(focus, $path);
                }
                if let Some((popup_widget, mut popup_attributes)) = event_ops.popup {
                    let owner_id = widget_update_tag.widget_id;
                    popup_attributes.rect = popup_attributes.rect + widget_offset;
                    popup_map_insert.push((owner_id, popup_widget, popup_attributes));
                }
                if event_ops.bubble $(& $bubble)* {
                    meta_tracker.push_bubble(
                        event.translate(widget_offset).into(),
                        $path
                    );
                }
                set_cursor_pos = set_cursor_pos.or(event_ops.cursor_pos.map(|p| p + widget_offset));
                set_cursor_icon = set_cursor_icon.or(event_ops.cursor_icon);
                $(*$bubble_store = event_ops.bubble;)*

                widget_update_tag.last_event_stamp.set(*event_stamp);
                let widget_update = widget_update_tag.needs_update(root_id);
                if widget_update.update_timer {
                    let mut register = timer_list.new_timer_register(widget_update_tag.widget_id);
                    $widget.register_timers(&mut register);
                    widget_update_tag.unmark_update_timer();
                }
                widget_update_tag.last_event_stamp.set(*event_stamp);

                widget_update_tag
            }};
        }

        match event {
            WindowEvent::WindowResize(new_size) => {
                // Resize the window, forcing a full redraw.

                widget_stack.move_to_root();
                *widget_stack.top_mut().widget.rect_mut() = new_size.cast::<i32>().unwrap_or(DimsBox::max_value()).into();
                *force_full_redraw = true;
            },
            WindowEvent::MouseEnter(enter_pos) => {
                let WidgetPath{ widget: root_widget, path: root_path, .. } = widget_stack.move_to_root();

                let top_update_tag = try_push_action!(
                    root_widget, root_path.iter().cloned(),
                    (mbd_array, mbdin_array) => WidgetEvent::MouseEnter {
                        enter_pos,
                        buttons_down: mbd_array,
                        buttons_down_in_widget: mbdin_array
                    }, to_rootspace: Vector2::new(0, 0)
                );

                let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                top_update_tag.mouse_state.set(MouseState::Hovering(enter_pos, top_mbseq));
            },
            WindowEvent::MouseExit(exit_pos) => {
                widget_stack.move_to_hover();
                widget_stack.drain_to_root(|widget, path, parent_offset| {
                    let widget_offset = widget.rect().min().to_vec() + parent_offset;

                    let update_tag = try_push_action!(
                        widget, path.iter().cloned(),
                        (mbd_array, mbdin_array) => WidgetEvent::MouseExit {
                            exit_pos: exit_pos - widget_offset,
                            buttons_down: mbd_array,
                            buttons_down_in_widget: mbdin_array
                        }, to_rootspace: widget_offset
                    );


                    let mbseq = update_tag.mouse_state.get().mouse_button_sequence();
                    match mbseq.len() {
                        0 => update_tag.mouse_state.set(MouseState::Untracked),
                        _ => update_tag.mouse_state.set(MouseState::Tracking(exit_pos, mbseq))
                    }
                    update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::MOUSE_HOVER);
                });
            },


            WindowEvent::MouseMove(new_pos_windowspace) if new_pos_windowspace != *mouse_pos => {
                // Update the stored mouse position
                *mouse_pos = new_pos_windowspace;

                loop {
                    widget_stack.move_to_hover();

                    // Calculate the bounds of the hovered widget in window-space, and use that to get hovered
                    // widget's upper-right corner in window space (the offset).
                    let top_bounds_windowspace = widget_stack.top_bounds_offset();
                    let top_bounds_offset = top_bounds_windowspace.min().to_vec();

                    // Use the position of the widget's upper-right corner to turn the window-space coordinates
                    // into widget-space coordinates.
                    let top_bounds = top_bounds_windowspace - top_bounds_offset;
                    let new_pos = new_pos_windowspace - top_bounds_offset;

                    // Get a read-only copy of the update tag.
                    let update_tag_copy = widget_stack.top().update_tag().clone();

                    if let MouseState::Hovering(widget_old_pos, mbseq) = update_tag_copy.mouse_state.get() {
                        let move_line = Segment {
                            start: widget_old_pos,
                            end: new_pos
                        };

                        // SEND MOVE ACTION
                        {
                            let WidgetPath{ widget, path, .. } = widget_stack.top_mut();
                            try_push_action!(
                                widget, path.iter().cloned(),
                                (mbd_array, mbdin_array) => WidgetEvent::MouseMove {
                                    old_pos: widget_old_pos,
                                    new_pos: new_pos,
                                    in_widget: true,
                                    buttons_down: mbd_array,
                                    buttons_down_in_widget: mbdin_array
                                }, to_rootspace: top_bounds_offset
                            );
                        }

                        // Get the bounds of the widget after the widget has potentially been moved by the
                        // move action.
                        let new_top_bounds_windowspace = widget_stack.top_bounds_offset();
                        let new_top_bounds = new_top_bounds_windowspace - top_bounds_offset;

                        match new_top_bounds.contains(new_pos) {
                            true => {
                                // Whether or not to update the child layout.
                                let update_layout: bool;
                                {
                                    let top_update_tag = widget_stack.top().update_tag();
                                    top_update_tag.mouse_state.set(MouseState::Hovering(new_pos, mbseq));

                                    // Store whether or not the layout needs to be updated. We can set that the layout
                                    // no longer needs to be updated here because it's going to be updated in the subtrait
                                    // match below.
                                    update_layout = top_update_tag.needs_update(root_id).update_layout;
                                    top_update_tag.unmark_update_layout();
                                }

                                // Send actions to the child widget. If the hover widget isn't a parent, there's
                                // nothing we need to do.
                                let WidgetPath{ widget: top_widget, path: top_path, .. } = widget_stack.top_mut();
                                match top_widget.subtrait_mut() {
                                    WidgetSubtraitMut::Widget(_) => (),
                                    WidgetSubtraitMut::Parent(top_widget_as_parent) => {
                                        // Actually update the layout, if necessary.
                                        if update_layout {
                                            top_widget_as_parent.update_child_layout();
                                        }

                                        struct EnterChildData {
                                            child_ident: WidgetIdent,
                                            enter_pos: Point2<i32>,
                                            child_pos_offset: Vector2<i32>
                                        }
                                        let mut enter_child: Option<EnterChildData> = None;

                                        // Figure out if the cursor has moved into a child widget, and send the relevant events if
                                        // we have.
                                        top_widget_as_parent.children_mut(&mut |summary_list| {
                                            let mut child_summary = None;
                                            for summary in summary_list {
                                                if summary.rect.contains(new_pos) {
                                                    child_summary = Some(summary);
                                                    break;
                                                }
                                            }
                                            let child_summary = match child_summary {
                                                Some(summary) => summary,
                                                None => return LoopFlow::Continue
                                            };

                                            let WidgetSummary {
                                                widget: ref mut child,
                                                rect: child_bounds,
                                                ident: child_ident,
                                                ..
                                            } = *child_summary;

                                            let child_pos_offset = child_bounds.min().to_vec();

                                            // Find the exact location where the cursor entered the child widget. This is
                                            // done in the child's parent's coordinate space (i.e. the currently hovered
                                            // widget), and is translated to the child's coordinate space when we enter the
                                            // child.
                                            let enter_pos = child_bounds.intersect_line(move_line).0
                                                .unwrap_or(new_pos);

                                            // Get the mouse buttons already down in the child, and set the mouse
                                            // state to hover.
                                            let child_mbdin;
                                            {
                                                let child_update_tag = child.update_tag();
                                                child_mbdin = child_update_tag.mouse_state.get().mouse_button_sequence();
                                                child_update_tag.mouse_state.set(MouseState::Hovering(
                                                    widget_old_pos - child_pos_offset,
                                                    child_mbdin
                                                ));
                                            }

                                            // SEND ENTER ACTION TO CHILD
                                            try_push_action!(
                                                child, top_path.iter().cloned().chain(Some(child_ident)),
                                                (mbd_array, mbdin_array) => WidgetEvent::MouseEnter {
                                                    enter_pos: enter_pos - child_pos_offset,
                                                    buttons_down: mbd_array,
                                                    buttons_down_in_widget: mbdin_array
                                                }, to_rootspace: child_pos_offset
                                            );

                                            // Store the information relating to the child we entered,
                                            enter_child = Some(EnterChildData{ child_ident, enter_pos, child_pos_offset });

                                            // We `continue` the loop after this, but the continue is handled by the
                                            // `enter_child` check below.
                                            LoopFlow::Break(())
                                        });

                                        if let Some(EnterChildData{ child_ident, enter_pos, child_pos_offset }) = enter_child {
                                            // SEND CHILD ENTER ACTION
                                            try_push_action!(
                                                top_widget_as_parent, top_path.iter().cloned(),
                                                (mbd_array, mbdin_array) => WidgetEvent::MouseEnterChild {
                                                    enter_pos,
                                                    buttons_down: mbd_array,
                                                    buttons_down_in_widget: mbdin_array,
                                                    child: child_ident
                                                }, to_rootspace: child_pos_offset
                                            );

                                            // Update the layout again, if the events we've sent have triggered a
                                            // relayout.
                                            let update_layout: bool;
                                            {
                                                let top_update_tag = top_widget_as_parent.update_tag();
                                                match mbseq.len() {
                                                    0 => top_update_tag.mouse_state.set(MouseState::Untracked),
                                                    _ => top_update_tag.mouse_state.set(MouseState::Tracking(new_pos, mbseq))
                                                }
                                                top_update_tag.child_event_recv.set(top_update_tag.child_event_recv.get() | ChildEventRecv::MOUSE_HOVER);
                                                update_layout = top_update_tag.needs_update(root_id).update_layout;
                                            }
                                            if update_layout {
                                                top_widget_as_parent.update_child_layout();
                                            }

                                            continue;
                                        }
                                    }
                                }
                            },
                            // If the cursor is no longer in the widget, send the exit events and move to the parent widget.
                            false => {
                                let mouse_exit = top_bounds.intersect_line(move_line).1.unwrap_or(new_pos);

                                {
                                    let WidgetPath{ widget, path, .. } = widget_stack.top_mut();
                                    try_push_action!(
                                        widget, path.iter().cloned(),
                                        (mbd_array, mbdin_array) => WidgetEvent::MouseExit {
                                            exit_pos: mouse_exit,
                                            buttons_down: mbd_array,
                                            buttons_down_in_widget: mbdin_array
                                        }, to_rootspace: top_bounds_offset
                                    );
                                }

                                {
                                    let top_update_tag = widget_stack.top().update_tag();
                                    match mbseq.len() {
                                        0 => top_update_tag.mouse_state.set(MouseState::Untracked),
                                        _ => top_update_tag.mouse_state.set(MouseState::Tracking(new_pos, mbseq))
                                    }
                                }

                                // Send the exit action and mark the parent as hovered, as long as we aren't at the root.
                                if 0 < widget_stack.depth() {
                                    let top_parent_offset = widget_stack.top_parent_offset();
                                    let child_exit_pos = mouse_exit - top_parent_offset;
                                    let child_ident = widget_stack.top_ident();

                                    widget_stack.pop();
                                    {
                                        let WidgetPath{ widget, path, .. } = widget_stack.top_mut();
                                        try_push_action!(
                                            widget, path.iter().cloned(),
                                            (mbd_array, mbdin_array) => WidgetEvent::MouseExitChild {
                                                exit_pos: child_exit_pos,
                                                buttons_down: mbd_array,
                                                buttons_down_in_widget: mbdin_array,
                                                child: child_ident
                                            }, to_rootspace: top_parent_offset
                                        );
                                    }

                                    let top_update_tag = widget_stack.top().update_tag();
                                    let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                                    top_update_tag.mouse_state.set(MouseState::Hovering(child_exit_pos, top_mbseq));
                                    top_update_tag.child_event_recv.set(top_update_tag.child_event_recv.get() & !ChildEventRecv::MOUSE_HOVER);

                                    continue;
                                }
                            }
                        }
                    } else {
                        // If we enter an untracked state, that means we've recieved a MouseMove event without a MouseEnter
                        // event. So, set the root as hover and re-calculate from there.
                        let root = widget_stack.move_to_root().widget;

                        let top_update_tag = root.update_tag();
                        let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                        top_update_tag.mouse_state.set(MouseState::Hovering(new_pos_windowspace, top_mbseq));
                        continue;
                    }

                    break;
                }

                // Send move events to widgets that are being click-dragged but aren't being hovered.
                widget_stack.move_over_flags(ChildEventRecv::MOUSE_BUTTONS, |widget, path, widget_parent_offset| {
                    let new_pos: Point2<i32>;
                    let (widget_needs_move_event, widget_old_pos, widget_offset): (bool, Point2<i32>, Vector2<i32>);

                    {
                        widget_offset = widget_parent_offset + widget.rect().min().to_vec();
                        let update_tag = widget.update_tag();
                        new_pos = new_pos_windowspace - widget_offset;

                        widget_needs_move_event = update_tag.last_event_stamp.get() != *event_stamp;
                        match update_tag.mouse_state.get() {
                            MouseState::Tracking(old_pos, mbseq) => {
                                update_tag.mouse_state.set(MouseState::Tracking(new_pos, mbseq));
                                widget_old_pos = old_pos;
                            },
                            s if widget_needs_move_event => panic!("unexpected mouse state: {:?}", s),
                            _ => widget_old_pos = Point2::from_value(0xDEDBEEF)
                        }
                    }

                    if widget_needs_move_event {
                        try_push_action!(
                            widget, path.iter().cloned(),
                            (mbd_array, mbdin_array) => WidgetEvent::MouseMove {
                                old_pos: widget_old_pos,
                                new_pos: new_pos,
                                in_widget: false,
                                buttons_down: mbd_array,
                                buttons_down_in_widget: mbdin_array
                            }, to_rootspace: widget_offset
                        );
                    }

                    widget.update_tag()
                });
            },
            WindowEvent::MouseMove(_) => (),


            WindowEvent::MouseDown(button) => {
                let recv_flags = ChildEventRecv::MOUSE_HOVER | ChildEventRecv::KEYBOARD;
                let button_mask = ChildEventRecv::mouse_button_mask(button);
                widget_stack.move_over_flags(recv_flags, |top_widget, path, top_parent_offset| {
                    let bounds_rootspace = top_widget.rect() + top_parent_offset;
                    let top_widget_offset = bounds_rootspace.min.to_vec();
                    let in_widget = bounds_rootspace.contains(*mouse_pos);

                    try_push_action!(
                        top_widget, path.iter().cloned(),
                        (_, _) => WidgetEvent::MouseDown {
                            pos: *mouse_pos - top_widget_offset,
                            in_widget,
                            button
                        }, to_rootspace: top_widget_offset
                    );

                    let top_update_tag = top_widget.update_tag();
                    match top_update_tag.mouse_state.get() {
                        MouseState::Untracked     |
                        MouseState::Tracking(..) => (),
                        MouseState::Hovering(mouse_pos, mut top_mbseq) => if in_widget {
                            top_update_tag.mouse_state.set(MouseState::Hovering(mouse_pos, *top_mbseq.push_button(button)));
                        }
                    }

                    if in_widget {
                        top_update_tag.child_event_recv.set(top_update_tag.child_event_recv.get() | button_mask)
                    }
                    top_update_tag
                });
                mouse_buttons_down.push_button(button, *mouse_pos);

                widget_stack.move_to_hover();
                widget_stack.drain_to_root(|widget, _, _| {
                    let widget_update_tag = widget.update_tag();
                    widget_update_tag.child_event_recv.set(widget_update_tag.child_event_recv.get() | button_mask);
                });
            },
            WindowEvent::MouseUp(button) => {
                let button_mask = ChildEventRecv::mouse_button_mask(button);
                let down_pos_rootspace = mouse_buttons_down.contains(button).map(|down| down.down_pos)
                    .unwrap_or(Point2::new(0, 0));
                mouse_buttons_down.release_button(button);

                // Send the mouse up event to the hover widget.
                let mut move_to_tracked = true;
                widget_stack.move_over_flags(ChildEventRecv::MOUSE_HOVER, |widget, path, top_parent_offset| {
                    let bounds_rootspace = widget.rect() + top_parent_offset;
                    let widget_offset = bounds_rootspace.min().to_vec();
                    let in_widget = bounds_rootspace.contains(*mouse_pos);
                    let pressed_in_widget = widget.update_tag().mouse_state.get().mouse_button_sequence().contains(button);
                    // If the hover widget wasn't the one where the mouse was originally pressed, ensure that
                    // we move to the widget where it was pressed.
                    move_to_tracked = !pressed_in_widget;

                    try_push_action!(
                        widget, path.iter().cloned(),
                        (_, _) => WidgetEvent::MouseUp {
                            pos: *mouse_pos - widget_offset,
                            down_pos: down_pos_rootspace - widget_offset,
                            in_widget,
                            pressed_in_widget,
                            button
                        }, to_rootspace: widget_offset
                    );

                    let update_tag = widget.update_tag();

                    let set_mouse_state = match update_tag.mouse_state.get() {
                        MouseState::Hovering(mouse_pos, mut top_mbseq) => {
                            MouseState::Hovering(mouse_pos, *top_mbseq.release_button(button))
                        },
                        MouseState::Untracked => unreachable!(),
                        MouseState::Tracking(..) => unreachable!()
                    };
                    update_tag.mouse_state.set(set_mouse_state);

                    update_tag
                });

                // If the hover widget wasn't the widget where the mouse button was originally pressed,
                // send the move event to original presser.
                if move_to_tracked {
                    widget_stack.move_over_flags(button_mask, |widget, path, top_parent_offset| {
                        let bounds_rootspace = widget.rect() + top_parent_offset;
                        let widget_offset = bounds_rootspace.min().to_vec();
                        try_push_action!(
                            widget, path.iter().cloned(),
                            (_, _) => WidgetEvent::MouseUp {
                                pos: *mouse_pos - widget_offset,
                                down_pos: down_pos_rootspace - widget_offset,
                                in_widget: bounds_rootspace.contains(*mouse_pos),
                                pressed_in_widget: true,
                                button
                            }, to_rootspace: widget_offset
                        );

                        let update_tag = widget.update_tag();

                        let set_mouse_state = match update_tag.mouse_state.get() {
                            MouseState::Untracked => unreachable!(),
                            MouseState::Tracking(mouse_pos, mut top_mbseq) => {
                                top_mbseq.release_button(button);
                                match top_mbseq.len() {
                                    0 => MouseState::Untracked,
                                    _ => MouseState::Tracking(mouse_pos, top_mbseq)
                                }
                            },
                            MouseState::Hovering(mouse_pos, mut top_mbseq) => MouseState::Hovering(mouse_pos, *top_mbseq.release_button(button))
                        };
                        update_tag.mouse_state.set(set_mouse_state);

                        update_tag
                    });
                }

                for widget in widget_stack.widgets() {
                    let update_tag = widget.update_tag();
                    update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !button_mask);
                }
            },
            WindowEvent::KeyDown(key) => {
                if let Some(WidgetPath{ widget: focus_widget, path, top_parent_offset }) = widget_stack.move_to_keyboard_focus() {
                    let bounds_rootspace = focus_widget.rect() + top_parent_offset;
                    let widget_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(focus_widget, path.iter().cloned(), (_, _) => WidgetEvent::KeyDown(key, *modifiers), to_rootspace: widget_offset);
                }
            },
            WindowEvent::KeyUp(key) => {
                if let Some(WidgetPath{ widget: focus_widget, path, top_parent_offset }) = widget_stack.move_to_keyboard_focus() {
                    let bounds_rootspace = focus_widget.rect() + top_parent_offset;
                    let widget_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(focus_widget, path.iter().cloned(), (_, _) => WidgetEvent::KeyUp(key, *modifiers), to_rootspace: widget_offset);
                }
            },
            WindowEvent::Char(c) => {
                if let Some(WidgetPath{ widget: focus_widget, path, top_parent_offset }) = widget_stack.move_to_keyboard_focus() {
                    let bounds_rootspace = focus_widget.rect() + top_parent_offset;
                    let widget_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(focus_widget, path.iter().cloned(), (_, _) => WidgetEvent::Char(c), to_rootspace: widget_offset);
                }
            },
            WindowEvent::Timer => {
                let triggered_timers = timer_list.trigger_timers().triggered_timers().to_vec();

                fn timer_widget_id(timer: Timer) -> WidgetID {
                    timer.widget_id
                }
                widget_stack.move_over_widgets(
                    triggered_timers.iter().cloned().map(timer_widget_id),
                    |widget, path, timer_index, top_parent_offset| {
                        let timer = triggered_timers[timer_index];
                        try_push_action!(
                            widget,
                            path.iter().cloned(),
                            (_, _) => WidgetEvent::Timer {
                                name: timer.name,
                                start_time: timer.start_time,
                                last_trigger: timer.last_trigger,
                                frequency: timer.frequency,
                                times_triggered: timer.times_triggered
                            }, to_rootspace: widget.rect().min().to_vec() + top_parent_offset
                        );
                    }
                );
            }
        }

        // Dispatch focus-changing events to the widgets.
        let mut meta_drain = meta_tracker.drain_meta();
        assert_eq!(0, widget_ident_stack.len());
        loop {
            let mut take_focus_to = |
                widget_stack: &mut WidgetStack<_, _, _>,
                widget_ident_stack: &[WidgetIdent],
                meta_drain: &mut MetaDrain
            | {
                if let Some(WidgetPath{ widget, path, top_parent_offset }) = widget_stack.move_to_keyboard_focus() {
                    if path == widget_ident_stack {
                        return;
                    }
                    widget.update_tag().has_keyboard_focus.set(false);
                    let bounds_rootspace = widget.rect() + top_parent_offset;
                    let widget_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(widget, path.into_iter().cloned() => (*meta_drain), (_, _) => WidgetEvent::LoseFocus, to_rootspace: widget_offset);

                    for update_tag in widget_stack.widgets().map(|n| n.update_tag()) {
                        update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::KEYBOARD);
                    }
                }
                if let Some(WidgetPath{ widget, path, top_parent_offset }) = widget_stack.move_to_path(widget_ident_stack.iter().cloned()) {
                    widget.update_tag().has_keyboard_focus.set(true);
                    let bounds_rootspace = widget.rect() + top_parent_offset;
                    let widget_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(widget, path.into_iter().cloned() => (*meta_drain), (_, _) => WidgetEvent::GainFocus, to_rootspace: widget_offset);

                    widget_stack.pop();
                    for update_tag in widget_stack.widgets().map(|n| n.update_tag()) {
                        update_tag.child_event_recv.set(update_tag.child_event_recv.get() | ChildEventRecv::KEYBOARD);
                    }
                }
            };
            let meta_variant = match meta_drain.next() {
                Some(MetaEvent{ source, variant }) => {
                    widget_ident_stack.extend(source);
                    variant
                },
                None => break
            };

            match meta_variant {
                MetaEventVariant::FocusChange(focus) => match focus {
                    FocusChange::Remove => {
                        if let Some(WidgetPath{ widget, path, top_parent_offset }) = widget_stack.move_to_path(widget_ident_stack.iter().cloned()) {
                            if widget.update_tag().has_keyboard_focus.get() {
                                widget.update_tag().has_keyboard_focus.set(false);
                                let bounds_rootspace = widget.rect() + top_parent_offset;
                                let widget_offset = bounds_rootspace.min().to_vec();

                                try_push_action!(
                                    widget,
                                    path.into_iter().cloned() => (meta_drain),
                                    (_, _) => WidgetEvent::LoseFocus, to_rootspace: widget_offset
                                );
                                for update_tag in widget_stack.widgets().map(|n| n.update_tag()) {
                                    update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::KEYBOARD);
                                }
                            }
                        }
                    },
                    FocusChange::Take => take_focus_to(&mut widget_stack, &**widget_ident_stack, &mut meta_drain),
                    FocusChange::Next |
                    FocusChange::Prev => {
                        let sibling_dist = match focus {
                            FocusChange::Next => 1,
                            FocusChange::Prev => -1,
                            _ => unreachable!()
                        };

                        widget_stack.move_to_path(widget_ident_stack.iter().cloned()).unwrap();
                        let mut advance_to_sibling = true;
                        loop {
                            let dist_multiplier = advance_to_sibling as isize;
                            advance_to_sibling = true;
                            match widget_stack.move_to_sibling_delta(sibling_dist * dist_multiplier) {
                                Ok(WidgetPath{ widget: sibling, .. }) => match sibling.accepts_focus() {
                                    OnFocus::Accept => {
                                        widget_ident_stack.clear();
                                        widget_ident_stack.extend(widget_stack.ident().iter().cloned());
                                        break;
                                    },
                                    OnFocus::Skip => continue,
                                    OnFocus::FocusChild => {
                                        widget_stack.try_push(|top, _| {
                                            let top_parent = top.subtrait_mut().as_parent()?;
                                            let child_index = match sibling_dist.signum() {
                                                -1 => top_parent.num_children() - 1,
                                                1 => 0,
                                                _ => unreachable!()
                                            };
                                            top_parent.child_by_index_mut(child_index)
                                        }).expect("Need to handle this");
                                        advance_to_sibling = false;
                                        continue;
                                    }
                                },
                                Err(index_out_of_bounds_cmp) => {
                                    match widget_stack.parent() {
                                        Some(parent) => match parent.on_child_focus_overflow() {
                                            OnFocusOverflow::Wrap => {
                                                let move_to_index = match index_out_of_bounds_cmp {
                                                    Ordering::Less => parent.num_children() - 1,
                                                    Ordering::Greater => 0,
                                                    Ordering::Equal => unreachable!()
                                                };
                                                widget_stack.move_to_sibling_index(move_to_index).unwrap();
                                                advance_to_sibling = false;
                                                continue;
                                            },
                                            OnFocusOverflow::Continue => {
                                                widget_stack.pop();
                                                continue;
                                            }
                                        },
                                        None => {
                                            match sibling_dist {
                                                -1 => widget_stack.try_push(|n, _| n.subtrait_mut().as_parent().and_then(|p| p.child_by_index_mut(p.num_children() - 1))),
                                                1 => widget_stack.try_push(|n, _| n.subtrait_mut().as_parent().and_then(|p| p.child_by_index_mut(0))),
                                                _ => unreachable!()
                                            };
                                            advance_to_sibling = false;
                                            continue;
                                        }
                                    }
                                },
                            }
                        }

                        take_focus_to(&mut widget_stack, &**widget_ident_stack, &mut meta_drain);
                    }
                },
                MetaEventVariant::EventBubble(event_owned) => {
                    if let None = widget_stack.move_to_path(widget_ident_stack[..widget_ident_stack.len() - 1].iter().cloned()) {
                        widget_ident_stack.clear();
                        continue;
                    }
                    event_owned.as_borrowed(mouse_buttons_down, |event| {
                        let mut continue_bubble = true;
                        let mut slice_range = widget_stack.depth() + 1..;
                        widget_stack.drain_to_root_while(|top_widget, ident, parent_offset| {
                            let widget_offset = parent_offset + top_widget.rect().min().to_vec();
                            try_push_action!(
                                top_widget,
                                ident.into_iter().cloned() => (meta_drain),
                                (_, _) => event.translate(-widget_offset),
                                to_rootspace: widget_offset,
                                bubble (false, &mut continue_bubble, &widget_ident_stack[slice_range.clone()])
                            );
                            slice_range.start -= 1;
                            continue_bubble
                        });

                        if continue_bubble {
                            if let Some(action) = bubble_fallthrough(event, &widget_ident_stack) {
                                actions.push_back(action);
                            }
                        }
                    });
                }
            }
            widget_ident_stack.clear();
        }

        // Increment the event stamp. Because new `UpdateTag`s have a default event stampo of 0,
        // make sure our event stamp is never 0.
        *event_stamp = event_stamp.wrapping_add(1);
        if *event_stamp == 0 {
            *event_stamp += 1;
        }

        let mut return_flow = LoopFlow::Continue;
        let mut root_update = widget_stack.move_to_root().widget.update_tag().needs_update(root_id);
        let mark_active_widgets_redraw = root_update.needs_redraw();

        drop(widget_stack);
        if 0 < actions.len() {
            while let Some(action) = actions.pop_front() {
                match on_action(action, root_widget, theme) {
                    LoopFlow::Continue => (),
                    LoopFlow::Break(ret) => {
                        return_flow = LoopFlow::Break(ret);
                        break;
                    }
                }
            }
        }
        let redraw_widget = match taken_popup_widget.as_mut() {
            Some(popup_widget) => &mut *popup_widget.widget,
            None => root_widget
        };

        with_renderer(event_popup_id, &mut |renderer| {
            if let Some(cursor_pos) = set_cursor_pos {
                renderer.set_cursor_pos(cursor_pos);
            }
            if let Some(set_icon) = set_cursor_icon {
                if set_icon != *cursor_icon {
                    renderer.set_cursor_icon(set_icon);
                    *cursor_icon = set_icon;
                }
            }


            // Draw the widget tree.
            if mark_active_widgets_redraw || *force_full_redraw {
                let force_full_redraw = *force_full_redraw || renderer.force_full_redraw();

                root_update.render_self |= force_full_redraw;
                root_update.update_child |= force_full_redraw;

                if root_update.render_self || root_update.update_child {
                    {
                        let (frame, base_transform) = renderer.make_frame();
                        let mut frame_rect_stack = FrameRectStack::new(frame, base_transform, theme, widget_ident_stack);

                        if let WidgetSubtraitMut::Parent(root_as_parent) = redraw_widget.subtrait_mut() {
                            update_widget_layout(root_id, force_full_redraw, root_as_parent);
                        }
                        if root_update.render_self {
                            redraw_widget.render(&mut frame_rect_stack);
                        }
                        if root_update.update_child {
                            if let WidgetSubtraitMut::Parent(root_as_parent) = redraw_widget.subtrait_mut() {
                                WidgetRenderer {
                                    root_id: root_id,
                                    frame: frame_rect_stack,
                                    force_full_redraw: force_full_redraw,
                                    theme
                                }.render_widget_children(root_as_parent)
                            }
                        }
                    }

                    renderer.finish_frame(theme);
                    redraw_widget.update_tag().mark_updated(root_id);
                }
            }

            renderer.set_size_bounds(redraw_widget.size_bounds());
            *force_full_redraw = false;
        });

        drop(redraw_widget);
        // Report popups that need to be created
        for (owner_id, popup_widget, popup_attributes) in popup_map_insert {
            let popup_id = popup_widgets.insert(owner_id, popup_attributes.ident, popup_widget);
            popup_deltas.push(PopupDelta::Create(PopupSummary {
                id: popup_id,
                attributes: popup_attributes
            }));
        }

        // Report popups that need to be destroyed.
        popup_deltas.extend(popup_widgets.popups_removed_by_children()
            .map(|removed_popup_id| PopupDelta::Remove(removed_popup_id)));

        // If we took a popup widget out of the popup map, replace it.
        if let Some(taken_popup_widget) = taken_popup_widget {
            popup_widgets.replace(event_popup_id.unwrap(), taken_popup_widget);
        }

        EventLoopResult {
            flow: return_flow,
            wait_until_call_timer: timer_list.time_until_trigger(),
            popup_deltas
        }
    }
}

fn update_widget_layout<A, F: RenderFrame>(root_id: RootID, force_full_redraw: bool, widget: &mut Parent<A, F>) -> bool {
    // Loop to re-solve widget layout, if children break their size bounds. Is 0..4 so that
    // it doesn't enter an infinite loop if children can never be properly solved.
    for _ in 0..4 {
        let Update {
            update_child,
            update_layout,
            ..
        } = widget.update_tag().needs_update(root_id);

        if update_layout || force_full_redraw {
            widget.update_child_layout();
            widget.update_tag().unmark_update_layout();
        }

        let mut children_break_bounds = false;
        if update_child || force_full_redraw {
            widget.children_mut(&mut |children_summaries| {
                for summary in children_summaries {
                    let WidgetSummary {
                        widget: ref mut child_widget,
                        ..
                    } = *summary;

                    if let WidgetSubtraitMut::Parent(child_widget_as_parent) = child_widget.subtrait_mut() {
                        children_break_bounds |= update_widget_layout(root_id, force_full_redraw, child_widget_as_parent);
                    }

                    child_widget.update_tag().unmark_update_layout();
                }

                LoopFlow::Continue
            });
        }

        if !children_break_bounds {
            break;
        }
    }

    let widget_rect = DimsBox::new(widget.rect().dims());
    widget.size_bounds().bound_rect(widget_rect) != widget_rect
}

struct WidgetRenderer<'a, F>
    where F: 'a + RenderFrame
{
    root_id: RootID,
    frame: FrameRectStack<'a, F>,
    force_full_redraw: bool,
    theme: &'a F::Theme
}

impl<'a, F> WidgetRenderer<'a, F>
    where F: 'a + RenderFrame
{
    fn render_widget_children<A>(&mut self, parent: &mut Parent<A, F>) {
        parent.children_mut(&mut |children_summaries| {
            for summary in children_summaries {
                let WidgetSummary {
                    widget: ref mut child_widget,
                    ident,
                    rect: child_rect,
                    ..
                } = *summary;

                let mut root_update = child_widget.update_tag().needs_update(self.root_id);
                root_update.render_self |= self.force_full_redraw;
                root_update.update_child |= self.force_full_redraw;
                let Update {
                    render_self,
                    update_child,
                    update_layout: _,
                    update_timer: _
                } = root_update;

                match child_widget.subtrait_mut() {
                    WidgetSubtraitMut::Parent(child_widget_as_parent) => {
                        let mut child_frame = self.frame.enter_child_widget(ident);
                        let mut child_frame = child_frame.enter_child_rect(child_rect);

                        if render_self {
                            child_widget_as_parent.render(&mut child_frame);
                        }
                        if update_child {
                            WidgetRenderer {
                                root_id: self.root_id,
                                frame: child_frame,
                                force_full_redraw: self.force_full_redraw,
                                theme: self.theme
                            }.render_widget_children(child_widget_as_parent);
                        }
                    },
                    WidgetSubtraitMut::Widget(child_widget) => {
                        if render_self {
                            let mut child_frame = self.frame.enter_child_widget(ident);
                            let mut child_frame = child_frame.enter_child_rect(child_rect);

                            child_widget.render(&mut child_frame);
                        }
                    }
                }

                child_widget.update_tag().mark_updated(self.root_id);
            }

            LoopFlow::Continue
        });
    }
}
