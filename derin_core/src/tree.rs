use std::cell::Cell;

use LoopFlow;
use cgmath::Point2;
use cgmath_geometry::BoundRect;

use mbseq::MouseButtonSequence;
use dct::buttons::MouseButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeIdent {
    Str(&'static str),
    Num(u32),
    StrCollection(&'static str, u32),
    NumCollection(u32, u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Update {
    pub render_self: bool,
    pub update_child: bool,
    pub update_layout: bool
}

#[derive(Debug, Clone)]
pub struct UpdateTag {
    last_root: Cell<u32>,
    pub(crate) mouse_hovering: Cell<bool>,
    pub(crate) mouse_buttons_down_in_node: Cell<MouseButtonSequence>,
    pub(crate) child_event_recv: Cell<ChildEventRecv>
}

bitflags! {
    #[doc(hidden)]
    pub struct ChildEventRecv: u8 {
        const MOUSE_L     = 1 << 0;
        const MOUSE_R     = 1 << 1;
        const MOUSE_M     = 1 << 2;
        const MOUSE_X1    = 1 << 3;
        const MOUSE_X2    = 1 << 4;
        const MOUSE_HOVER = 1 << 5;
        // const KEYS        = 1 << 6;
    }
}

impl ChildEventRecv {
    #[inline]
    pub(crate) fn mouse_button_mask(button: MouseButton) -> ChildEventRecv {
        ChildEventRecv::from_bits_truncate(1 << (u8::from(button) - 1))
    }
}

impl From<MouseButtonSequence> for ChildEventRecv {
    #[inline]
    fn from(mbseq: MouseButtonSequence) -> ChildEventRecv {
        mbseq.into_iter().fold(ChildEventRecv::empty(), |child_event_recv, mb| child_event_recv | ChildEventRecv::mouse_button_mask(mb))
    }
}

impl<'a> From<&'a UpdateTag> for ChildEventRecv {
    #[inline]
    fn from(update_tag: &'a UpdateTag) -> ChildEventRecv {
        let node_mb_flags = ChildEventRecv::from(update_tag.mouse_buttons_down_in_node.get());

        node_mb_flags |
        ChildEventRecv::from_bits_truncate(update_tag.mouse_hovering.get() as u8 * ChildEventRecv::MOUSE_HOVER.bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootID(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeEvent<'a> {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton]
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton]
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton],
        child: NodeIdent
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton],
        child: NodeIdent
    },
    MouseMove {
        old: Point2<i32>,
        new: Point2<i32>,
        in_node: bool,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton]
    },
    MouseDown {
        pos: Point2<i32>,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        in_node: bool,
        pressed_in_node: bool,
        button: MouseButton
    }
}

macro_rules! subtrait_enums {
    (subtraits {
        $( $Variant:ident($SubTrait:ty) ),+
    }) => {
        pub enum NodeSubtrait<'a, A: 'a, F: 'a + RenderFrame> {
            $( $Variant(&'a $SubTrait) ),+
        }

        pub enum NodeSubtraitMut<'a, A: 'a, F: 'a + RenderFrame> {
            $( $Variant(&'a mut $SubTrait) ),+
        }
    }
}

subtrait_enums! {subtraits {
    Parent(Parent<A, F>),
    Node(Node<A, F>)
}}

pub trait Renderer {
    type Frame: RenderFrame;
    fn force_full_redraw(&self) -> bool {false}
    fn make_frame(&mut self) -> FrameRectStack<Self::Frame>;
    fn finish_frame(&mut self);
}

pub trait RenderFrame {
    type Transform;
    type Primitive: Copy;

    fn upload_primitives<I>(&mut self, transform: &Self::Transform, prim_iter: I)
        where I: Iterator<Item=Self::Primitive>;
    fn child_rect_transform(self_transform: &Self::Transform, child_rect: BoundRect<u32>) -> Self::Transform;
}

pub struct FrameRectStack<'a, F: 'a + RenderFrame> {
    frame: &'a mut F,
    transform: F::Transform
}

pub trait Node<A, F: RenderFrame> {
    fn update_tag(&self) -> &UpdateTag;
    fn bounds(&self) -> BoundRect<u32>;
    fn bounds_mut(&mut self) -> &mut BoundRect<u32>;
    fn render(&self, frame: &mut FrameRectStack<F>);
    fn on_node_event(&mut self, event: NodeEvent) -> Option<A>;
    fn subtrait(&self) -> NodeSubtrait<A, F>;
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<A, F>;
}

#[derive(Debug, Clone)]
pub struct NodeSummary<N> {
    pub node: N,
    pub ident: NodeIdent,
    pub rect: BoundRect<u32>,
    pub update_tag: UpdateTag
}

pub trait Parent<A, F: RenderFrame>: Node<A, F> {
    fn child(&self, node_ident: NodeIdent) -> Option<NodeSummary<&Node<A, F>>>;
    fn child_mut(&mut self, node_ident: NodeIdent) -> Option<NodeSummary<&mut Node<A, F>>>;

    fn children<'a>(&'a self, for_each: &mut FnMut(&[NodeSummary<&'a Node<A, F>>]) -> LoopFlow<()>);
    fn children_mut<'a>(&'a mut self, for_each: &mut FnMut(&mut [NodeSummary<&'a mut Node<A, F>>]) -> LoopFlow<()>);

    fn update_child_layout(&mut self);

    fn child_by_point(&self, point: Point2<u32>) -> Option<NodeSummary<&Node<A, F>>>;
    fn child_by_point_mut(&mut self, point: Point2<u32>) -> Option<NodeSummary<&mut Node<A, F>>>;
}

const RENDER_SELF: u32 = 1 << 31;
const UPDATE_CHILD: u32 = 1 << 30;
const RENDER_ALL: u32 = RENDER_SELF | UPDATE_CHILD;
const UPDATE_LAYOUT: u32 = (1 << 29);

const UPDATE_MASK: u32 = RENDER_SELF | UPDATE_CHILD | RENDER_ALL | UPDATE_LAYOUT;

impl UpdateTag {
    #[inline]
    pub fn new() -> UpdateTag {
        UpdateTag {
            last_root: Cell::new(UPDATE_MASK),
            mouse_hovering: Cell::new(false),
            mouse_buttons_down_in_node: Cell::new(MouseButtonSequence::new()),
            child_event_recv: Cell::new(ChildEventRecv::empty())
        }
    }

    #[inline]
    pub fn mark_render_self(&mut self) -> &mut UpdateTag {
        self.last_root.set(self.last_root.get() | RENDER_SELF);
        self
    }

    #[inline]
    pub fn mark_update_child(&mut self) -> &mut UpdateTag {
        self.last_root.set(self.last_root.get() | UPDATE_CHILD);
        self
    }

    #[inline]
    pub fn mark_update_layout(&mut self) -> &mut UpdateTag {
        self.last_root.set(self.last_root.get() | UPDATE_LAYOUT);
        self
    }

    #[inline]
    pub(crate) fn mark_updated(&self, root_id: RootID) {
        self.last_root.set(root_id.0);
    }

    pub(crate) fn mark_update_child_immutable(&self) {
        self.last_root.set(self.last_root.get() | UPDATE_CHILD);
    }

    #[inline]
    pub(crate) fn needs_update(&self, root_id: RootID) -> Update {
        match self.last_root.get() {
            r if r == root_id.0 => Update {
                render_self: false,
                update_child: false,
                update_layout: false
            },
            r => Update {
                render_self: r & UPDATE_MASK & RENDER_SELF != 0,
                update_child: r & UPDATE_MASK & UPDATE_CHILD != 0,
                update_layout: r & UPDATE_MASK & UPDATE_LAYOUT != 0
            },
        }
    }
}

impl RootID {
    #[inline]
    pub fn new() -> RootID {
        use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

        static ID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
        let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;
        assert!(id < UPDATE_MASK);

        RootID(id as u32)
    }
}

impl<'a, F: RenderFrame> FrameRectStack<'a, F> {
    #[inline]
    pub fn new(frame: &'a mut F, base_transform: F::Transform) -> FrameRectStack<'a, F> {
        FrameRectStack {
            frame,
            transform: base_transform
        }
    }

    #[inline]
    pub fn upload_primitives<I>(&mut self, prim_iter: I)
        where I: Iterator<Item=F::Primitive>
    {
        self.frame.upload_primitives(&self.transform, prim_iter)
    }

    #[inline]
    pub fn enter_child_rect<'b>(&'b mut self, child_rect: BoundRect<u32>) -> FrameRectStack<'b, F> {
        FrameRectStack {
            frame: self.frame,
            transform: F::child_rect_transform(&self.transform, child_rect)
        }
    }
}
