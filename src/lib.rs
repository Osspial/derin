pub extern crate dct;
extern crate dle;
pub extern crate derin_core as core;
extern crate cgmath;
extern crate cgmath_geometry;
extern crate gl_raii;
#[macro_use]
extern crate gl_raii_macros;
extern crate glutin;
extern crate arrayvec;

pub mod gl_render;

use std::cell::RefCell;

use gl_render::Vertex;

use dct::hints::{WidgetHints, GridSize};
use dle::{GridEngine, UpdateHeapCache, SolveError};
use core::LoopFlow;
use core::tree::{NodeIdent, NodeSummary, UpdateTag, NodeEvent, NodeSubtrait, NodeSubtraitMut, RenderFrame, FrameRectStack, Node, Parent};

use gl_raii::glsl::{Nu8, Nu16};
use gl_raii::colors::Rgba;
use cgmath::{Zero, One, Point2, Vector2};
use cgmath_geometry::{BoundRect, DimsRect, Rectangle};

use arrayvec::ArrayVec;

pub mod geometry {
    pub use cgmath::*;
    pub use cgmath_geometry::*;
}


pub trait NodeContainer<F: RenderFrame> {
    type Action;

    fn children<'a, G, R>(&'a self, for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a;

    fn children_mut<'a, G, R>(&'a mut self, for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a mut Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a;

