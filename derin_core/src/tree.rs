use std::cmp::PartialEq;
use std::cell::Cell;

use cgmath::Point2;

use render::DVertex;
use dct::buttons::MouseButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildID {
    Str(&'static str),
    Num(u32),
    StrCollection(&'static str, u32),
    NumCollection(u32, u32)
}

pub(crate) enum Draw {
    Self_,
    Child,
    All,
    None
}

#[derive(Debug, Clone)]
pub struct DrawTag {
    last_root: Cell<u32>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootID(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawEvent {
    MouseEnter {
        enter_pos: Point2<i32>,
        mb_down: Option<MouseButton>
    },
    MouseExit {
        exit_pos: Point2<i32>,
        mb_down: Option<MouseButton>
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        mb_down: Option<MouseButton>,
        child: ChildID
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        mb_down: Option<MouseButton>,
        child: ChildID
    },
    MouseMove {
        old: Point2<i32>,
        new: Point2<i32>,
        in_node: bool,
        mb_down: Option<MouseButton>
    },
    MouseClick {
        pos: Point2<i32>,
        mb_down: MouseButton
    },
    MouseRelease {
        pos: Point2<i32>,
        in_node: bool,
        mb_up: MouseButton
    }
}

pub trait Node<A> {
    fn draw_tag(&self) -> &DrawTag;
    fn render<F>(&self, for_each_vertex: F)
        where F: FnMut(DVertex);
    fn on_raw_event(&mut self, event: RawEvent) -> Option<A>;
}

pub trait Parent<A>: Node<A> {
    fn children<C>(&self, _: C)
        where C: ChildTraverser;
    fn children_mut<C>(&mut self, _: C)
        where C: ChildTraverserMut;
}

pub trait ChildTraverser {
    type Action;
    fn view_child<N>(&mut self, child: &N)
        where N: Node<Self::Action>;
}

pub trait ChildTraverserMut {
    type Action;
    fn view_child<N>(&mut self, child: &mut N)
        where N: Node<Self::Action>;
}

const DRAW_SELF: u32 = 1 << 31;
const DRAW_CHILD: u32 = 1 << 30;
const DRAW_ALL: u32 = DRAW_SELF | DRAW_CHILD;

impl DrawTag {
    #[inline]
    pub fn new() -> DrawTag {
        DrawTag {
            last_root: Cell::new(DRAW_ALL)
        }
    }

    #[inline]
    pub fn mark_draw_self(&mut self) -> &mut DrawTag {
        self.last_root.set((self.last_root.get() & DRAW_ALL) | DRAW_SELF);
        self
    }

    #[inline]
    pub fn mark_draw_child(&mut self) -> &mut DrawTag {
        self.last_root.set((self.last_root.get() & DRAW_ALL) | DRAW_CHILD);
        self
    }

    #[inline]
    pub(crate) fn mark_drawn(&self, root_id: RootID) {
        self.last_root.set(root_id.0);
    }

    #[inline]
    pub(crate) fn needs_draw(&self, root_id: RootID) -> Draw {
        match self.last_root.get() {
            r if r == root_id.0  => Draw::None,
            r if r == DRAW_SELF  => Draw::Self_,
            r if r == DRAW_CHILD => Draw::Child,
            _                    => Draw::All
        }
    }
}

impl RootID {
    #[inline]
    pub fn new() -> RootID {
        use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

        static ID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;
        assert!(id < DRAW_ALL);

        RootID(id as u32)
    }
}
