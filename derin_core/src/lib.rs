#![feature(specialization)]

extern crate cgmath;
extern crate cgmath_geometry;
extern crate dct;

pub mod tree;

use cgmath::{Point2, Vector2, EuclideanSpace};
use cgmath_geometry::{Rectangle, BoundRect, DimsRect, Segment};

use std::mem;
use std::collections::{VecDeque, HashMap};

use tree::{Node, Parent, Renderer, RootID, NodeIdent, NodeEvent, NodeUpdater};
use dct::buttons::MouseButton;

pub struct Root<A, N, R>
    where N: Node<A, R>,
          R: Renderer
{
    id: RootID,
    state: RootState<A>,
    pub root_node: N,
    renderer: R
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootEvent {
    MouseMove(Point2<i32>),
    MouseDown {
        pos: Point2<i32>,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        button: MouseButton
    }
}

struct RootState<A> {
    dims: DimsRect<u32>,
    mouse_pos: Point2<i32>,
    mouse_buttons_down: Vec<MouseButton>,
    mouse_hover_node: Vec<NodeIdent>,
    mouse_click_nodes: HashMap<MouseButton, NodeIdent>,
    keyboard_node: Vec<NodeIdent>,
    actions: VecDeque<A>
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopFlow<R> {
    Continue,
    Break(R)
}

impl<A, N, R> Root<A, N, R>
    where N: Node<A, R>,
          R: Renderer
{
    #[inline]
    pub fn new(root_node: N, renderer: R, dims: DimsRect<u32>) -> Root<A, N, R> {
        Root {
            id: RootID::new(),
            state: RootState {
                dims,
                mouse_pos: Point2::new(-1, -1),
                mouse_buttons_down: Vec::new(),
                mouse_hover_node: Vec::new(),
                mouse_click_nodes: HashMap::new(),
                keyboard_node: Vec::new(),
                actions: VecDeque::new(),
            },
            root_node,
            renderer
        }
    }

    pub fn run_forever<I, F, G>(&mut self, mut event_iter: I, mut on_action: F) -> G
        where I: Iterator<Item=RootEvent>,
              F: FnMut(A) -> LoopFlow<G>
    {
        'main_loop: loop {
            let _ = NodeEventPump {
                root_id: self.id,
                node_rect: self.state.dims.into(),
                root_state: &mut self.state,
                event_iter: &mut event_iter,
            }.update_node(&mut self.root_node);

            while let Some(action) = self.state.actions.pop_front() {
                match on_action(action) {
                    LoopFlow::Continue => (),
                    LoopFlow::Break(ret) => break 'main_loop ret
                }
            }
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraverserRet {
    /// The traverser returned because the mouse exited the node. Contains the point, in root
    /// space, at which the exit occured.
    Exit(Point2<i32>),
    Unwind(Unwind)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unwind {
    Continue,
    Break
}

struct NodeEventPump<'a, A, I>
    where A: 'a,
          I: 'a + Iterator<Item=RootEvent>
{
    root_id: RootID,
    root_state: &'a mut RootState<A>,
    event_iter: &'a mut I,
    node_rect: BoundRect<u32>
}

impl<'a, A, I, R> NodeUpdater<A, R> for NodeEventPump<'a, A, I>
    where I: Iterator<Item=RootEvent>,
          R: Renderer
{
    type Ret = TraverserRet;
    fn update_node<C>(mut self, node: &mut C) -> TraverserRet
        where C: Node<A, R>
    {
        trait ParentSwitch<A, C, R>
            where C: Node<A, R>,
                  R: Renderer
        {
            fn move_event(&mut self, node: &mut C, in_node: bool, move_line: Segment<i32>, node_coord_offset: Vector2<i32>) -> Option<Unwind>;
        }
        impl<'a, A, I, C, R> ParentSwitch<A, C, R> for NodeEventPump<'a, A, I>
            where I: Iterator<Item=RootEvent>,
                  C: Node<A, R>,
                  R: Renderer
        {
            default fn move_event(&mut self, node: &mut C, in_node: bool, move_line: Segment<i32>, node_coord_offset: Vector2<i32>) -> Option<Unwind> {
                node.on_node_event(NodeEvent::MouseMove {
                    old: move_line.start + node_coord_offset,
                    new: move_line.end + node_coord_offset,
                    in_node,
                    buttons_down: &self.root_state.mouse_buttons_down
                }).map(|action| self.root_state.actions.push_back(action));
                None
            }
        }
        impl<'a, A, I, P, R> ParentSwitch<A, P, R> for NodeEventPump<'a, A, I>
            where I: Iterator<Item=RootEvent>,
                  P: Parent<A, R>,
                  R: Renderer
        {
            fn move_event(&mut self, node: &mut P, in_node: bool, move_line: Segment<i32>, node_coord_offset: Vector2<i32>) -> Option<Unwind> {
                let child_search_result = (move_line.end - node_coord_offset).cast().and_then(|p| node.find_child(p));

                match child_search_result {
                    // If a child wasn't found, send a standard move event and return
                    None => {
                        node.on_node_event(NodeEvent::MouseMove {
                            old: move_line.start + node_coord_offset,
                            new: move_line.end + node_coord_offset,
                            in_node,
                            buttons_down: &self.root_state.mouse_buttons_down
                        }).map(|action| self.root_state.actions.push_back(action));
                        None
                    },

                    // If a child *was* found, send the relevant events to this node and hand control
                    // flow over to the child.
                    Some((child_id, child_rect)) => {
                        let enter_pos = child_rect.cast().unwrap().intersects_int(move_line).0.unwrap();
                        // If an action is pushed to the action buffer here, the child handles unwinding
                        // the stack.
                        node.on_node_event(NodeEvent::MouseEnterChild {
                            enter_pos,
                            buttons_down: &self.root_state.mouse_buttons_down,
                            child: child_id
                        }).map(|action| self.root_state.actions.push_back(action));

                        self.root_state.mouse_hover_node.push(child_id);
                        let child_ret = node.child_mut(child_id, NodeEventPump {
                            root_id: self.root_id,
                            root_state: self.root_state,
                            event_iter: self.event_iter,
                            node_rect: child_rect
                        });

                        match child_ret {
                            // The child node has been exited, so send an exit node to this node (the
                            // child's parent) and continue executing the event loop as the parent.
                            TraverserRet::Exit(exit_pos) => {
                                node.on_node_event(NodeEvent::MouseExitChild {
                                    exit_pos,
                                    buttons_down: &self.root_state.mouse_buttons_down,
                                    child: child_id
                                }).map(|action| self.root_state.actions.push_back(action));
                                None
                            },
                            // The child triggered an unwind, so propagate the unwind
                            TraverserRet::Unwind(unwind) => Some(unwind)
                        }
                    }
                }
            }
        }


        loop {
            // The action check is before the main body of the loop so we can unwinding from actions
            // created by the parent node.
            if 0 < self.root_state.actions.len() {
                // There are actions to be processed, so unwind the traverser stack, run the action
                // function, and then rebuild the stack.
                return TraverserRet::Unwind(Unwind::Continue);
            }

            let mut exit_pos_opt = None;
            match self.event_iter.next() {
                None => return TraverserRet::Unwind(Unwind::Break),
                Some(RootEvent::MouseMove(new)) => {
                    let old = mem::replace(&mut self.root_state.mouse_pos, new);
                    let node_coord_offset = self.node_rect.min.to_vec().cast::<i32>().unwrap();

                    let move_line = Segment {
                        start: old,
                        end: new
                    };

                    let in_node: bool;
                    let (mut enter_pos, mut exit_pos) = (None, None);
                    let node_rect_i = self.node_rect.cast::<i32>().unwrap();
                    match node_rect_i.intersects_int(move_line) {
                        (None, None) => {
                            in_node = node_rect_i.contains(new);
                        },
                        (None, Some(exit)) => {
                            exit_pos = Some(exit);
                            in_node = false;
                        },
                        (Some(enter), None) => {
                            enter_pos = Some(enter);
                            in_node = true;
                        },
                        (Some(enter), Some(exit)) => {
                            enter_pos = Some(enter);
                            exit_pos = Some(exit);
                            in_node = false;
                        }
                    }

                    if let Some(enter) = enter_pos {
                        node.on_node_event(NodeEvent::MouseEnter {
                            enter_pos: enter + node_coord_offset,
                            buttons_down: &self.root_state.mouse_buttons_down
                        }).map(|action| self.root_state.actions.push_back(action));
                    }
                    if let Some(exit) = exit_pos {
                        node.on_node_event(NodeEvent::MouseExit {
                            exit_pos: exit + node_coord_offset,
                            buttons_down: &self.root_state.mouse_buttons_down
                        }).map(|action| self.root_state.actions.push_back(action));
                        exit_pos_opt = Some(exit);
                    }

                    self.move_event(node, in_node, move_line, node_coord_offset)
                        .map(|unwind| return TraverserRet::Unwind(unwind));

                    // TODO: DRAW IF DRAWING IS NEEDED
                },
                _ => unimplemented!()
            }

            if let Some(exit_pos) = exit_pos_opt {
                self.root_state.mouse_hover_node.pop();
                return TraverserRet::Exit(exit_pos);
            }
        }
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
