extern crate cgmath;
extern crate cgmath_geometry;
#[macro_use]
extern crate bitflags;
extern crate dct;
extern crate arrayvec;

pub mod tree;
mod mbseq;

use arrayvec::ArrayVec;

use cgmath::{EuclideanSpace, Point2, Vector2, Bounded, Array};
use cgmath_geometry::{Rectangle, DimsRect, Segment};

use std::marker::PhantomData;
use std::collections::VecDeque;

use tree::{Node, Parent, Renderer, NodeSummary, RenderFrame, ChildEventRecv, FrameRectStack, RootID, NodeEvent, Update, UpdateTag, NodeSubtraitMut};
use mbseq::MouseButtonSequence;
use dct::buttons::MouseButton;

pub struct Root<A, N, F>
    where N: Node<A, F> + 'static,
          F: RenderFrame,
          A: 'static,
          F: 'static
{
    id: RootID,
    mouse_pos: Point2<i32>,
    mouse_buttons_down: MouseButtonSequence,
    actions: VecDeque<A>,
    active_node_stack: Vec<*mut Node<A, F>>,
    active_node_offset: Vector2<i32>,
    force_full_redraw: bool,
    force_full_relayout: bool,
    pub root_node: N,
    _marker: PhantomData<*const F>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    MouseMove(Point2<i32>),
    MouseEnter(Point2<i32>),
    MouseExit(Point2<i32>),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    WindowResize(DimsRect<u32>)
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
    pub fn new(mut root_node: N, dims: DimsRect<u32>) -> Root<A, N, F> {
        *root_node.bounds_mut() = dims.into();
        Root {
            id: RootID::new(),
            mouse_pos: Point2::new(-1, -1),
            mouse_buttons_down: MouseButtonSequence::new(),
            actions: VecDeque::new(),
            active_node_stack: Vec::new(),
            active_node_offset: Vector2::new(0, 0),
            force_full_redraw: true,
            force_full_relayout: true,
            root_node,
            _marker: PhantomData
        }
    }

    fn draw<R: Renderer<Frame=F>>(&mut self, renderer: &mut R) {
        let force_full_redraw = self.force_full_redraw || renderer.force_full_redraw();

        let mut root_update = self.root_node.update_tag().needs_update(self.id);
        root_update.render_self |= force_full_redraw;
        root_update.update_child |= force_full_redraw;
        root_update.update_layout |= self.force_full_relayout;

        if root_update.render_self || root_update.update_child {
            {
                let mut frame = renderer.make_frame();
                if let NodeSubtraitMut::Parent(root_as_parent) = self.root_node.subtrait_mut() {
                    if root_update.update_layout {
                        root_as_parent.update_child_layout();
                    }
                }
                if root_update.render_self {
                    self.root_node.render(&mut frame);
                }
                if root_update.update_child {
                    if let NodeSubtraitMut::Parent(root_as_parent) = self.root_node.subtrait_mut() {
                        NodeRenderer {
                            root_id: self.id,
                            frame,
                            force_full_redraw: force_full_redraw,
                            force_full_relayout: self.force_full_relayout
                        }.render_node_children(root_as_parent)
                    }
                }
            }

            renderer.finish_frame();
            self.root_node.update_tag().mark_updated(self.id);
        }

        self.force_full_redraw = false;
    }

    fn build_active_stack(&mut self) {
        if self.active_node_stack.len() == 0 {
            self.active_node_stack.push(&mut self.root_node);
        }

        loop {
            let active_node = unsafe{ &mut **self.active_node_stack.last_mut().unwrap() };
            match active_node.subtrait_mut() {
                NodeSubtraitMut::Node(_) => break,
                NodeSubtraitMut::Parent(parent) => {
                    match parent.child_by_point_mut(self.mouse_pos.cast().unwrap_or(Point2::from_value(!0))) {
                        Some(summary) => self.active_node_stack.push(summary.node),
                        None          => break
                    }
                }
            }
        }
    }

    fn move_active_over_flags<G>(&mut self, flags: ChildEventRecv, for_each_flag: G)
        where G: FnMut(&mut Node<A, F>) -> &UpdateTag
    {
        self.move_active_over_flags_inner(flags, for_each_flag);
        self.move_active_over_flags_inner(ChildEventRecv::MOUSE_HOVER, |node| {
            // TODO: HANDLE CHANGE IN NODE LOCATION
            node.update_tag()
        });
    }

    fn move_active_over_flags_inner<G>(&mut self, mut flags: ChildEventRecv, mut for_each_flag: G)
        where G: FnMut(&mut Node<A, F>) -> &UpdateTag
    {
        let get_update_flags = |update: &UpdateTag| update.child_event_recv.get() | ChildEventRecv::from(update);
        // Remove flags from the search set that aren't found at the root of the tree.
        flags &= {
            let root_update = self.root_node.update_tag();
            get_update_flags(root_update)
        };

        let mut on_flag_trail = None;

        while !flags.is_empty() {
            if on_flag_trail.is_none() {
                // The index and update tag of the closest flagged parent
                let (cfp_index, cfp_update_tag) =
                    self.active_node_stack.iter().enumerate().rev()
                        .map(|(i, n)| (i, unsafe{ &**n }.update_tag()))
                        .find(|&(_, u)| flags & (get_update_flags(u)) != ChildEventRecv::empty())
                        .unwrap();

                self.active_node_stack.truncate(cfp_index + 1);
                on_flag_trail = Some(get_update_flags(cfp_update_tag) & flags);
            }
            let flag_trail_flags = on_flag_trail.unwrap();

            let top_node = unsafe{ &mut **self.active_node_stack.last_mut().unwrap() };

            macro_rules! call_fn {
                ($node:expr) => {{
                    let update_tag = for_each_flag($node);
                    let flags_removed = flag_trail_flags - ChildEventRecv::from(update_tag);
                    if flags_removed != ChildEventRecv::empty() {
                        flags &= !flags_removed;

                        for update_tag in self.active_node_stack.iter().map(|n| unsafe{ &**n }.update_tag()) {
                            update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !flags_removed);
                        }
                    }
                }}
            }

            match top_node.subtrait_mut() {
                NodeSubtraitMut::Node(top_node) => {
                    let node_tags = ChildEventRecv::from(top_node.update_tag());
                    if node_tags & flag_trail_flags != ChildEventRecv::empty() {
                        call_fn!(top_node);
                    }

                    flags &= !flag_trail_flags;
                    on_flag_trail = None;
                },
                NodeSubtraitMut::Parent(top_node_as_parent) => {
                    let mut child_found = false;

                    top_node_as_parent.children_mut(&mut |children_summaries| {
                        let mut run_on_index = None;

                        for (index, child_summary) in children_summaries.iter_mut().enumerate() {
                            let child_node_ptr = child_summary.node as *mut Node<A, F>;

                            match child_summary.node.subtrait_mut() {
                                NodeSubtraitMut::Node(_) => {
                                    let node_tags = ChildEventRecv::from(&child_summary.update_tag);

                                    if node_tags & flag_trail_flags != ChildEventRecv::empty() {
                                        child_found = true;
                                        flags &= !flag_trail_flags;
                                        on_flag_trail = None;
                                        self.active_node_stack.push(child_node_ptr);

                                        run_on_index = Some(index);
                                        break;
                                    }
                                },
                                NodeSubtraitMut::Parent(_) => {
                                    let child_flags = flags & child_summary.update_tag.child_event_recv.get();
                                    if child_flags != ChildEventRecv::empty() {
                                        child_found = true;
                                        flags &= !flag_trail_flags;
                                        on_flag_trail = Some(child_flags);
                                        self.active_node_stack.push(child_node_ptr);

                                        run_on_index = Some(index);
                                        break;
                                    }
                                }
                            }
                        }

                        match run_on_index {
                            None => LoopFlow::Continue,
                            Some(index) => {
                                call_fn!(children_summaries[index].node);
                                LoopFlow::Break(())
                            }
                        }
                    });

                    if !child_found {
                        flags &= !flag_trail_flags;
                        on_flag_trail = None;
                    }
                }
            }
        }
    }

    pub fn run_forever<E, AF, R, G>(&mut self, mut gen_events: E, mut on_action: AF, renderer: &mut R) -> Option<G>
        where E: FnMut(&mut FnMut(WindowEvent) -> LoopFlow<G>) -> Option<G>,
              AF: FnMut(A) -> LoopFlow<G>,
              R: Renderer<Frame=F>
    {
        self.active_node_stack.clear();
        self.draw(renderer);

        gen_events(&mut |event| {
            let mut mark_active_nodes_redraw = false;

            macro_rules! mouse_button_arrays {
                ($update_tag:expr) => {{
                    let mbd_array = self.mouse_buttons_down.into_iter().collect::<ArrayVec<[_; 5]>>();
                    let mbdin_array = $update_tag.mouse_buttons_down_in_node.get().into_iter().collect::<ArrayVec<[_; 5]>>();
                    (mbd_array, mbdin_array)
                }}
            }

            macro_rules! try_push_action {
                ($action_opt:expr) => {{
                    if let Some(action) = $action_opt {
                        self.actions.push_back(action);
                    }
                }};
            }

            macro_rules! mark_if_needs_update {
                ($node:expr) => {{
                    let node_update_tag = $node.update_tag();
                    let node_update = node_update_tag.needs_update(self.id);
                    let no_update = Update{ render_self: false, update_child: false, update_layout: false };
                    if node_update != no_update {
                        mark_active_nodes_redraw = true;
                    }
                    if mark_active_nodes_redraw {
                        node_update_tag.mark_update_child_immutable();
                    }

                    node_update_tag
                }}
            }

            match event {
                WindowEvent::WindowResize(new_size) => {
                    self.active_node_stack.clear();
                    self.force_full_redraw = true;
                    self.force_full_relayout = true;
                    *self.root_node.bounds_mut() = new_size.into();
                }
                WindowEvent::MouseEnter(enter_pos) => {
                    let (mbd_array, mbdin_array) = mouse_button_arrays!(self.root_node.update_tag());
                    try_push_action!{
                        self.root_node.on_node_event(NodeEvent::MouseEnter {
                            enter_pos,
                            buttons_down: &mbd_array,
                            buttons_down_in_node: &mbdin_array
                        })
                    }
                    self.root_node.update_tag().mouse_hovering.set(true);
                    assert_eq!(self.active_node_stack.len(), 0);
                    let root_node_ptr = &mut self.root_node as *mut Node<A, F>;
                    self.active_node_stack.push(root_node_ptr);
                },
                WindowEvent::MouseExit(exit_pos) => {
                    assert_ne!(self.active_node_stack.len(), 0);

                    for node in self.active_node_stack.drain(..).rev().map(|node_ptr| unsafe{ &mut *node_ptr }) {
                        let (mbd_array, mbdin_array) = mouse_button_arrays!(node.update_tag());
                        try_push_action!{
                            node.on_node_event(NodeEvent::MouseExit {
                                exit_pos,
                                buttons_down: &mbd_array,
                                buttons_down_in_node: &mbdin_array
                            })
                        }

                        let update_tag = mark_if_needs_update!(node);
                        update_tag.mouse_hovering.set(false);
                        update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !ChildEventRecv::MOUSE_HOVER);
                    }
                },

                WindowEvent::MouseMove(mut move_to) => {
                    self.build_active_stack();

                    let mut old_pos = self.mouse_pos - self.active_node_offset;
                    self.mouse_pos = move_to;
                    move_to -= self.active_node_offset;

                    if self.root_node.bounds().cast().map(|r| r.contains(move_to)).unwrap_or(false) {
                        loop {
                            let active_node = unsafe{ &mut **self.active_node_stack.last_mut().unwrap() };

                            let node_bounds = active_node.bounds().cast::<i32>().unwrap();
                            let move_line = Segment {
                                start: old_pos,
                                end: move_to
                            };


                            let (_, exit_pos) = node_bounds.intersects_int(move_line);

                            match exit_pos {
                                Some(exit) => {
                                    let (mbd_array, mbdin_array) = {
                                        let update_tag = active_node.update_tag();
                                        update_tag.mouse_hovering.set(false);
                                        mouse_button_arrays!(update_tag)
                                    };

                                    try_push_action!{
                                        active_node.on_node_event(NodeEvent::MouseMove {
                                            old: old_pos,
                                            new: exit_pos.unwrap_or(move_to),
                                            in_node: true,
                                            buttons_down: &mbd_array,
                                            buttons_down_in_node: &mbdin_array
                                        })
                                    }
                                    try_push_action!{
                                        active_node.on_node_event(NodeEvent::MouseExit {
                                            exit_pos: exit,
                                            buttons_down: &mbd_array,
                                            buttons_down_in_node: &mbdin_array
                                        })
                                    }

                                    mark_if_needs_update!(active_node);

                                    self.active_node_stack.pop();
                                    if let Some(parent_node) = self.active_node_stack.last_mut().map(|n| unsafe{ &mut **n }) {
                                        let parent_update_tag = parent_node.update_tag();
                                        parent_update_tag.mouse_hovering.set(true);
                                        parent_update_tag.child_event_recv.set(parent_update_tag.child_event_recv.get() & !ChildEventRecv::MOUSE_HOVER);
                                    }

                                    old_pos = exit;
                                    continue;
                                },
                                None => {
                                    match active_node.subtrait_mut() {
                                        NodeSubtraitMut::Parent(active_node_as_parent) => {
                                            let child_ident_and_rect = active_node_as_parent
                                                .child_by_point_mut(move_to.cast().unwrap_or(Point2::max_value()))
                                                .map(|s| (s.ident, s.rect));

                                            match child_ident_and_rect {
                                                None => {
                                                    let (mbd_array, mbdin_array) = mouse_button_arrays!(active_node_as_parent.update_tag());
                                                    try_push_action!{
                                                        active_node_as_parent.on_node_event(NodeEvent::MouseMove {
                                                            old: old_pos,
                                                            new: exit_pos.unwrap_or(move_to),
                                                            in_node: true,
                                                            buttons_down: &mbd_array,
                                                            buttons_down_in_node: &mbdin_array
                                                        })
                                                    }

                                                    mark_if_needs_update!(active_node_as_parent);
                                                },
                                                Some((child_ident, child_rect)) => {
                                                    let (child_enter_pos, _) = child_rect.cast()
                                                        .map(|rect| rect.intersects_int(move_line))
                                                        .unwrap_or((None, None));


                                                    if let Some(child_enter) = child_enter_pos {
                                                        let (mbd_array, mbdin_array) = mouse_button_arrays!(active_node_as_parent.update_tag());
                                                        try_push_action!{
                                                            active_node_as_parent.on_node_event(NodeEvent::MouseMove {
                                                                old: old_pos,
                                                                new: child_enter,
                                                                in_node: true,
                                                                buttons_down: &mbd_array,
                                                                buttons_down_in_node: &mbdin_array
                                                            })
                                                        }
                                                        try_push_action!{
                                                            active_node_as_parent.on_node_event(NodeEvent::MouseEnterChild {
                                                                enter_pos: child_enter,
                                                                buttons_down: &mbd_array,
                                                                buttons_down_in_node: &mbdin_array,
                                                                child: child_ident
                                                            })
                                                        }
                                                    }

                                                    {
                                                        let update_tag = mark_if_needs_update!(active_node_as_parent);
                                                        update_tag.mouse_hovering.set(false);
                                                        update_tag.child_event_recv.set(update_tag.child_event_recv.get() | ChildEventRecv::MOUSE_HOVER);
                                                    }


                                                    let child_node = active_node_as_parent.child_mut(child_ident).unwrap().node;
                                                    if let Some(child_enter) = child_enter_pos {
                                                        let (mbd_array, mbdin_array) = mouse_button_arrays!(child_node.update_tag());
                                                        try_push_action!{
                                                            child_node.on_node_event(NodeEvent::MouseEnter {
                                                                enter_pos: child_enter,
                                                                buttons_down: &mbd_array,
                                                                buttons_down_in_node: &mbdin_array
                                                            })
                                                        }
                                                    }

                                                    {
                                                        let child_update_tag = mark_if_needs_update!(child_node);
                                                        child_update_tag.mouse_hovering.set(true);
                                                    }

                                                    self.active_node_stack.push(child_node);

                                                    continue;
                                                }
                                            }
                                        },
                                        NodeSubtraitMut::Node(active_node) => {
                                            let (mbd_array, mbdin_array) = mouse_button_arrays!(active_node.update_tag());
                                            try_push_action!{
                                                active_node.on_node_event(NodeEvent::MouseMove {
                                                    old: old_pos,
                                                    new: exit_pos.unwrap_or(move_to),
                                                    in_node: true,
                                                    buttons_down: &mbd_array,
                                                    buttons_down_in_node: &mbdin_array
                                                })
                                            }
                                        }
                                    }
                                }
                            }

                            break;
                        }
                    }
                },
                WindowEvent::MouseDown(button) => {
                    self.build_active_stack();

                    let active_node = unsafe{ &mut **self.active_node_stack.last_mut().unwrap() };
                    let active_node_offset = active_node.bounds().min().cast().unwrap_or(Point2::new(0, 0)).to_vec();
                    try_push_action!{
                        active_node.on_node_event(NodeEvent::MouseDown {
                            pos: self.mouse_pos + active_node_offset,
                            button
                        })
                    }
                    mark_if_needs_update!(active_node);
                    self.mouse_buttons_down.push_button(button);

                    let update_tag = active_node.update_tag();
                    update_tag.mouse_buttons_down_in_node.set(
                        *update_tag.mouse_buttons_down_in_node.get().clone().push_button(button)
                    );
                    let button_mask = ChildEventRecv::mouse_button_mask(button);
                    update_tag.child_event_recv.set(update_tag.child_event_recv.get() | button_mask);

                    for node in self.active_node_stack[..self.active_node_stack.len()].iter().map(|n| unsafe{ &**n }) {
                        let node_update_tag = node.update_tag();
                        node_update_tag.child_event_recv.set(node_update_tag.child_event_recv.get() | button_mask);
                    }
                },
                WindowEvent::MouseUp(button) => {
                    self.build_active_stack();

                    let move_to_mouse_node;
                    let button_mask = ChildEventRecv::mouse_button_mask(button);

                    {
                        let active_node = unsafe{ &mut **self.active_node_stack.last_mut().unwrap() };
                        let active_node_offset = active_node.bounds().min().cast().unwrap_or(Point2::new(0, 0)).to_vec();
                        let pressed_in_node = active_node.update_tag().mouse_buttons_down_in_node.get().contains(button);
                        try_push_action!{
                            active_node.on_node_event(NodeEvent::MouseUp {
                                pos: self.mouse_pos + active_node_offset,
                                in_node: true,
                                pressed_in_node,
                                button
                            })
                        }
                        mark_if_needs_update!(active_node);
                        self.mouse_buttons_down.release_button(button);

                        let update_tag = active_node.update_tag();
                        update_tag.mouse_buttons_down_in_node.set(
                            *update_tag.mouse_buttons_down_in_node.get().clone().release_button(button)
                        );

                        move_to_mouse_node = !update_tag.mouse_buttons_down_in_node.get().contains(button);
                    }

                    if move_to_mouse_node && self.root_node.update_tag().child_event_recv.get().contains(button_mask) {
                        let mouse_pos = self.mouse_pos;
                        let mut action_opt = None;
                        self.move_active_over_flags(button_mask, |node| {
                            action_opt =
                                node.on_node_event(NodeEvent::MouseUp {
                                    pos: mouse_pos,
                                    in_node: false,
                                    pressed_in_node: true,
                                    button
                                });

                            let update_tag = node.update_tag();
                            update_tag.mouse_buttons_down_in_node.set(
                                *update_tag.mouse_buttons_down_in_node.get().release_button(button)
                            );
                            update_tag
                        });
                        try_push_action!(action_opt);
                    } else {
                        for node in self.active_node_stack.iter().map(|n| unsafe{ &**n }) {
                            let update_tag = node.update_tag();
                            update_tag.child_event_recv.set(update_tag.child_event_recv.get() & !button_mask);
                        }
                    }
                }
            }

            if mark_active_nodes_redraw {
                for node_ptr in &self.active_node_stack {
                    let node = unsafe{ &**node_ptr };
                    let update_tag = node.update_tag();
                    update_tag.mark_update_child_immutable();
                }
            }
            if 0 < self.actions.len() {
                self.active_node_stack.clear();
            }

            let mut return_flow = LoopFlow::Continue;
            while let Some(action) = self.actions.pop_front() {
                match on_action(action) {
                    LoopFlow::Continue => (),
                    LoopFlow::Break(ret) => {
                        return_flow = LoopFlow::Break(ret);
                        break;
                    }
                }
            }

            self.draw(renderer);

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
    force_full_relayout: bool
}

impl<'a, F> NodeRenderer<'a, F>
    where F: 'a + RenderFrame
{
    fn render_node_children<A>(&mut self, parent: &mut Parent<A, F>) {
        parent.children_mut(&mut |children_summaries| {
            for summary in children_summaries {
                let NodeSummary {
                    node: ref mut child_node,
                    ident: _,
                    rect: child_rect,
                    update_tag: _
                } = *summary;

                let mut root_update = child_node.update_tag().needs_update(self.root_id);
                root_update.render_self |= self.force_full_redraw;
                root_update.update_child |= self.force_full_redraw;
                root_update.update_layout |= self.force_full_relayout;
                let Update {
                    render_self,
                    update_child,
                    update_layout
                } = root_update;

                match child_node.subtrait_mut() {
                    NodeSubtraitMut::Parent(child_node_as_parent) => {
                        let mut child_frame = self.frame.enter_child_rect(child_rect);

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
                                force_full_relayout: self.force_full_relayout
                            }.render_node_children(child_node_as_parent);
                        }
                    },
                    NodeSubtraitMut::Node(child_node) => {
                        if render_self {
                            child_node.render(&mut self.frame.enter_child_rect(child_rect));
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
