#![feature(conservative_impl_trait, universal_impl_trait, range_contains, nll)]

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
mod event_loop_ops;

use cgmath::{Point2, Bounded};
use cgmath_geometry::DimsBox;

use std::time::Duration;
use std::marker::PhantomData;
use std::collections::{HashMap, VecDeque};

use tree::*;
pub use event_loop_ops::EventLoopOps;
use timer::TimerList;
use event::NodeEvent;
use render::{Renderer, RenderFrame};
use mbseq::MouseButtonSequenceTrackPos;
use node_stack::NodeStackBase;
use meta_tracker::MetaEventTracker;
use dct::buttons::{MouseButton, Key, ModifierKeys};
use dct::cursor::CursorIcon;

pub struct Root<A, N, F>
    where N: Node<A, F> + 'static,
          A: 'static,
          F: RenderFrame + 'static
{
    id: RootID,
    mouse_pos: Point2<i32>,
    modifiers: ModifierKeys,
    cursor_icon: CursorIcon,

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
    popup_nodes: HashMap<PopupID, PopupNode<A, F>>,
    _marker: PhantomData<*const F>
}

struct PopupNode<A, F: RenderFrame> {
    node: Box<Node<A, F>>,
    mouse_pos: Point2<i32>
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLoopResult<R> {
    pub flow: LoopFlow<R>,
    pub wait_until_call_timer: Option<Duration>,
    pub popups: Vec<PopupAttributes>
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
            modifiers: ModifierKeys::empty(),
            cursor_icon: CursorIcon::default(),
            actions: VecDeque::new(),
            node_stack_base: NodeStackBase::new(),
            force_full_redraw: false,
            event_stamp: 1,
            node_ident_stack: Vec::new(),
            meta_tracker: MetaEventTracker::default(),
            timer_list: TimerList::new(None),
            root_node, theme,
            popup_nodes: HashMap::new(),
            _marker: PhantomData
        }
    }

    pub fn run_forever<R, G>(
        &mut self,
        mut gen_events: impl FnMut(&mut EventLoopOps<A, N, F, R, G>) -> Option<G>,
        mut on_action: impl FnMut(A, &mut N, &mut F::Theme) -> LoopFlow<G>,
        mut bubble_fallthrough: impl FnMut(NodeEvent, &[NodeIdent]) -> Option<A>,
        mut with_renderer: impl FnMut(Option<PopupID>, &mut FnMut(&mut R))
    ) -> Option<G>
        where R: Renderer<Frame=F>
    {
        gen_events(&mut EventLoopOps {
            root: self,
            on_action: &mut on_action,
            bubble_fallthrough: &mut bubble_fallthrough,
            with_renderer: &mut with_renderer
        })
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
