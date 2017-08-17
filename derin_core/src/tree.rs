use std::cell::Cell;

use cgmath::Point2;
use cgmath_geometry::BoundRect;

use LoopFlow;
use dct::buttons::MouseButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeIdent {
    Str(&'static str),
    Num(u32),
    StrCollection(&'static str, u32),
    NumCollection(u32, u32)
}

pub(crate) enum Update {
    This,
    Child,
    All,
    None
}

#[derive(Debug, Clone)]
pub struct UpdateTag {
    last_root: Cell<u32>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootID(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeEvent<'a> {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseButton]
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseButton]
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        child: NodeIdent
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        child: NodeIdent
    },
    MouseMove {
        old: Point2<i32>,
        new: Point2<i32>,
        in_node: bool,
        buttons_down: &'a [MouseButton]
    },
    MouseDown {
        pos: Point2<i32>,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        in_node: bool,
        button: MouseButton
    }
}

pub trait Renderer {
    type Primitive: Copy;
    fn upload_primitives<I>(&mut self, prim_iter: I)
        where I: Iterator<Item=Self::Primitive>;
}

pub trait Node<A, R: Renderer> {
    fn update_tag(&self) -> &UpdateTag;
    fn bounds(&self) -> BoundRect<u32>;
    fn render(&self, renderer: &mut R);
    fn on_node_event(&mut self, event: NodeEvent) -> Option<A>;
}

pub trait Parent<A, R: Renderer>: Node<A, R> {
    fn child<C>(&self, node_ident: NodeIdent, _: C) -> C::Ret
        where C: NodeViewer<A, R>;
    fn child_mut<C>(&mut self, node_ident: NodeIdent, _: C) -> C::Ret
        where C: NodeUpdater<A, R>;

    fn children<C>(&self, _: C) -> Option<R>
        where C: NodeSeqViewer<A, R, Ret=LoopFlow<R>>;
    fn children_mut<C>(&mut self, _: C) -> Option<R>
        where C: NodeSeqUpdater<A, R, Ret=LoopFlow<R>>;

    fn find_child(&self, point: Point2<u32>) -> Option<(NodeIdent, BoundRect<u32>)>;
}

pub trait NodeViewer<A, R: Renderer> {
    type Ret;
    fn view_node<N>(self, node: &N) -> Self::Ret
        where N: Node<A, R>;
}

pub trait NodeUpdater<A, R: Renderer> {
    type Ret;
    fn update_node<N>(self, node: &mut N) -> Self::Ret
        where N: Node<A, R>;
}

/// A type which can view a sequence of immutable nodes
pub trait NodeSeqViewer<A, R: Renderer> {
    type Ret;
    fn view_node<N>(&mut self, node: &N, node_ident: NodeIdent) -> LoopFlow<Self::Ret>
        where N: Node<A, R>;
}

/// A type which can view a sequence of mutable nodes
pub trait NodeSeqUpdater<A, R: Renderer> {
    type Ret;
    fn update_node<N>(&mut self, node: &mut N, node_ident: NodeIdent) -> LoopFlow<Self::Ret>
        where N: Node<A, R>;
}

const UPDATE_THIS: u32 = 1 << 31;
const UPDATE_CHILD: u32 = 1 << 30;
const UPDATE_ALL: u32 = UPDATE_THIS | UPDATE_CHILD;

impl UpdateTag {
    #[inline]
    pub fn new() -> UpdateTag {
        UpdateTag {
            last_root: Cell::new(UPDATE_ALL)
        }
    }

    #[inline]
    pub fn mark_update_this(&mut self) -> &mut UpdateTag {
        self.last_root.set((self.last_root.get() & UPDATE_ALL) | UPDATE_THIS);
        self
    }

    #[inline]
    pub fn mark_update_child(&mut self) -> &mut UpdateTag {
        self.last_root.set((self.last_root.get() & UPDATE_ALL) | UPDATE_CHILD);
        self
    }

    #[inline]
    pub(crate) fn mark_updated(&self, root_id: RootID) {
        self.last_root.set(root_id.0);
    }

    #[inline]
    pub(crate) fn needs_update(&self, root_id: RootID) -> Update {
        match self.last_root.get() {
            r if r == root_id.0  => Update::None,
            r if r == UPDATE_THIS  => Update::This,
            r if r == UPDATE_CHILD => Update::Child,
            _                    => Update::All
        }
    }
}

impl RootID {
    #[inline]
    pub fn new() -> RootID {
        use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

        static ID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;
        assert!(id < UPDATE_ALL);

        RootID(id as u32)
    }
}