    fn child(&self, node_ident: NodeIdent) -> Option<NodeSummary<&Node<Self::Action, F>>> {
        self.children(|summary| {
            if summary.ident == node_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    fn child_mut(&mut self, node_ident: NodeIdent) -> Option<NodeSummary<&mut Node<Self::Action, F>>> {
        self.children_mut(|summary| {
            if summary.ident == node_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }
}

pub trait NodeLayout {
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetHints>;
    fn grid_size(&self) -> GridSize;
}

pub trait ButtonHandler {
    type Action;

    fn on_click(&mut self) -> Option<Self::Action>;
}

#[derive(Debug, Clone)]
pub struct Button<H: ButtonHandler> {
    update_tag: UpdateTag,
    bounds: BoundRect<u32>,
    state: ButtonState,
    handler: H
}

#[derive(Debug, Clone)]
pub struct Group<C, L>
    where L: NodeLayout
{
    update_tag: UpdateTag,
    bounds: BoundRect<u32>,
    layout_engine: GridEngine,
    container: C,
    layout: L
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Normal,
    Hover,
    Clicked,
    Disabled,
    Defaulted
}

impl<H: ButtonHandler> Button<H> {
    pub fn new(handler: H) -> Button<H> {
        Button {
            update_tag: UpdateTag::new(),
            bounds: BoundRect::new(0, 0, 0, 0),
            state: ButtonState::Normal,
            handler
        }
    }
}

impl<C, L> Group<C, L>
    where L: NodeLayout
{
    pub fn new(container: C, layout: L) -> Group<C, L> {
        Group {
            update_tag: UpdateTag::new(),
            bounds: BoundRect::new(0, 0, 0, 0),
            layout_engine: GridEngine::new(),
            container, layout
        }
    }
}

impl<F, H> Node<H::Action, F> for Button<H>
    where F: RenderFrame<Primitive=[Vertex; 3]>,
          H: ButtonHandler
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn bounds(&self) -> BoundRect<u32> {
        self.bounds
    }

    #[inline]
    fn bounds_mut(&mut self) -> &mut BoundRect<u32> {
        &mut self.bounds
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        let color = match self.state {
            ButtonState::Normal    => Rgba::new(Nu8(128), Nu8(0)  , Nu8(0)  , Nu8(255)),
            ButtonState::Hover     => Rgba::new(Nu8(255), Nu8(0)  , Nu8(0)  , Nu8(255)),
            ButtonState::Clicked   => Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
            ButtonState::Disabled  => Rgba::new(Nu8(0)  , Nu8(0)  , Nu8(0)  , Nu8(255)),
            ButtonState::Defaulted => Rgba::new(Nu8(230), Nu8(230), Nu8(0)  , Nu8(255))
        };

        let tl = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::zero(), Nu16::one()), color);
        let bl = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::zero(), Nu16::zero()), color);
        let tr = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::one(), Nu16::one()), color);
        let br = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::one(), Nu16::zero()), color);
        frame.upload_primitives([
            [tl, bl, br],
            [tl, tr, br]
        ].iter().cloned());
    }

    fn on_node_event(&mut self, event: NodeEvent) -> Option<H::Action> {
        use self::NodeEvent::*;

        let mut action = None;
        let new_state = match event {
            MouseEnter{buttons_down_in_node, ..} if buttons_down_in_node.is_empty() => ButtonState::Hover,
            MouseExit{buttons_down_in_node, ..} if buttons_down_in_node.is_empty() => ButtonState::Normal,
            MouseEnter{..} |
            MouseExit{..}  |
            MouseMove{..} => self.state,
            MouseDown{..} => ButtonState::Clicked,
            MouseUp{in_node: true, pressed_in_node, ..} => {
                match pressed_in_node {
                    true => {
                        action = self.handler.on_click();
                        ButtonState::Hover
                    },
                    false => self.state
                }
            },
            MouseUp{in_node: false, ..} => ButtonState::Normal,
            MouseEnterChild{..} |
            MouseExitChild{..} => unreachable!()
        };

        if new_state != self.state {
            self.update_tag.mark_render_self();
            self.state = new_state;
        }

        action
    }

    #[inline]
    fn subtrait(&self) -> NodeSubtrait<H::Action, F> {
        NodeSubtrait::Node(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<H::Action, F> {
        NodeSubtraitMut::Node(self)
    }
}

impl<A, F, C, L> Node<A, F> for Group<C, L>
    where F: RenderFrame<Primitive=[Vertex; 3]>,
          C: NodeContainer<F, Action=A>,
          L: NodeLayout
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn bounds(&self) -> BoundRect<u32> {
        self.bounds
    }

    #[inline]
    fn bounds_mut(&mut self) -> &mut BoundRect<u32> {
        self.update_tag.mark_update_layout();
        &mut self.bounds
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        let color = Rgba::new(Nu8(128), Nu8(128), Nu8(128), Nu8(255));

        let tl = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::zero(), Nu16::one() ), color);
        let bl = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::zero(), Nu16::zero()), color);
        let tr = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::one() , Nu16::one() ), color);
        let br = Vertex::new(Point2::new(0, 0), Vector2::new(Nu16::one() , Nu16::zero()), color);
        frame.upload_primitives([
            [tl, bl, br],
            [tl, tr, br]
        ].iter().cloned());
    }

    #[inline]
    fn on_node_event(&mut self, _: NodeEvent) -> Option<A> {
        None
    }

    #[inline]
    fn subtrait(&self) -> NodeSubtrait<A, F> {
        NodeSubtrait::Parent(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<A, F> {
        NodeSubtraitMut::Parent(self)
    }
}

const CHILD_BATCH_SIZE: usize = 24;

impl<A, F, C, L> Parent<A, F> for Group<C, L>
    where F: RenderFrame<Primitive=[Vertex; 3]>,
          C: NodeContainer<F, Action=A>,
          L: NodeLayout
{
    fn child(&self, node_ident: NodeIdent) -> Option<NodeSummary<&Node<A, F>>> {
        self.container.child(node_ident)
    }

    fn child_mut(&mut self, node_ident: NodeIdent) -> Option<NodeSummary<&mut Node<A, F>>> {
        self.container.child_mut(node_ident)
    }

    fn children<'a>(&'a self, for_each: &mut FnMut(&[NodeSummary<&'a Node<A, F>>]) -> LoopFlow<()>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        self.container.children::<_, ()>(|summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    match for_each(&child_avec) {
                        LoopFlow::Break(_) => return LoopFlow::Break(()),
                        LoopFlow::Continue => ()
                    }
                    child_avec.clear();
                    child_avec.push(caperr.element());
                }
            }

            LoopFlow::Continue
        });

        if child_avec.len() != 0 {
            let _ = for_each(&child_avec);
        }
    }

    fn children_mut<'a>(&'a mut self, for_each: &mut FnMut(&mut [NodeSummary<&'a mut Node<A, F>>]) -> LoopFlow<()>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        self.container.children_mut::<_, ()>(|summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    match for_each(&mut child_avec) {
                        LoopFlow::Break(_) => return LoopFlow::Break(()),
                        LoopFlow::Continue => ()
                    }
                    child_avec.clear();
                    child_avec.push(caperr.element());
                }
            }

            LoopFlow::Continue
        });

        if child_avec.len() != 0 {
            let _ = for_each(&mut child_avec);
        }
    }

    fn update_child_layout(&mut self) {
        #[derive(Default)]
        struct HeapCache {
            update_heap_cache: UpdateHeapCache,
            hints_vec: Vec<WidgetHints>,
            rects_vec: Vec<Result<BoundRect<u32>, SolveError>>
        }
        thread_local! {
            static HEAP_CACHE: RefCell<HeapCache> = RefCell::new(HeapCache::default());
        }

        HEAP_CACHE.with(|hc| {
            let mut hc = hc.borrow_mut();

            let HeapCache {
                ref mut update_heap_cache,
                ref mut hints_vec,
                ref mut rects_vec
            } = *hc;

            self.container.children::<_, ()>(|summary| {
                hints_vec.push(self.layout.hints(summary.ident).unwrap_or(WidgetHints::default()));
                rects_vec.push(Ok(BoundRect::new(0, 0, 0, 0)));
                LoopFlow::Continue
            });

            self.layout_engine.desired_size = DimsRect::new(self.bounds.width(), self.bounds.height());
            self.layout_engine.set_grid_size(self.layout.grid_size());
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            let mut rects_iter = rects_vec.drain(..);
            self.container.children_mut::<_, ()>(|summary| {
                match rects_iter.next() {
                    Some(rect) => *summary.node.bounds_mut() = rect.unwrap_or(BoundRect::new(0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF)),
                    None => return LoopFlow::Break(())
                }
                LoopFlow::Continue
            });

            hints_vec.clear();
        })
    }

    fn child_by_point(&self, point: Point2<u32>) -> Option<NodeSummary<&Node<A, F>>> {
        self.container.children(|summary| {
            if summary.rect.contains(point) {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    fn child_by_point_mut(&mut self, point: Point2<u32>) -> Option<NodeSummary<&mut Node<A, F>>> {
        self.container.children_mut(|summary| {
            if summary.rect.contains(point) {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }
}
