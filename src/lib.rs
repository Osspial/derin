#![feature(slice_rotate)]

pub extern crate dct;
extern crate dat;
extern crate dle;
pub extern crate derin_core as core;
extern crate cgmath;
extern crate cgmath_geometry;
extern crate gl_raii;
#[macro_use]
extern crate gl_raii_macros;
extern crate glutin;
extern crate arrayvec;
extern crate glyphydog;
extern crate itertools;

pub mod gl_render;
pub mod theme;

use self::gl_render::{ThemedPrim, Prim, RelPoint};

use std::cell::RefCell;

use dct::hints::{WidgetPos, GridSize};
use dle::{GridEngine, UpdateHeapCache, SolveError};
use core::LoopFlow;
use core::event::{NodeEvent, EventOps};
use core::render::{RenderFrame, FrameRectStack};
use core::tree::{NodeIdent, NodeSummary, UpdateTag, NodeSubtrait, NodeSubtraitMut, Node, Parent};

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};

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
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetPos>;
    fn grid_size(&self) -> GridSize;
}

pub trait ButtonHandler {
    type Action;

    fn on_click(&mut self) -> Option<Self::Action>;
}

#[derive(Debug, Clone)]
pub struct Button<H: ButtonHandler> {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<u32>>,
    state: ButtonState,
    handler: H,
    string: String
}

#[derive(Debug, Clone)]
pub struct Group<C, L>
    where L: NodeLayout
{
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<u32>>,
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
    pub fn new(string: String, handler: H) -> Button<H> {
        Button {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            state: ButtonState::Normal,
            handler, string
        }
    }
}

impl<C, L> Group<C, L>
    where L: NodeLayout
{
    pub fn new(container: C, layout: L) -> Group<C, L> {
        Group {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            layout_engine: GridEngine::new(),
            container, layout
        }
    }
}

impl<F, H> Node<H::Action, F> for Button<H>
    where F: RenderFrame<Primitive=ThemedPrim>,
          H: ButtonHandler
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn bounds(&self) -> BoundBox<Point2<u32>> {
        self.bounds
    }

    #[inline]
    fn bounds_mut(&mut self) -> &mut BoundBox<Point2<u32>> {
        &mut self.bounds
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        let image_str = match self.state {
            ButtonState::Normal    => "Button::Normal",
            ButtonState::Hover     => "Button::Hover",
            ButtonState::Clicked   => "Button::Clicked",
            ButtonState::Disabled  => "Button::Disabled",
            ButtonState::Defaulted => "Button::Defaulted"
        };

        frame.upload_primitives([
            ThemedPrim {
                theme_path: image_str,
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image
            },
            ThemedPrim {
                theme_path: image_str,
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Text(&self.string[..])
            }
        ].iter().cloned());
    }

    fn on_node_event(&mut self, event: NodeEvent, _: &[NodeIdent]) -> EventOps<H::Action> {
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

        EventOps {
            action,
            focus: None
        }
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
    where F: RenderFrame<Primitive=ThemedPrim>,
          C: NodeContainer<F, Action=A>,
          L: NodeLayout
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn bounds(&self) -> BoundBox<Point2<u32>> {
        self.bounds
    }

    #[inline]
    fn bounds_mut(&mut self) -> &mut BoundBox<Point2<u32>> {
        self.update_tag.mark_update_layout();
        &mut self.bounds
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives([
            ThemedPrim {
                theme_path: "Group",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image
            }
        ].iter().cloned());
    }

    #[inline]
    fn on_node_event(&mut self, _: NodeEvent, _: &[NodeIdent]) -> EventOps<A> {
        EventOps {
            action: None,
            focus: None
        }
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
    where F: RenderFrame<Primitive=ThemedPrim>,
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
            hints_vec: Vec<WidgetPos>,
            rects_vec: Vec<Result<BoundBox<Point2<u32>>, SolveError>>
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
                hints_vec.push(self.layout.hints(summary.ident).unwrap_or(WidgetPos::default()));
                rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
                LoopFlow::Continue
            });

            self.layout_engine.desired_size = DimsBox::new2(self.bounds.width(), self.bounds.height());
            self.layout_engine.set_grid_size(self.layout.grid_size());
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            let mut rects_iter = rects_vec.drain(..);
            self.container.children_mut::<_, ()>(|summary| {
                match rects_iter.next() {
                    Some(rect) => *summary.node.bounds_mut() = rect.unwrap_or(BoundBox::new2(0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF, 0xDEADBEEF)),
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
