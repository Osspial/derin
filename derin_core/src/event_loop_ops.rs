use arrayvec::ArrayVec;

use cgmath::{EuclideanSpace, Point2, Vector2, Array, Bounded};
use cgmath_geometry::{GeoBox, DimsBox, Segment};

use std::cmp::Ordering;

use {WindowEvent, LoopFlow, Root};
use tree::*;
use timer::Timer;
use popup::{PopupSummary, PopupID};
use event::{NodeEvent, InputState, MouseDown, FocusChange};
use render::{Renderer, RenderFrame, FrameRectStack};
use node_stack::{NodePath, NodeStack};
use meta_tracker::{MetaDrain, MetaEvent, MetaEventVariant};
use dct::buttons::ModifierKeys;

use std::time::Duration;

pub struct EventLoopOps<'a, A: 'static, N: 'static, F: 'a, R: 'a, G: 'a>
    where N: Node<A, F>,
          F: RenderFrame,
          R: Renderer<Frame=F>
{
    pub(crate) root: &'a mut Root<A, N, F>,
    pub(crate) on_action: &'a mut FnMut(A, &mut N, &mut F::Theme) -> LoopFlow<G>,
    pub(crate) bubble_fallthrough: &'a mut FnMut(NodeEvent, &[NodeIdent]) -> Option<A>,
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
    where N: Node<A, F>,
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
        self.root.popup_nodes.remove(popup_id);
    }

    fn process_event_inner(&mut self, event_popup_id: Option<PopupID>, event: WindowEvent) -> EventLoopResult<G> {
        let EventLoopOps {
            root: &mut Root {
                id: root_id,
                mouse_pos: ref mut root_mouse_pos,
                ref mut mouse_buttons_down,
                ref mut actions,
                ref mut node_stack_base,
                ref mut force_full_redraw,
                ref mut event_stamp,
                ref mut node_ident_stack,
                ref mut meta_tracker,
                ref mut timer_list,
                ref mut root_node,
                ref mut theme,
                ref mut popup_nodes,
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
        let mut taken_popup_node = None;

        let node_stack_root = match event_popup_id {
            Some(id) => {
                taken_popup_node = Some(popup_nodes.take(id).expect("Bad Popup ID"));
                let popup_node = taken_popup_node.as_mut().unwrap();

                mouse_pos = &mut popup_node.mouse_pos;
                &mut *popup_node.node
            },
            None => {
                mouse_pos = root_mouse_pos;
                &mut *root_node
            }
        };
        let mut node_stack = node_stack_base.use_stack_dyn(node_stack_root, root_id);

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
                $node:expr, $path:expr $(=> ($meta_tracker:expr))*,
                ($mbd_array:tt, $mbdin_array:tt) => $event:expr, to_rootspace: $node_offset:expr $(, bubble ($bubble:expr, $bubble_store:expr, $bubble_path:expr))*
            ) => {{
                let node_update_tag = $node.update_tag();
                let node_offset = $node_offset;
                let (mbd_array, mbdin_array) = mouse_button_arrays!(node_update_tag, node_offset);
                let node_id = node_update_tag.node_id;

                let input_state = InputState {
                    mouse_pos: *mouse_pos - node_offset,
                    modifiers: *modifiers,
                    mouse_buttons_down: &mbd_array,
                    mouse_buttons_down_in_node: &mbdin_array
                };
                let ($mbd_array, $mbdin_array) = (&mbd_array, &mbdin_array);
                let event = $event;
                let event_ops = $node.on_node_event(
                    event,
                    input_state,
                    popup_nodes.popups_owned_by_mut(node_id),
                    if_tokens!(($($bubble_path)*) {
                        $($bubble_path)*
                    } else {
                        &[]
                    })
                );

                let node_update_tag = $node.update_tag();

                let ref mut meta_tracker = if_tokens!(($($meta_tracker)*) {
                    $($meta_tracker)*
                } else {*meta_tracker});
                if let Some(action) = event_ops.action {
                    actions.push_back(action);
                }
                if let Some(focus) = event_ops.focus {
                    meta_tracker.push_focus(focus, $path);
                }
                if let Some((popup_node, mut popup_attributes)) = event_ops.popup {
                    let owner_id = node_update_tag.node_id;
                    popup_attributes.rect = popup_attributes.rect + node_offset;
                    popup_map_insert.push((owner_id, popup_node, popup_attributes));
                }
                if event_ops.bubble $(& $bubble)* {
                    meta_tracker.push_bubble(
                        event.translate(node_offset).into(),
                        $path
                    );
                }
                set_cursor_pos = set_cursor_pos.or(event_ops.cursor_pos.map(|p| p + node_offset));
                set_cursor_icon = set_cursor_icon.or(event_ops.cursor_icon);
                $(*$bubble_store = event_ops.bubble;)*

                node_update_tag.last_event_stamp.set(*event_stamp);
                let node_update = node_update_tag.needs_update(root_id);
                if node_update.update_timer {
                    let mut register = timer_list.new_timer_register(node_update_tag.node_id);
                    $node.register_timers(&mut register);
                    node_update_tag.unmark_update_timer();
                }
                node_update_tag.last_event_stamp.set(*event_stamp);

                node_update_tag
            }};
        }

        match event {
            WindowEvent::WindowResize(new_size) => {
                // Resize the window, forcing a full redraw.

                node_stack.move_to_root();
                *node_stack.top_mut().node.rect_mut() = new_size.cast::<i32>().unwrap_or(DimsBox::max_value()).into();
                *force_full_redraw = true;
            },
            WindowEvent::MouseEnter(enter_pos) => {
                let NodePath{ node: root_node, path: root_path, .. } = node_stack.move_to_root();

                let top_update_tag = try_push_action!(
                    root_node, root_path.iter().cloned(),
                    (mbd_array, mbdin_array) => NodeEvent::MouseEnter {
                        enter_pos,
                        buttons_down: mbd_array,
                        buttons_down_in_node: mbdin_array
                    }, to_rootspace: Vector2::new(0, 0)
                );

                let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                top_update_tag.mouse_state.set(MouseState::Hovering(enter_pos, top_mbseq));
            },
            WindowEvent::MouseExit(exit_pos) => {
                node_stack.move_to_hover();
                node_stack.drain_to_root(|node, path, parent_offset| {
                    let node_offset = node.rect().min().to_vec() + parent_offset;

                    let update_tag = try_push_action!(
                        node, path.iter().cloned(),
                        (mbd_array, mbdin_array) => NodeEvent::MouseExit {
                            exit_pos: exit_pos - node_offset,
                            buttons_down: mbd_array,
                            buttons_down_in_node: mbdin_array
                        }, to_rootspace: node_offset
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
                    node_stack.move_to_hover();

                    // Calculate the bounds of the hovered node in window-space, and use that to get hovered
                    // node's upper-right corner in window space (the offset).
                    let top_bounds_windowspace = node_stack.top_bounds_offset();
                    let top_bounds_offset = top_bounds_windowspace.min().to_vec();

                    // Use the position of the node's upper-right corner to turn the window-space coordinates
                    // into node-space coordinates.
                    let top_bounds = top_bounds_windowspace - top_bounds_offset;
                    let new_pos = new_pos_windowspace - top_bounds_offset;

                    // Get a read-only copy of the update tag.
                    let update_tag_copy = node_stack.top().update_tag().clone();

                    if let MouseState::Hovering(node_old_pos, mbseq) = update_tag_copy.mouse_state.get() {
                        let move_line = Segment {
                            start: node_old_pos,
                            end: new_pos
                        };

                        // SEND MOVE ACTION
                        {
                            let NodePath{ node, path, .. } = node_stack.top_mut();
                            try_push_action!(
                                node, path.iter().cloned(),
                                (mbd_array, mbdin_array) => NodeEvent::MouseMove {
                                    old_pos: node_old_pos,
                                    new_pos: new_pos,
                                    in_node: true,
                                    buttons_down: mbd_array,
                                    buttons_down_in_node: mbdin_array
                                }, to_rootspace: top_bounds_offset
                            );
                        }

                        // Get the bounds of the node after the node has potentially been moved by the
                        // move action.
                        let new_top_bounds_windowspace = node_stack.top_bounds_offset();
                        let new_top_bounds = new_top_bounds_windowspace - top_bounds_offset;

                        match new_top_bounds.contains(new_pos) {
                            true => {
                                // Whether or not to update the child layout.
                                let update_layout: bool;
                                {
                                    let top_update_tag = node_stack.top().update_tag();
                                    top_update_tag.mouse_state.set(MouseState::Hovering(new_pos, mbseq));

                                    // Store whether or not the layout needs to be updated. We can set that the layout
                                    // no longer needs to be updated here because it's going to be updated in the subtrait
                                    // match below.
                                    update_layout = top_update_tag.needs_update(root_id).update_layout;
                                    top_update_tag.unmark_update_layout();
                                }

                                // Send actions to the child node. If the hover node isn't a parent, there's
                                // nothing we need to do.
                                let NodePath{ node: top_node, path: top_path, .. } = node_stack.top_mut();
                                match top_node.subtrait_mut() {
                                    NodeSubtraitMut::Node(_) => (),
                                    NodeSubtraitMut::Parent(top_node_as_parent) => {
                                        // Actually update the layout, if necessary.
                                        if update_layout {
                                            top_node_as_parent.update_child_layout();
                                        }

                                        struct EnterChildData {
                                            child_ident: NodeIdent,
                                            enter_pos: Point2<i32>,
                                            child_pos_offset: Vector2<i32>
                                        }
                                        let mut enter_child: Option<EnterChildData> = None;

                                        // Figure out if the cursor has moved into a child node, and send the relevant events if
                                        // we have.
                                        top_node_as_parent.children_mut(&mut |summary_list| {
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

                                            let NodeSummary {
                                                node: ref mut child,
                                                rect: child_bounds,
                                                ident: child_ident,
                                                ..
                                            } = *child_summary;

                                            let child_pos_offset = child_bounds.min().to_vec();

                                            // Find the exact location where the cursor entered the child node. This is
                                            // done in the child's parent's coordinate space (i.e. the currently hovered
                                            // node), and is translated to the child's coordinate space when we enter the
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
                                                    node_old_pos - child_pos_offset,
                                                    child_mbdin
                                                ));
                                            }

                                            // SEND ENTER ACTION TO CHILD
                                            try_push_action!(
                                                child, top_path.iter().cloned().chain(Some(child_ident)),
                                                (mbd_array, mbdin_array) => NodeEvent::MouseEnter {
                                                    enter_pos: enter_pos - child_pos_offset,
                                                    buttons_down: mbd_array,
                                                    buttons_down_in_node: mbdin_array
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
                                                top_node_as_parent, top_path.iter().cloned(),
                                                (mbd_array, mbdin_array) => NodeEvent::MouseEnterChild {
                                                    enter_pos,
                                                    buttons_down: mbd_array,
                                                    buttons_down_in_node: mbdin_array,
                                                    child: child_ident
                                                }, to_rootspace: child_pos_offset
                                            );

                                            // Update the layout again, if the events we've sent have triggered a
                                            // relayout.
                                            let update_layout: bool;
                                            {
                                                let top_update_tag = top_node_as_parent.update_tag();
                                                match mbseq.len() {
                                                    0 => top_update_tag.mouse_state.set(MouseState::Untracked),
                                                    _ => top_update_tag.mouse_state.set(MouseState::Tracking(new_pos, mbseq))
                                                }
                                                top_update_tag.child_event_recv.set(top_update_tag.child_event_recv.get() | ChildEventRecv::MOUSE_HOVER);
                                                update_layout = top_update_tag.needs_update(root_id).update_layout;
                                            }
                                            if update_layout {
                                                top_node_as_parent.update_child_layout();
                                            }

                                            continue;
                                        }
                                    }
                                }
                            },
                            // If the cursor is no longer in the node, send the exit events and move to the parent node.
                            false => {
                                let mouse_exit = top_bounds.intersect_line(move_line).1.unwrap_or(new_pos);

                                {
                                    let NodePath{ node, path, .. } = node_stack.top_mut();
                                    try_push_action!(
                                        node, path.iter().cloned(),
                                        (mbd_array, mbdin_array) => NodeEvent::MouseExit {
                                            exit_pos: mouse_exit,
                                            buttons_down: mbd_array,
                                            buttons_down_in_node: mbdin_array
                                        }, to_rootspace: top_bounds_offset
                                    );
                                }

                                {
                                    let top_update_tag = node_stack.top().update_tag();
                                    match mbseq.len() {
                                        0 => top_update_tag.mouse_state.set(MouseState::Untracked),
                                        _ => top_update_tag.mouse_state.set(MouseState::Tracking(new_pos, mbseq))
                                    }
                                }

                                // Send the exit action and mark the parent as hovered, as long as we aren't at the root.
                                if 0 < node_stack.depth() {
                                    let top_parent_offset = node_stack.top_parent_offset();
                                    let child_exit_pos = mouse_exit - top_parent_offset;
                                    let child_ident = node_stack.top_ident();

                                    node_stack.pop();
                                    {
                                        let NodePath{ node, path, .. } = node_stack.top_mut();
                                        try_push_action!(
                                            node, path.iter().cloned(),
                                            (mbd_array, mbdin_array) => NodeEvent::MouseExitChild {
                                                exit_pos: child_exit_pos,
                                                buttons_down: mbd_array,
                                                buttons_down_in_node: mbdin_array,
                                                child: child_ident
                                            }, to_rootspace: top_parent_offset
                                        );
                                    }

                                    let top_update_tag = node_stack.top().update_tag();
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
                        let root = node_stack.move_to_root().node;

                        let top_update_tag = root.update_tag();
                        let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                        top_update_tag.mouse_state.set(MouseState::Hovering(new_pos_windowspace, top_mbseq));
                        continue;
                    }

                    break;
                }

                // Send move events to nodes that are being click-dragged but aren't being hovered.
                node_stack.move_over_flags(ChildEventRecv::MOUSE_BUTTONS, |node, path, node_parent_offset| {
                    let new_pos: Point2<i32>;
                    let (node_needs_move_event, node_old_pos, node_offset): (bool, Point2<i32>, Vector2<i32>);

                    {
                        node_offset = node_parent_offset + node.rect().min().to_vec();
                        let update_tag = node.update_tag();
                        new_pos = new_pos_windowspace - node_offset;

                        node_needs_move_event = update_tag.last_event_stamp.get() != *event_stamp;
                        match update_tag.mouse_state.get() {
                            MouseState::Tracking(old_pos, mbseq) => {
                                update_tag.mouse_state.set(MouseState::Tracking(new_pos, mbseq));
                                node_old_pos = old_pos;
                            },
                            s if node_needs_move_event => panic!("unexpected mouse state: {:?}", s),
                            _ => node_old_pos = Point2::from_value(0xDEDBEEF)
                        }
                    }

                    if node_needs_move_event {
                        try_push_action!(
                            node, path.iter().cloned(),
                            (mbd_array, mbdin_array) => NodeEvent::MouseMove {
                                old_pos: node_old_pos,
                                new_pos: new_pos,
                                in_node: false,
                                buttons_down: mbd_array,
                                buttons_down_in_node: mbdin_array
                            }, to_rootspace: node_offset
                        );
                    }

                    node.update_tag()
                });
            },
            WindowEvent::MouseMove(_) => (),


            WindowEvent::MouseDown(button) => {
                let recv_flags = ChildEventRecv::MOUSE_HOVER | ChildEventRecv::KEYBOARD;
                let button_mask = ChildEventRecv::mouse_button_mask(button);
                node_stack.move_over_flags(recv_flags, |top_node, path, top_parent_offset| {
                    let bounds_rootspace = top_node.rect() + top_parent_offset;
                    let top_node_offset = bounds_rootspace.min.to_vec();
                    let in_node = bounds_rootspace.contains(*mouse_pos);

                    try_push_action!(
                        top_node, path.iter().cloned(),
                        (_, _) => NodeEvent::MouseDown {
                            pos: *mouse_pos - top_node_offset,
                            in_node,
                            button
                        }, to_rootspace: top_node_offset
                    );

                    let top_update_tag = top_node.update_tag();
                    match top_update_tag.mouse_state.get() {
                        MouseState::Untracked     |
                        MouseState::Tracking(..) => (),
                        MouseState::Hovering(mouse_pos, mut top_mbseq) => if in_node {
                            top_update_tag.mouse_state.set(MouseState::Hovering(mouse_pos, *top_mbseq.push_button(button)));
                        }
                    }

                    if in_node {
                        top_update_tag.child_event_recv.set(top_update_tag.child_event_recv.get() | button_mask)
                    }
                    top_update_tag
                });
                mouse_buttons_down.push_button(button, *mouse_pos);

                node_stack.move_to_hover();
                node_stack.drain_to_root(|node, _, _| {
                    let node_update_tag = node.update_tag();
                    node_update_tag.child_event_recv.set(node_update_tag.child_event_recv.get() | button_mask);
                });
            },
            WindowEvent::MouseUp(button) => {
                let button_mask = ChildEventRecv::mouse_button_mask(button);
                let down_pos_rootspace = mouse_buttons_down.contains(button).map(|down| down.down_pos)
                    .unwrap_or(Point2::new(0, 0));
                mouse_buttons_down.release_button(button);

                // Send the mouse up event to the hover node.
                let mut move_to_tracked = true;
                node_stack.move_over_flags(ChildEventRecv::MOUSE_HOVER, |node, path, top_parent_offset| {
                    let bounds_rootspace = node.rect() + top_parent_offset;
                    let node_offset = bounds_rootspace.min().to_vec();
                    let in_node = bounds_rootspace.contains(*mouse_pos);
                    let pressed_in_node = node.update_tag().mouse_state.get().mouse_button_sequence().contains(button);
                    // If the hover node wasn't the one where the mouse was originally pressed, ensure that
                    // we move to the node where it was pressed.
                    move_to_tracked = !pressed_in_node;

                    try_push_action!(
                        node, path.iter().cloned(),
                        (_, _) => NodeEvent::MouseUp {
                            pos: *mouse_pos - node_offset,
                            down_pos: down_pos_rootspace - node_offset,
                            in_node,
                            pressed_in_node,
                            button
                        }, to_rootspace: node_offset
                    );

                    let update_tag = node.update_tag();

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

                // If the hover node wasn't the node where the mouse button was originally pressed,
                // send the move event to original presser.
                if move_to_tracked {
                    node_stack.move_over_flags(button_mask, |node, path, top_parent_offset| {
                        let bounds_rootspace = node.rect() + top_parent_offset;
                        let node_offset = bounds_rootspace.min().to_vec();
                        try_push_action!(
                            node, path.iter().cloned(),
                            (_, _) => NodeEvent::MouseUp {
                                pos: *mouse_pos - node_offset,
                                down_pos: down_pos_rootspace - node_offset,
                                in_node: bounds_rootspace.contains(*mouse_pos),
                                pressed_in_node: true,
                                button
                            }, to_rootspace: node_offset
                        );

                        let update_tag = node.update_tag();

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

                for node in node_stack.nodes() {
                    let update_tag = node.update_tag();
                    update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !button_mask);
                }
            },
            WindowEvent::KeyDown(key) => {
                if let Some(NodePath{ node: focus_node, path, top_parent_offset }) = node_stack.move_to_keyboard_focus() {
                    let bounds_rootspace = focus_node.rect() + top_parent_offset;
                    let node_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(focus_node, path.iter().cloned(), (_, _) => NodeEvent::KeyDown(key, *modifiers), to_rootspace: node_offset);
                }
            },
            WindowEvent::KeyUp(key) => {
                if let Some(NodePath{ node: focus_node, path, top_parent_offset }) = node_stack.move_to_keyboard_focus() {
                    let bounds_rootspace = focus_node.rect() + top_parent_offset;
                    let node_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(focus_node, path.iter().cloned(), (_, _) => NodeEvent::KeyUp(key, *modifiers), to_rootspace: node_offset);
                }
            },
            WindowEvent::Char(c) => {
                if let Some(NodePath{ node: focus_node, path, top_parent_offset }) = node_stack.move_to_keyboard_focus() {
                    let bounds_rootspace = focus_node.rect() + top_parent_offset;
                    let node_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(focus_node, path.iter().cloned(), (_, _) => NodeEvent::Char(c), to_rootspace: node_offset);
                }
            },
            WindowEvent::Timer => {
                let triggered_timers = timer_list.trigger_timers().triggered_timers().to_vec();

                fn timer_node_id(timer: Timer) -> NodeID {
                    timer.node_id
                }
                node_stack.move_over_nodes(
                    triggered_timers.iter().cloned().map(timer_node_id),
                    |node, path, timer_index, top_parent_offset| {
                        let timer = triggered_timers[timer_index];
                        try_push_action!(
                            node,
                            path.iter().cloned(),
                            (_, _) => NodeEvent::Timer {
                                name: timer.name,
                                start_time: timer.start_time,
                                last_trigger: timer.last_trigger,
                                frequency: timer.frequency,
                                times_triggered: timer.times_triggered
                            }, to_rootspace: node.rect().min().to_vec() + top_parent_offset
                        );
                    }
                );
            }
        }

        // Dispatch focus-changing events to the nodes.
        let mut meta_drain = meta_tracker.drain_meta();
        assert_eq!(0, node_ident_stack.len());
        loop {
            let mut take_focus_to = |
                node_stack: &mut NodeStack<_, _, _>,
                node_ident_stack: &[NodeIdent],
                meta_drain: &mut MetaDrain
            | {
                if let Some(NodePath{ node, path, top_parent_offset }) = node_stack.move_to_keyboard_focus() {
                    if path == node_ident_stack {
                        return;
                    }
                    node.update_tag().has_keyboard_focus.set(false);
                    let bounds_rootspace = node.rect() + top_parent_offset;
                    let node_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(node, path.into_iter().cloned() => (*meta_drain), (_, _) => NodeEvent::LoseFocus, to_rootspace: node_offset);

                    for update_tag in node_stack.nodes().map(|n| n.update_tag()) {
                        update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::KEYBOARD);
                    }
                }
                if let Some(NodePath{ node, path, top_parent_offset }) = node_stack.move_to_path(node_ident_stack.iter().cloned()) {
                    node.update_tag().has_keyboard_focus.set(true);
                    let bounds_rootspace = node.rect() + top_parent_offset;
                    let node_offset = bounds_rootspace.min().to_vec();

                    try_push_action!(node, path.into_iter().cloned() => (*meta_drain), (_, _) => NodeEvent::GainFocus, to_rootspace: node_offset);

                    node_stack.pop();
                    for update_tag in node_stack.nodes().map(|n| n.update_tag()) {
                        update_tag.child_event_recv.set(update_tag.child_event_recv.get() | ChildEventRecv::KEYBOARD);
                    }
                }
            };
            let meta_variant = match meta_drain.next() {
                Some(MetaEvent{ source, variant }) => {
                    node_ident_stack.extend(source);
                    variant
                },
                None => break
            };

            match meta_variant {
                MetaEventVariant::FocusChange(focus) => match focus {
                    FocusChange::Remove => {
                        if let Some(NodePath{ node, path, top_parent_offset }) = node_stack.move_to_path(node_ident_stack.iter().cloned()) {
                            if node.update_tag().has_keyboard_focus.get() {
                                node.update_tag().has_keyboard_focus.set(false);
                                let bounds_rootspace = node.rect() + top_parent_offset;
                                let node_offset = bounds_rootspace.min().to_vec();

                                try_push_action!(
                                    node,
                                    path.into_iter().cloned() => (meta_drain),
                                    (_, _) => NodeEvent::LoseFocus, to_rootspace: node_offset
                                );
                                for update_tag in node_stack.nodes().map(|n| n.update_tag()) {
                                    update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::KEYBOARD);
                                }
                            }
                        }
                    },
                    FocusChange::Take => take_focus_to(&mut node_stack, &**node_ident_stack, &mut meta_drain),
                    FocusChange::Next |
                    FocusChange::Prev => {
                        let sibling_dist = match focus {
                            FocusChange::Next => 1,
                            FocusChange::Prev => -1,
                            _ => unreachable!()
                        };

                        node_stack.move_to_path(node_ident_stack.iter().cloned()).unwrap();
                        let mut advance_to_sibling = true;
                        loop {
                            let dist_multiplier = advance_to_sibling as isize;
                            advance_to_sibling = true;
                            match node_stack.move_to_sibling_delta(sibling_dist * dist_multiplier) {
                                Ok(NodePath{ node: sibling, .. }) => match sibling.accepts_focus() {
                                    OnFocus::Accept => {
                                        node_ident_stack.clear();
                                        node_ident_stack.extend(node_stack.ident().iter().cloned());
                                        break;
                                    },
                                    OnFocus::Skip => continue,
                                    OnFocus::FocusChild => {
                                        node_stack.try_push(|top, _| {
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
                                    match node_stack.parent() {
                                        Some(parent) => match parent.on_child_focus_overflow() {
                                            OnFocusOverflow::Wrap => {
                                                let move_to_index = match index_out_of_bounds_cmp {
                                                    Ordering::Less => parent.num_children() - 1,
                                                    Ordering::Greater => 0,
                                                    Ordering::Equal => unreachable!()
                                                };
                                                node_stack.move_to_sibling_index(move_to_index).unwrap();
                                                advance_to_sibling = false;
                                                continue;
                                            },
                                            OnFocusOverflow::Continue => {
                                                node_stack.pop();
                                                continue;
                                            }
                                        },
                                        None => {
                                            match sibling_dist {
                                                -1 => node_stack.try_push(|n, _| n.subtrait_mut().as_parent().and_then(|p| p.child_by_index_mut(p.num_children() - 1))),
                                                1 => node_stack.try_push(|n, _| n.subtrait_mut().as_parent().and_then(|p| p.child_by_index_mut(0))),
                                                _ => unreachable!()
                                            };
                                            advance_to_sibling = false;
                                            continue;
                                        }
                                    }
                                },
                            }
                        }

                        take_focus_to(&mut node_stack, &**node_ident_stack, &mut meta_drain);
                    }
                },
                MetaEventVariant::EventBubble(event_owned) => {
                    if let None = node_stack.move_to_path(node_ident_stack[..node_ident_stack.len() - 1].iter().cloned()) {
                        node_ident_stack.clear();
                        continue;
                    }
                    event_owned.as_borrowed(mouse_buttons_down, |event| {
                        let mut continue_bubble = true;
                        let mut slice_range = node_stack.depth() + 1..;
                        node_stack.drain_to_root_while(|top_node, ident, parent_offset| {
                            let node_offset = parent_offset + top_node.rect().min().to_vec();
                            try_push_action!(
                                top_node,
                                ident.into_iter().cloned() => (meta_drain),
                                (_, _) => event.translate(-node_offset),
                                to_rootspace: node_offset,
                                bubble (false, &mut continue_bubble, &node_ident_stack[slice_range.clone()])
                            );
                            slice_range.start -= 1;
                            continue_bubble
                        });

                        if continue_bubble {
                            if let Some(action) = bubble_fallthrough(event, &node_ident_stack) {
                                actions.push_back(action);
                            }
                        }
                    });
                }
            }
            node_ident_stack.clear();
        }

        // Increment the event stamp. Because new `UpdateTag`s have a default event stampo of 0,
        // make sure our event stamp is never 0.
        *event_stamp = event_stamp.wrapping_add(1);
        if *event_stamp == 0 {
            *event_stamp += 1;
        }

        let mut return_flow = LoopFlow::Continue;
        let mut root_update = node_stack.move_to_root().node.update_tag().needs_update(root_id);
        let mark_active_nodes_redraw = root_update.needs_redraw();

        drop(node_stack);
        if 0 < actions.len() {
            while let Some(action) = actions.pop_front() {
                match on_action(action, root_node, theme) {
                    LoopFlow::Continue => (),
                    LoopFlow::Break(ret) => {
                        return_flow = LoopFlow::Break(ret);
                        break;
                    }
                }
            }
        }
        let redraw_node = match taken_popup_node.as_mut() {
            Some(popup_node) => &mut *popup_node.node,
            None => root_node
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


            // Draw the node tree.
            if mark_active_nodes_redraw || *force_full_redraw {
                let force_full_redraw = *force_full_redraw || renderer.force_full_redraw();

                root_update.render_self |= force_full_redraw;
                root_update.update_child |= force_full_redraw;

                if root_update.render_self || root_update.update_child {
                    {
                        let (frame, base_transform) = renderer.make_frame();
                        let mut frame_rect_stack = FrameRectStack::new(frame, base_transform, theme, node_ident_stack);

                        if let NodeSubtraitMut::Parent(root_as_parent) = redraw_node.subtrait_mut() {
                            update_node_layout(root_id, force_full_redraw, root_as_parent);
                        }
                        if root_update.render_self {
                            redraw_node.render(&mut frame_rect_stack);
                        }
                        if root_update.update_child {
                            if let NodeSubtraitMut::Parent(root_as_parent) = redraw_node.subtrait_mut() {
                                NodeRenderer {
                                    root_id: root_id,
                                    frame: frame_rect_stack,
                                    force_full_redraw: force_full_redraw,
                                    theme
                                }.render_node_children(root_as_parent)
                            }
                        }
                    }

                    renderer.finish_frame(theme);
                    redraw_node.update_tag().mark_updated(root_id);
                }
            }

            renderer.set_size_bounds(redraw_node.size_bounds());
            *force_full_redraw = false;
        });

        drop(redraw_node);
        // Report popups that need to be created
        for (owner_id, popup_node, popup_attributes) in popup_map_insert {
            let popup_id = popup_nodes.insert(owner_id, popup_attributes.ident, popup_node);
            popup_deltas.push(PopupDelta::Create(PopupSummary {
                id: popup_id,
                attributes: popup_attributes
            }));
        }

        // Report popups that need to be destroyed.
        popup_deltas.extend(popup_nodes.popups_removed_by_children()
            .map(|removed_popup_id| PopupDelta::Remove(removed_popup_id)));

        // If we took a popup node out of the popup map, replace it.
        if let Some(taken_popup_node) = taken_popup_node {
            popup_nodes.replace(event_popup_id.unwrap(), taken_popup_node);
        }

        EventLoopResult {
            flow: return_flow,
            wait_until_call_timer: timer_list.time_until_trigger(),
            popup_deltas
        }
    }
}

fn update_node_layout<A, F: RenderFrame>(root_id: RootID, force_full_redraw: bool, node: &mut Parent<A, F>) -> bool {
    // Loop to re-solve node layout, if children break their size bounds. Is 0..4 so that
    // it doesn't enter an infinite loop if children can never be properly solved.
    for _ in 0..4 {
        let Update {
            update_child,
            update_layout,
            ..
        } = node.update_tag().needs_update(root_id);

        if update_layout || force_full_redraw {
            node.update_child_layout();
            node.update_tag().unmark_update_layout();
        }

        let mut children_break_bounds = false;
        if update_child || force_full_redraw {
            node.children_mut(&mut |children_summaries| {
                for summary in children_summaries {
                    let NodeSummary {
                        node: ref mut child_node,
                        ..
                    } = *summary;

                    if let NodeSubtraitMut::Parent(child_node_as_parent) = child_node.subtrait_mut() {
                        children_break_bounds |= update_node_layout(root_id, force_full_redraw, child_node_as_parent);
                    }

                    child_node.update_tag().unmark_update_layout();
                }

                LoopFlow::Continue
            });
        }

        if !children_break_bounds {
            break;
        }
    }

    let node_rect = DimsBox::new(node.rect().dims());
    node.size_bounds().bound_rect(node_rect) != node_rect
}

struct NodeRenderer<'a, F>
    where F: 'a + RenderFrame
{
    root_id: RootID,
    frame: FrameRectStack<'a, F>,
    force_full_redraw: bool,
    theme: &'a F::Theme
}

impl<'a, F> NodeRenderer<'a, F>
    where F: 'a + RenderFrame
{
    fn render_node_children<A>(&mut self, parent: &mut Parent<A, F>) {
        parent.children_mut(&mut |children_summaries| {
            for summary in children_summaries {
                let NodeSummary {
                    node: ref mut child_node,
                    ident,
                    rect: child_rect,
                    ..
                } = *summary;

                let mut root_update = child_node.update_tag().needs_update(self.root_id);
                root_update.render_self |= self.force_full_redraw;
                root_update.update_child |= self.force_full_redraw;
                let Update {
                    render_self,
                    update_child,
                    update_layout: _,
                    update_timer: _
                } = root_update;

                match child_node.subtrait_mut() {
                    NodeSubtraitMut::Parent(child_node_as_parent) => {
                        let mut child_frame = self.frame.enter_child_node(ident);
                        let mut child_frame = child_frame.enter_child_rect(child_rect);

                        if render_self {
                            child_node_as_parent.render(&mut child_frame);
                        }
                        if update_child {
                            NodeRenderer {
                                root_id: self.root_id,
                                frame: child_frame,
                                force_full_redraw: self.force_full_redraw,
                                theme: self.theme
                            }.render_node_children(child_node_as_parent);
                        }
                    },
                    NodeSubtraitMut::Node(child_node) => {
                        if render_self {
                            let mut child_frame = self.frame.enter_child_node(ident);
                            let mut child_frame = child_frame.enter_child_rect(child_rect);

                            child_node.render(&mut child_frame);
                        }
                    }
                }

                child_node.update_tag().mark_updated(self.root_id);
            }

            LoopFlow::Continue
        });
    }
}
