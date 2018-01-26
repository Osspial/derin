#![feature(conservative_impl_trait, nll)]

extern crate cgmath;
extern crate cgmath_geometry;
#[macro_use]
extern crate bitflags;
extern crate dct;
extern crate arrayvec;
extern crate itertools;

pub mod tree;
pub mod event;
pub mod render;
mod mbseq;
mod node_stack;
mod focus_tracker;

use arrayvec::ArrayVec;

use cgmath::{EuclideanSpace, Point2, Vector2, Array};
use cgmath_geometry::{GeoBox, DimsBox, BoundBox, Segment};

use std::marker::PhantomData;
use std::collections::VecDeque;

use tree::*;
use event::{NodeEvent, FocusChange};
use render::{Renderer, RenderFrame, FrameRectStack};
use mbseq::MouseButtonSequence;
use node_stack::{NodeStackBase, NodePath};
use focus_tracker::KeyboardFocusTracker;
use dct::buttons::{MouseButton, Key};

pub struct Root<A, N, F>
    where N: Node<A, F> + 'static,
          A: 'static,
          F: RenderFrame + 'static
{
    id: RootID,
    mouse_pos: Point2<i32>,
    mouse_buttons_down: MouseButtonSequence,
    actions: VecDeque<A>,
    node_stack_base: NodeStackBase<A, F>,
    force_full_redraw: bool,
    event_stamp: u32,
    node_ident_stack: Vec<NodeIdent>,
    focus_tracker: KeyboardFocusTracker,
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
    KeyDown(Key),
    KeyUp(Key),
    Char(char)
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopFlow<R> {
    Continue,
    Break(R)
}

impl<A, N, F> Root<A, N, F>
    where N: Node<A, F>,
          F: RenderFrame
{
    #[inline]
    pub fn new(mut root_node: N, theme: F::Theme, dims: DimsBox<Point2<u32>>) -> Root<A, N, F> {
        // TODO: DRAW ROOT AND DO INITIAL LAYOUT
        *root_node.bounds_mut() = dims.into();
        Root {
            id: RootID::new(),
            mouse_pos: Point2::new(-1, -1),
            mouse_buttons_down: MouseButtonSequence::new(),
            actions: VecDeque::new(),
            node_stack_base: NodeStackBase::new(),
            force_full_redraw: false,
            event_stamp: 1,
            node_ident_stack: Vec::new(),
            focus_tracker: KeyboardFocusTracker::default(),
            root_node, theme,
            _marker: PhantomData
        }
    }

    pub fn run_forever<E, AF, R, G>(&mut self, mut gen_events: E, mut on_action: AF, renderer: &mut R) -> Option<G>
        where E: FnMut(&mut FnMut(WindowEvent) -> LoopFlow<G>) -> Option<G>,
              AF: FnMut(A, &mut N, &mut F::Theme) -> LoopFlow<G>,
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
            ref mut focus_tracker,
            ref mut root_node,
            ref mut theme,
            ..
        } = *self;
        // Initialize node stack to root.
        let mut node_stack = node_stack_base.use_stack(root_node);

        gen_events(&mut |event| {
            let mut mark_active_nodes_redraw = false;

            macro_rules! mouse_button_arrays {
                ($update_tag:expr) => {{
                    let mbd_array: ArrayVec<[_; 5]> = mouse_buttons_down.into_iter().collect();
                    let mbdin_array: ArrayVec<[_; 5]> = $update_tag.mouse_state.get().mouse_button_sequence().into_iter().collect();
                    (mbd_array, mbdin_array)
                }}
            }

            macro_rules! try_push_action {
                ($node:expr, $path:expr, $event:expr) => {{
                    try_push_action!($node, $path => (focus_tracker), $event)
                }};
                ($node:expr, $path:expr => ($focus_tracker:expr), $event:expr) => {{
                    let event = $event;
                    let event_ops = $node.on_node_event(event, &[]);
                    if let Some(action) = event_ops.action {
                        actions.push_back(action);
                    }
                    if let Some(focus) = event_ops.focus {
                        $focus_tracker.push_focus(focus, $path);
                    }
                }};
            }

            macro_rules! mark_if_needs_update {
                ($node:expr) => {{
                    let node_update_tag = $node.update_tag();
                    let node_update = node_update_tag.needs_update(root_id);
                    let no_update = Update{ render_self: false, update_child: false, update_layout: false };
                    if node_update != no_update {
                        mark_active_nodes_redraw = true;
                    }
                    if mark_active_nodes_redraw {
                        node_update_tag.mark_update_child_immutable();
                    }
                    node_update_tag.last_event_stamp.set(*event_stamp);

                    node_update_tag
                }}
            }

            match event {
                WindowEvent::WindowResize(new_size) => {
                    // Resize the window, forcing a full redraw.

                    node_stack.move_to_root();
                    *node_stack.top_mut().node.bounds_mut() = new_size.into();
                    *force_full_redraw = true;
                },
                WindowEvent::MouseEnter(enter_pos) => {
                    let NodePath{ node: root_node, path: root_path } = node_stack.move_to_root();

                    let (mbd_array, mbdin_array) = mouse_button_arrays!(root_node.update_tag());
                    try_push_action!{
                        root_node, root_path.iter().cloned(),
                        NodeEvent::MouseEnter {
                            enter_pos,
                            buttons_down: &mbd_array,
                            buttons_down_in_node: &mbdin_array
                        }
                    }

                    let top_update_tag = mark_if_needs_update!(root_node);
                    let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                    top_update_tag.mouse_state.set(MouseState::Hovering(enter_pos, top_mbseq));
                },
                WindowEvent::MouseExit(exit_pos) => {
                    node_stack.drain_to_root(|node, path, parent_offset| {
                        let node_offset = node.bounds().min().to_vec() + parent_offset;
                        let (mbd_array, mbdin_array) = mouse_button_arrays!(node.update_tag());

                        try_push_action!{
                            node, path.iter().cloned(),
                            NodeEvent::MouseExit {
                                exit_pos: exit_pos - node_offset.cast::<i32>().unwrap_or(Vector2::from_value(i32::max_value())),
                                buttons_down: &mbd_array,
                                buttons_down_in_node: &mbdin_array
                            }
                        }

                        let update_tag = mark_if_needs_update!(node);

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
                        let top_bounds_windowspace = node_stack.top_bounds_offset().cast::<i32>().unwrap_or(
                            BoundBox::new2(i32::max_value(), i32::max_value(), i32::max_value(), i32::max_value())
                        );
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
                            let (mbd_array, mbdin_array) = mouse_button_arrays!(update_tag_copy);
                            {
                                let NodePath{ node, path } = node_stack.top_mut();
                                try_push_action!{
                                    node, path.iter().cloned(),
                                    NodeEvent::MouseMove {
                                        old: node_old_pos,
                                        new: new_pos,
                                        in_node: true,
                                        buttons_down: &mbd_array,
                                        buttons_down_in_node: &mbdin_array
                                    }
                                }
                                mark_if_needs_update!(node);
                            }

                            // Get the bounds of the node after the node has potentially been moved by the
                            // move action.
                            let new_top_bounds_windowspace = node_stack.top_bounds_offset().cast::<i32>().unwrap_or(
                                BoundBox::new2(i32::max_value(), i32::max_value(), i32::max_value(), i32::max_value())
                            );
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

                                            if let Some(new_pos_unsigned) = new_pos.cast::<u32>() {
                                                struct EnterChildData {
                                                    child_ident: NodeIdent,
                                                    enter_pos: Point2<i32>
                                                }
                                                let mut enter_child: Option<EnterChildData> = None;

                                                // Figure out if the cursor has moved into a child node, and send the relevant events if
                                                // we have.
                                                if let Some(child_summary) = top_node_as_parent.child_by_point_mut(new_pos_unsigned) {
                                                    let NodeSummary {
                                                        node: child,
                                                        rect: child_bounds,
                                                        ident: child_ident,
                                                        ..
                                                    } = child_summary;

                                                    let child_pos_offset = child_bounds.min().to_vec().cast::<i32>()
                                                        .unwrap_or(Vector2::from_value(i32::max_value()));

                                                    // Find the exact location where the cursor entered the child node. This is
                                                    // done in the child's parent's coordinate space (i.e. the currently hovered
                                                    // node), and is translated to the child's coordinate space when we enter the
                                                    // child.
                                                    let enter_pos = child_bounds.cast::<i32>()
                                                        .and_then(|bounds| bounds.intersect_line(move_line).0)
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
                                                    let child_mbdin_array: ArrayVec<[_; 5]> = child_mbdin.into_iter().collect();
                                                    try_push_action!{
                                                        child, top_path.iter().cloned().chain(Some(child_ident)),
                                                        NodeEvent::MouseEnter {
                                                            enter_pos: enter_pos - child_pos_offset,
                                                            buttons_down: &mbd_array,
                                                            buttons_down_in_node: &child_mbdin_array
                                                        }
                                                    }

                                                    // Store the information relating to the child we entered,
                                                    enter_child = Some(EnterChildData{ child_ident, enter_pos });

                                                    // We `continue` the loop after this, but the continue is handled by the
                                                    // `enter_child` check below. `mark_if_needs_update` is called after the
                                                    // continue.
                                                }

                                                if let Some(EnterChildData{child_ident, enter_pos}) = enter_child {
                                                    // SEND CHILD ENTER ACTION
                                                    try_push_action!{
                                                        top_node_as_parent, top_path.iter().cloned(),
                                                        NodeEvent::MouseEnterChild {
                                                            enter_pos,
                                                            buttons_down: &mbd_array,
                                                            buttons_down_in_node: &mbdin_array,
                                                            child: child_ident
                                                        }
                                                    }

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
                                                    mark_if_needs_update!(top_node_as_parent);
                                                    if update_layout {
                                                        top_node_as_parent.update_child_layout();
                                                    }

                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                },
                                // If the cursor is no longer in the node, send the exit events and move to the parent node.
                                false => {
                                    let mouse_exit = top_bounds.intersect_line(move_line).1.unwrap_or(new_pos);

                                    {
                                        let NodePath{ node, path } = node_stack.top_mut();
                                        try_push_action!{
                                            node, path.iter().cloned(),
                                            NodeEvent::MouseExit {
                                                exit_pos: mouse_exit,
                                                buttons_down: &mbd_array,
                                                buttons_down_in_node: &mbdin_array
                                            }
                                        }
                                    }

                                    mark_if_needs_update!(node_stack.top());

                                    {
                                        let top_update_tag = node_stack.top().update_tag();
                                        match mbseq.len() {
                                            0 => top_update_tag.mouse_state.set(MouseState::Untracked),
                                            _ => top_update_tag.mouse_state.set(MouseState::Tracking(new_pos, mbseq))
                                        }
                                    }

                                    // Send the exit action and mark the parent as hovered, as long as we aren't at the root.
                                    if 0 < node_stack.depth() {
                                        let child_exit_pos = mouse_exit - node_stack.top_parent_offset().cast::<i32>().unwrap_or(Vector2::from_value(i32::max_value()));
                                        let child_ident = node_stack.top_ident();

                                        node_stack.pop();
                                        {
                                            let NodePath{ node, path } = node_stack.top_mut();
                                            try_push_action!{
                                                node, path.iter().cloned(),
                                                NodeEvent::MouseExitChild {
                                                    exit_pos: child_exit_pos,
                                                    buttons_down: &mbd_array,
                                                    buttons_down_in_node: &mbdin_array,
                                                    child: child_ident
                                                }
                                            }
                                        }

                                        let top_update_tag = node_stack.top().update_tag();
                                        let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                                        top_update_tag.mouse_state.set(MouseState::Hovering(child_exit_pos, top_mbseq));
                                        top_update_tag.child_event_recv.set(top_update_tag.child_event_recv.get() & !ChildEventRecv::MOUSE_HOVER);

                                        // `mark_if_needs_update` called after continue.
                                        continue;
                                    }
                                }
                            }
                        } else if let MouseState::Untracked = update_tag_copy.mouse_state.get() {
                            // If we enter an untracked state, that means we've recieved a MouseMove event without a MouseEnter
                            // event. So, set the root as hover and re-calculate from there.
                            let root = node_stack.move_to_root().node;

                            let top_update_tag = mark_if_needs_update!(root);
                            let top_mbseq = top_update_tag.mouse_state.get().mouse_button_sequence();
                            top_update_tag.mouse_state.set(MouseState::Hovering(new_pos_windowspace, top_mbseq));
                            continue;
                        } else {
                            // We told the stack to move to the hover node. If that's not where we are, something went
                            // *very* wrong.
                            panic!("unexpected mouse state: {:?}", update_tag_copy.mouse_state.get())
                        }

                        break;
                    }

                    // Send move events to nodes that are being click-dragged but aren't being hovered.
                    node_stack.move_over_flags(ChildEventRecv::MOUSE_BUTTONS, |node, path, node_parent_offset| {
                        let new_pos: Point2<i32>;
                        let (mbd_array, mbdin_array);
                        let (node_needs_move_event, node_old_pos, node_offset): (bool, Point2<i32>, Vector2<i32>);

                        {
                            node_offset = (node_parent_offset + node.bounds().min().to_vec()).cast::<i32>().unwrap_or(Vector2::from_value(i32::max_value()));
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

                            let mbds = mouse_button_arrays!(update_tag);
                            mbd_array = mbds.0;
                            mbdin_array = mbds.1;
                        }

                        if node_needs_move_event {
                            try_push_action!{
                                node, path.iter().cloned(),
                                NodeEvent::MouseMove {
                                    old: node_old_pos,
                                    new: new_pos,
                                    in_node: false,
                                    buttons_down: &mbd_array,
                                    buttons_down_in_node: &mbdin_array
                                }
                            }
                            mark_if_needs_update!(node);
                        }

                        node.update_tag()
                    });
                },


                WindowEvent::MouseDown(button) => {
                    node_stack.move_to_hover();

                    let button_mask = ChildEventRecv::mouse_button_mask(button);
                    {
                        let NodePath{ node: top_node, path } = node_stack.top_mut();
                        let top_node_offset = top_node.bounds().min().cast().unwrap_or(Point2::new(i32::max_value(), i32::max_value())).to_vec();
                        try_push_action!{
                            top_node, path.iter().cloned(),
                            NodeEvent::MouseDown {
                                pos: *mouse_pos + top_node_offset,
                                button
                            }
                        }
                        mark_if_needs_update!(top_node);
                        mouse_buttons_down.push_button(button);

                        let top_update_tag = top_node.update_tag();
                        match top_update_tag.mouse_state.get() {
                            MouseState::Untracked     |
                            MouseState::Tracking(..) => unreachable!(),
                            MouseState::Hovering(mouse_pos, mut top_mbseq) => {
                                top_update_tag.mouse_state.set(MouseState::Hovering(mouse_pos, *top_mbseq.push_button(button)))
                            }
                        }

                        top_update_tag.child_event_recv.set(top_update_tag.child_event_recv.get() | button_mask);
                    }

                    node_stack.drain_to_root(|node, _, _| {
                        let node_update_tag = node.update_tag();
                        node_update_tag.child_event_recv.set(node_update_tag.child_event_recv.get() | button_mask);
                    });
                },
                WindowEvent::MouseUp(button) => {
                    let button_mask = ChildEventRecv::mouse_button_mask(button);
                    mouse_buttons_down.release_button(button);

                    // Send the mouse up event to the hover node.
                    let mut move_to_tracked = true;
                    node_stack.move_over_flags(ChildEventRecv::MOUSE_HOVER, |node, path, top_parent_offset| {
                        let bounds = node.bounds() + top_parent_offset;
                        let in_node = mouse_pos.cast::<u32>().map(|p| bounds.contains(p)).unwrap_or(false);
                        let pressed_in_node = node.update_tag().mouse_state.get().mouse_button_sequence().contains(button);
                        // If the hover node wasn't the one where the mouse was originally pressed, ensure that
                        // we move to the node where it was pressed.
                        move_to_tracked = !pressed_in_node;

                        try_push_action!{
                            node, path.iter().cloned(),
                            NodeEvent::MouseUp {
                                pos: *mouse_pos,
                                in_node,
                                pressed_in_node,
                                button
                            }
                        }
                        mark_if_needs_update!(node);

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
                            let bounds = node.bounds() + top_parent_offset;
                            try_push_action!{
                                node, path.iter().cloned(),
                                NodeEvent::MouseUp {
                                    pos: *mouse_pos,
                                    in_node: mouse_pos.cast::<u32>().map(|p| bounds.contains(p)).unwrap_or(false),
                                    pressed_in_node: true,
                                    button
                                }
                            }
                            mark_if_needs_update!(node);

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
                    if let Some(NodePath{ node: focus_node, path }) = node_stack.move_to_keyboard_focus() {
                        try_push_action!(focus_node, path.iter().cloned(), NodeEvent::KeyDown(key));
                        mark_if_needs_update!(focus_node);
                    }
                },
                WindowEvent::KeyUp(key) => {
                    if let Some(NodePath{ node: focus_node, path }) = node_stack.move_to_keyboard_focus() {
                        try_push_action!(focus_node, path.iter().cloned(), NodeEvent::KeyUp(key));
                        mark_if_needs_update!(focus_node);
                    }
                },
                WindowEvent::Char(c) => {
                    if let Some(NodePath{ node: focus_node, path }) = node_stack.move_to_keyboard_focus() {
                        try_push_action!(focus_node, path.iter().cloned(), NodeEvent::Char(c));
                        mark_if_needs_update!(focus_node);
                    }
                },
            }

            {
                let mut focus_drain = focus_tracker.drain_focus();
                assert_eq!(0, node_ident_stack.len());
                loop {
                    let focus = match focus_drain.next() {
                        Some((focus, ident_iter)) => {
                            node_ident_stack.extend(ident_iter);
                            focus
                        },
                        None => break
                    };
                    match focus {
                        FocusChange::Remove => {
                            if let Some(NodePath{ node, path }) = node_stack.move_to_path(node_ident_stack.iter().cloned()) {
                                if node.update_tag().has_keyboard_focus.get() {
                                    try_push_action!(
                                        node,
                                        path.into_iter().cloned() => (focus_drain),
                                        NodeEvent::LoseFocus
                                    );
                                    let update_tag = mark_if_needs_update!(node);
                                    update_tag.has_keyboard_focus.set(false);
                                    for update_tag in node_stack.nodes().map(|n| n.update_tag()) {
                                        update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::KEYBOARD);
                                    }
                                }
                            }
                        },
                        FocusChange::Take => {
                            if let Some(NodePath{ node, path }) = node_stack.move_to_keyboard_focus() {
                                if path == &**node_ident_stack {
                                    continue;
                                }
                                try_push_action!(node, path.into_iter().cloned() => (focus_drain), NodeEvent::LoseFocus);
                            }
                            if let Some(NodePath{ node, path}) = node_stack.move_to_path(node_ident_stack.iter().cloned()) {
                                try_push_action!(node, path.into_iter().cloned() => (focus_drain), NodeEvent::GainFocus);
                            }
                        }
                        _ => unimplemented!()
                    }
                    node_ident_stack.clear();
                }
            }

            // Increment the event stamp. Because new `UpdateTag`s have a default event stampo of 0,
            // make sure our event stamp is never 0.
            *event_stamp = event_stamp.wrapping_add(1);
            if *event_stamp == 0 {
                *event_stamp += 1;
            }

            if mark_active_nodes_redraw {
                node_stack.drain_to_root(|node, _, _| {
                    let update_tag = node.update_tag();
                    update_tag.mark_update_child_immutable();
                });
            }

            let mut return_flow = LoopFlow::Continue;
            if 0 < actions.len() {
                let root = node_stack.move_to_root().node;
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


            // Draw the node tree.
            if mark_active_nodes_redraw || *force_full_redraw {
                let root = node_stack.move_to_root().node;

                let force_full_redraw = *force_full_redraw || renderer.force_full_redraw();

                let mut root_update = root.update_tag().needs_update(root_id);
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

            return_flow
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
                    update_tag: _
                } = *summary;

                let mut root_update = child_node.update_tag().needs_update(self.root_id);
                root_update.render_self |= self.force_full_redraw;
                root_update.update_child |= self.force_full_redraw;
                let Update {
                    render_self,
                    update_child,
                    update_layout
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
