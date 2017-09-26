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

use tree::{Node, Parent, Renderer, NodeSummary, RenderFrame, FrameRectStack, RootID, NodeEvent, Update, NodeSubtraitMut};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveNode {
    Click(ButtonsDown),
    Keyboard
}

bitflags!{
    struct ButtonsDown: u8 {
        const BUTTON_L  = 0b00001;
        const BUTTON_R  = 0b00010;
        const BUTTON_M  = 0b00100;
        const BUTTON_X1 = 0b01000;
        const BUTTON_X2 = 0b10000;
    }
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
            root_node,
            _marker: PhantomData
        }
    }

    fn draw<R: Renderer<Frame=F>>(&mut self, renderer: &mut R) {
        let root_update = match self.force_full_redraw {
            false => self.root_node.update_tag().needs_update(self.id),
            true => Update{ render_self: true, update_child: true, update_layout: true }
        };

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
                            force_full_redraw: self.force_full_redraw
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

    pub fn run_forever<E, AF, R, G>(&mut self, mut gen_events: E, mut on_action: AF, renderer: &mut R) -> Option<G>
        where E: FnMut(&mut FnMut(WindowEvent) -> LoopFlow<G>) -> Option<G>,
              AF: FnMut(A) -> LoopFlow<G>,
              R: Renderer<Frame=F>
    {

        self.active_node_stack.clear();
        self.draw(renderer);

        gen_events(&mut |event| {
            let mut mark_active_nodes_redraw = false;

            macro_rules! try_push_action {
                ($action_opt:expr) => {{
                    if let Some(action) = $action_opt {
                        self.actions.push_back(action);
                    }
                }};

                ($mbd_array:ident; $action_opt:expr) => {{
                    let $mbd_array = self.mouse_buttons_down.into_iter().collect::<ArrayVec<[_; 5]>>();
                    if let Some(action) = $action_opt {
                        self.actions.push_back(action);
                    }
                }}
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
                }}
            }

            match event {
                WindowEvent::WindowResize(new_size) => {
                    self.active_node_stack.clear();
                    self.force_full_redraw = true;
                    *self.root_node.bounds_mut() = new_size.into();
                }
                WindowEvent::MouseEnter(enter_pos) => {
                    try_push_action!{
                        mbd_array; self.root_node.on_node_event(NodeEvent::MouseEnter {
                            enter_pos,
                            buttons_down: &mbd_array
                        })
                    }
                    assert_eq!(self.active_node_stack.len(), 0);
                    let root_node_ptr = &mut self.root_node as *mut Node<A, F>;
                    self.active_node_stack.push(root_node_ptr);
                },
                WindowEvent::MouseExit(exit_pos) => {
                    assert_ne!(self.active_node_stack.len(), 0);

                    for node in self.active_node_stack.drain(..).rev().map(|node_ptr| unsafe{ &mut *node_ptr }) {
                        try_push_action!{
                            mbd_array; node.on_node_event(NodeEvent::MouseExit {
                                exit_pos,
                                buttons_down: &mbd_array
                            })
                        }
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
                                    try_push_action!{
                                        mbd_array; active_node.on_node_event(NodeEvent::MouseMove {
                                            old: old_pos,
                                            new: exit_pos.unwrap_or(move_to),
                                            in_node: true,
                                            buttons_down: &mbd_array
                                        })
                                    }
                                    try_push_action!{
                                        mbd_array; active_node.on_node_event(NodeEvent::MouseExit {
                                            exit_pos: exit,
                                            buttons_down: &mbd_array
                                        })
                                    }

                                    mark_if_needs_update!(active_node);

                                    self.active_node_stack.pop();
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
                                                    try_push_action!{
                                                        mbd_array; active_node_as_parent.on_node_event(NodeEvent::MouseMove {
                                                            old: old_pos,
                                                            new: exit_pos.unwrap_or(move_to),
                                                            in_node: true,
                                                            buttons_down: &mbd_array
                                                        })
                                                    }

                                                    mark_if_needs_update!(active_node_as_parent);
                                                },
                                                Some((child_ident, child_rect)) => {
                                                    let (child_enter_pos, _) = child_rect.cast()
                                                        .map(|rect| rect.intersects_int(move_line))
                                                        .unwrap_or((None, None));


                                                    if let Some(child_enter) = child_enter_pos {
                                                        try_push_action!{
                                                            mbd_array; active_node_as_parent.on_node_event(NodeEvent::MouseMove {
                                                                old: old_pos,
                                                                new: child_enter,
                                                                in_node: true,
                                                                buttons_down: &mbd_array
                                                            })
                                                        }
                                                        try_push_action!{
                                                            mbd_array; active_node_as_parent.on_node_event(NodeEvent::MouseEnterChild {
                                                                enter_pos: child_enter,
                                                                buttons_down: &mbd_array,
                                                                child: child_ident
                                                            })
                                                        }
                                                    }

                                                    mark_if_needs_update!(active_node_as_parent);


                                                    let child_node = active_node_as_parent.child_mut(child_ident).unwrap().node;
                                                    if let Some(child_enter) = child_enter_pos {
                                                        try_push_action!{
                                                            mbd_array; child_node.on_node_event(NodeEvent::MouseEnter {
                                                                enter_pos: child_enter,
                                                                buttons_down: &mbd_array
                                                            })
                                                        }
                                                    }
                                                    mark_if_needs_update!(child_node);

                                                    self.active_node_stack.push(child_node);

                                                    continue;
                                                }
                                            }
                                        },
                                        NodeSubtraitMut::Node(active_node) => {
                                            try_push_action!{
                                                mbd_array; active_node.on_node_event(NodeEvent::MouseMove {
                                                    old: old_pos,
                                                    new: exit_pos.unwrap_or(move_to),
                                                    in_node: true,
                                                    buttons_down: &mbd_array
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
                },
                WindowEvent::MouseUp(button) => {
                    self.build_active_stack();

                    let active_node = unsafe{ &mut **self.active_node_stack.last_mut().unwrap() };
                    let active_node_offset = active_node.bounds().min().cast().unwrap_or(Point2::new(0, 0)).to_vec();
                    try_push_action!{
                        active_node.on_node_event(NodeEvent::MouseUp {
                            pos: self.mouse_pos + active_node_offset,
                            in_node: true,
                            button
                        })
                    }
                    mark_if_needs_update!(active_node);
                    self.mouse_buttons_down.release_button(button);
                }
            }

            if mark_active_nodes_redraw {
                for node_ptr in &self.active_node_stack {
                    let node = unsafe{ &**node_ptr };
                    node.update_tag().mark_update_child_immutable();
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
    force_full_redraw: bool
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
                    ref update_tag
                } = *summary;

                let Update {
                    render_self,
                    update_child,
                    update_layout
                } = match self.force_full_redraw {
                    false => update_tag.needs_update(self.root_id),
                    true => Update{ render_self: true, update_child: true, update_layout: true }
                };

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
                                force_full_redraw: self.force_full_redraw
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
