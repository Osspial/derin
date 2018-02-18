#![feature(conservative_impl_trait, range_contains, nll)]

extern crate cgmath;
extern crate cgmath_geometry;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate dct;
extern crate arrayvec;
extern crate itertools;

pub mod timer;
pub mod tree;
pub mod event;
pub mod render;
mod mbseq;
mod node_stack;
mod meta_tracker;

use arrayvec::ArrayVec;

use cgmath::{EuclideanSpace, Point2, Vector2, Array, Bounded};
use cgmath_geometry::{GeoBox, DimsBox, Segment};

use std::cmp::Ordering;
use std::time::Duration;
use std::marker::PhantomData;
use std::collections::VecDeque;

use tree::*;
use timer::{Timer, TimerList};
use event::{NodeEvent, MouseDown, FocusChange};
use render::{Renderer, RenderFrame, FrameRectStack};
use mbseq::MouseButtonSequenceTrackPos;
use node_stack::{NodeStackBase, NodePath, NodeStack};
use meta_tracker::{MetaEventTracker, MetaDrain, MetaEvent, MetaEventVariant};
use dct::buttons::{MouseButton, Key, ModifierKeys};

pub struct Root<A, N, F>
    where N: Node<A, F> + 'static,
          A: 'static,
          F: RenderFrame + 'static
{
    id: RootID,
    mouse_pos: Point2<i32>,
    mouse_buttons_down: MouseButtonSequenceTrackPos,
    actions: VecDeque<A>,
    node_stack_base: NodeStackBase<A, F>,
    force_full_redraw: bool,
    event_stamp: u32,
    node_ident_stack: Vec<NodeIdent>,
    meta_tracker: MetaEventTracker,
    timer_list: TimerList,
    pub root_node: N,
    pub theme: F::Theme,
    _marker: PhantomData<*const F>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    MouseMove(Point2<i32>),
    MouseEnter(Point2<i32>),
    MouseExit(Point2<i32>),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    WindowResize(DimsBox<Point2<u32>>),
    KeyDown(Key, ModifierKeys),
    KeyUp(Key, ModifierKeys),
    Char(char),
    Timer
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopFlow<R> {
    Continue,
    Break(R)
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventLoopResult<R> {
    pub flow: LoopFlow<R>,
    pub wait_until_call_timer: Option<Duration>
}

impl<A, N, F> Root<A, N, F>
    where N: Node<A, F>,
          F: RenderFrame
{
    #[inline]
    pub fn new(mut root_node: N, theme: F::Theme, dims: DimsBox<Point2<u32>>) -> Root<A, N, F> {
        // TODO: DRAW ROOT AND DO INITIAL LAYOUT
        *root_node.bounds_mut() = dims.cast().unwrap_or(DimsBox::max_value()).into();
        Root {
            id: RootID::new(),
            mouse_pos: Point2::new(-1, -1),
            mouse_buttons_down: MouseButtonSequenceTrackPos::new(),
            actions: VecDeque::new(),
            node_stack_base: NodeStackBase::new(),
            force_full_redraw: false,
            event_stamp: 1,
            node_ident_stack: Vec::new(),
            meta_tracker: MetaEventTracker::default(),
            timer_list: TimerList::new(None),
            root_node, theme,
            _marker: PhantomData
        }
    }

    pub fn run_forever<E, AF, BF, R, G>(&mut self, mut gen_events: E, mut on_action: AF, mut bubble_fallthrough: BF, renderer: &mut R) -> Option<G>
        where E: FnMut(&mut FnMut(WindowEvent) -> EventLoopResult<G>) -> Option<G>,
              AF: FnMut(A, &mut N, &mut F::Theme) -> LoopFlow<G>,
              BF: FnMut(NodeEvent, &[NodeIdent]) -> Option<A>,
              R: Renderer<Frame=F>
    {
        let Root {
            id: root_id,
            ref mut mouse_pos,
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
            ..
        } = *self;
        // Initialize node stack to root.
        let mut node_stack = node_stack_base.use_stack(root_node, root_id);
        let mut current_cursor_icon = Default::default();

        gen_events(&mut |event| {
            let mut set_cursor_pos = None;
            let mut set_cursor_icon = None;

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
                    $event:expr $(, to_rootspace: $node_offset:expr)* $(, bubble ($bubble:expr, $bubble_store:expr, $bubble_path:expr))*
                ) => {{
                    let event = $event;
                    let node_offset = $($node_offset +)* Vector2::from_value(0);
                    let event_ops = $node.on_node_event(
                        event,
                        if_tokens!(($($bubble_path)*) {
                            $($bubble_path)*
                        } else {
                            &[]
                        })
                    );

                    let ref mut meta_tracker = if_tokens!(($($meta_tracker)*) {
                        $($meta_tracker)*
                    } else {*meta_tracker});
                    if let Some(action) = event_ops.action {
                        actions.push_back(action);
                    }
                    if let Some(focus) = event_ops.focus {
                        meta_tracker.push_focus(focus, $path);
                    }
                    if event_ops.bubble $(& $bubble)* {
                        meta_tracker.push_bubble(
                            event.translate(node_offset).into(),
                            $path
                        );
                    }
                    set_cursor_pos = set_cursor_pos.or(event_ops.cursor_pos.map(|p| p $(+ $node_offset)*));
                    set_cursor_icon = set_cursor_icon.or(event_ops.cursor_icon);
                    $(*$bubble_store = event_ops.bubble;)*

                    let node_update_tag = $node.update_tag();
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
                    *node_stack.top_mut().node.bounds_mut() = new_size.cast::<i32>().unwrap_or(DimsBox::max_value()).into();
                    *force_full_redraw = true;
                },
                WindowEvent::MouseEnter(enter_pos) => {
                    let NodePath{ node: root_node, path: root_path } = node_stack.move_to_root();

                    let (mbd_array, mbdin_array) = mouse_button_arrays!(root_node.update_tag(), Vector2::new(0, 0));
                    let top_update_tag = try_push_action!(
                        root_node, root_path.iter().cloned(),
                        NodeEvent::MouseEnter {
                            enter_pos,
                            buttons_down: &mbd_array,
                            buttons_down_in_node: &mbdin_array
                        }
                    );

                    let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                    top_update_tag.mouse_state.set(MouseState::Hovering(enter_pos, top_mbseq));
                },
                WindowEvent::MouseExit(exit_pos) => {
                    node_stack.drain_to_root(|node, path, parent_offset| {
                        let node_offset = node.bounds().min().to_vec() + parent_offset;
                        let (mbd_array, mbdin_array) = mouse_button_arrays!(node.update_tag(), node_offset);

                        let update_tag = try_push_action!(
                            node, path.iter().cloned(),
                            NodeEvent::MouseExit {
                                exit_pos: exit_pos - node_offset,
                                buttons_down: &mbd_array,
                                buttons_down_in_node: &mbdin_array
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


                WindowEvent::MouseMove(new_pos_windowspace) => {
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
                            let (mbd_array, mbdin_array) = mouse_button_arrays!(update_tag_copy, top_bounds_offset);
                            {
                                let NodePath{ node, path } = node_stack.top_mut();
                                try_push_action!(
                                    node, path.iter().cloned(),
                                    NodeEvent::MouseMove {
                                        old: node_old_pos,
                                        new: new_pos,
                                        in_node: true,
                                        buttons_down: &mbd_array,
                                        buttons_down_in_node: &mbdin_array
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
                                    let NodePath{ node: top_node, path: top_path } = node_stack.top_mut();
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
                                                let child_mbdin_array: ArrayVec<[_; 5]> = child_mbdin
                                                .into_iter().filter_map(|b| mouse_buttons_down.contains(b)).collect();
                                                try_push_action!(
                                                    child, top_path.iter().cloned().chain(Some(child_ident)),
                                                    NodeEvent::MouseEnter {
                                                        enter_pos: enter_pos - child_pos_offset,
                                                        buttons_down: &mbd_array,
                                                        buttons_down_in_node: &child_mbdin_array
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
                                                    NodeEvent::MouseEnterChild {
                                                        enter_pos,
                                                        buttons_down: &mbd_array,
                                                        buttons_down_in_node: &mbdin_array,
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
                                        let NodePath{ node, path } = node_stack.top_mut();
                                        try_push_action!(
                                            node, path.iter().cloned(),
                                            NodeEvent::MouseExit {
                                                exit_pos: mouse_exit,
                                                buttons_down: &mbd_array,
                                                buttons_down_in_node: &mbdin_array
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
                                            let NodePath{ node, path } = node_stack.top_mut();
                                            try_push_action!(
                                                node, path.iter().cloned(),
                                                NodeEvent::MouseExitChild {
                                                    exit_pos: child_exit_pos,
                                                    buttons_down: &mbd_array,
                                                    buttons_down_in_node: &mbdin_array,
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
                        let (mbd_array, mbdin_array);
                        let (node_needs_move_event, node_old_pos, node_offset): (bool, Point2<i32>, Vector2<i32>);

                        {
                            node_offset = node_parent_offset + node.bounds().min().to_vec();
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

                            let mbds = mouse_button_arrays!(update_tag, node_offset);
                            mbd_array = mbds.0;
                            mbdin_array = mbds.1;
                        }

                        if node_needs_move_event {
                            try_push_action!(
                                node, path.iter().cloned(),
                                NodeEvent::MouseMove {
                                    old: node_old_pos,
                                    new: new_pos,
                                    in_node: false,
                                    buttons_down: &mbd_array,
                                    buttons_down_in_node: &mbdin_array
                                }, to_rootspace: node_offset
                            );
                        }

                        node.update_tag()
                    });
                },


                WindowEvent::MouseDown(button) => {
                    let recv_flags = ChildEventRecv::MOUSE_HOVER | ChildEventRecv::KEYBOARD;
                    let button_mask = ChildEventRecv::mouse_button_mask(button);
                    node_stack.move_over_flags(recv_flags, |top_node, path, top_parent_offset| {
                        let bounds_rootspace = top_node.bounds() + top_parent_offset;
                        let top_node_offset = bounds_rootspace.min.to_vec();
                        let in_node = bounds_rootspace.contains(*mouse_pos);

                        try_push_action!(
                            top_node, path.iter().cloned(),
                            NodeEvent::MouseDown {
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
                        let bounds_rootspace = node.bounds() + top_parent_offset;
                        let node_offset = bounds_rootspace.min().to_vec();
                        let in_node = bounds_rootspace.contains(*mouse_pos);
                        let pressed_in_node = node.update_tag().mouse_state.get().mouse_button_sequence().contains(button);
                        // If the hover node wasn't the one where the mouse was originally pressed, ensure that
                        // we move to the node where it was pressed.
                        move_to_tracked = !pressed_in_node;

                        try_push_action!(
                            node, path.iter().cloned(),
                            NodeEvent::MouseUp {
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
                            let bounds_rootspace = node.bounds() + top_parent_offset;
                            let node_offset = bounds_rootspace.min().to_vec();
                            try_push_action!(
                                node, path.iter().cloned(),
                                NodeEvent::MouseUp {
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
                WindowEvent::KeyDown(key, modifiers) => {
                    if let Some(NodePath{ node: focus_node, path }) = node_stack.move_to_keyboard_focus() {
                        try_push_action!(focus_node, path.iter().cloned(), NodeEvent::KeyDown(key, modifiers));
                    }
                },
                WindowEvent::KeyUp(key, modifiers) => {
                    if let Some(NodePath{ node: focus_node, path }) = node_stack.move_to_keyboard_focus() {
                        try_push_action!(focus_node, path.iter().cloned(), NodeEvent::KeyUp(key, modifiers));
                    }
                },
                WindowEvent::Char(c) => {
                    if let Some(NodePath{ node: focus_node, path }) = node_stack.move_to_keyboard_focus() {
                        try_push_action!(focus_node, path.iter().cloned(), NodeEvent::Char(c));
                    }
                },
                WindowEvent::Timer => {
                    let triggered_timers = timer_list.trigger_timers().triggered_timers().to_vec();

                    fn timer_node_id(timer: Timer) -> NodeID {
                        timer.node_id
                    }
                    node_stack.move_over_nodes(
                        triggered_timers.iter().cloned().map(timer_node_id),
                        |node, path, timer_index, _| {
                            let timer = triggered_timers[timer_index];
                            try_push_action!(
                                node,
                                path.iter().cloned(),
                                NodeEvent::Timer {
                                    name: timer.name,
                                    start_time: timer.start_time,
                                    last_trigger: timer.last_trigger,
                                    frequency: timer.frequency,
                                    times_triggered: timer.times_triggered
                                }
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
                    if let Some(NodePath{ node, path }) = node_stack.move_to_keyboard_focus() {
                        if path == node_ident_stack {
                            return;
                        }
                        node.update_tag().has_keyboard_focus.set(false);
                        try_push_action!(node, path.into_iter().cloned() => (*meta_drain), NodeEvent::LoseFocus);

                        for update_tag in node_stack.nodes().map(|n| n.update_tag()) {
                            update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::KEYBOARD);
                        }
                    }
                    if let Some(NodePath{ node, path }) = node_stack.move_to_path(node_ident_stack.iter().cloned()) {
                        node.update_tag().has_keyboard_focus.set(true);
                        try_push_action!(node, path.into_iter().cloned() => (*meta_drain), NodeEvent::GainFocus);

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
                            if let Some(NodePath{ node, path }) = node_stack.move_to_path(node_ident_stack.iter().cloned()) {
                                if node.update_tag().has_keyboard_focus.get() {
                                    node.update_tag().has_keyboard_focus.set(false);
                                    try_push_action!(
                                        node,
                                        path.into_iter().cloned() => (meta_drain),
                                        NodeEvent::LoseFocus
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
                                let node_offset = parent_offset + top_node.bounds().min().to_vec();
                                try_push_action!(
                                    top_node,
                                    ident.into_iter().cloned() => (meta_drain),
                                    event.translate(-node_offset),
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
            let root = node_stack.move_to_root().node;
            let mut root_update = root.update_tag().needs_update(root_id);
            let mark_active_nodes_redraw = root_update.needs_redraw();

            if 0 < actions.len() {
                while let Some(action) = actions.pop_front() {
                    match on_action(action, root, theme) {
                        LoopFlow::Continue => (),
                        LoopFlow::Break(ret) => {
                            return_flow = LoopFlow::Break(ret);
                            break;
                        }
                    }
                }
            }

            if let Some(cursor_pos) = set_cursor_pos {
                renderer.set_cursor_pos(cursor_pos);
            }
            if let Some(cursor_icon) = set_cursor_icon {
                if cursor_icon != current_cursor_icon {
                    renderer.set_cursor_icon(set_cursor_icon.unwrap_or(Default::default()));
                    current_cursor_icon = cursor_icon;
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

                        if let NodeSubtraitMut::Parent(root_as_parent) = root.subtrait_mut() {
                            if root_update.update_layout {
                                root_as_parent.update_child_layout();
                            }
                        }
                        if root_update.render_self {
                            root.render(&mut frame_rect_stack);
                        }
                        if root_update.update_child {
                            if let NodeSubtraitMut::Parent(root_as_parent) = root.subtrait_mut() {
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
                    root.update_tag().mark_updated(root_id);
                }
            }

            *force_full_redraw = false;

            EventLoopResult {
                flow: return_flow,
                wait_until_call_timer: timer_list.time_until_trigger()
            }
        })
    }
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
                    update_layout,
                    update_timer: _
                } = root_update;

                match child_node.subtrait_mut() {
                    NodeSubtraitMut::Parent(child_node_as_parent) => {
                        let mut child_frame = self.frame.enter_child_node(ident);
                        let mut child_frame = child_frame.enter_child_rect(child_rect);

                        if update_layout {
                            child_node_as_parent.update_child_layout();
                        }
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

impl<T> Into<Option<T>> for LoopFlow<T> {
    #[inline]
    fn into(self) -> Option<T> {
        match self {
            LoopFlow::Continue => None,
            LoopFlow::Break(t) => Some(t)
        }
    }
}
