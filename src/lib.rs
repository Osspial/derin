#![feature(slice_rotate, nll, range_contains, conservative_impl_trait, universal_impl_trait)]

pub extern crate dct;
extern crate dat;
extern crate dle;
pub extern crate derin_core as core;
extern crate cgmath;
extern crate cgmath_geometry;
extern crate gullery;
#[macro_use]
extern crate gullery_macros;
extern crate glutin;
extern crate arrayvec;
extern crate glyphydog;
extern crate itertools;
extern crate unicode_segmentation;
extern crate clipboard;
extern crate png;
extern crate parking_lot;

pub mod gl_render;
pub mod glutin_window;
pub mod theme;

use self::gl_render::{ThemedPrim, Prim, PrimFrame, RelPoint, EditString, RenderString};

use std::mem;
use std::cell::{Cell, RefCell};
use std::time::Duration;
use std::marker::PhantomData;

use dct::hints::{WidgetPos, GridSize, Margins, Align2, NodeSpan, SizeBounds};
use dct::cursor::CursorIcon;
use dct::buttons::{Key, ModifierKeys};
use dle::{GridEngine, UpdateHeapCache, SolveError};
use core::LoopFlow;
use core::event::{NodeEvent, EventOps, FocusChange, InputState};
use core::render::{RenderFrame, FrameRectStack, Theme as CoreTheme};
use core::timer::TimerRegister;
use core::tree::{NodeIdent, NodeSummary, UpdateTag, NodeSubtrait, NodeSubtraitMut, Node, Parent, OnFocus};
use core::popup::ChildPopupsMut;

use cgmath::{EuclideanSpace, Point2};
use cgmath_geometry::{BoundBox, Segment, DimsBox, GeoBox};

use arrayvec::ArrayVec;
use clipboard::{ClipboardContext, ClipboardProvider};


pub mod geometry {
    pub use cgmath::*;
    pub use cgmath_geometry::*;
}


pub trait NodeContainer<F: RenderFrame> {
    type Action;

    fn num_children(&self) -> usize;

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

    fn child_by_index(&self, mut index: usize) -> Option<NodeSummary<&Node<Self::Action, F>>> {
        self.children(|summary| {
            if index == 0 {
                LoopFlow::Break(summary)
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        })
    }
    fn child_by_index_mut(&mut self, mut index: usize) -> Option<NodeSummary<&mut Node<Self::Action, F>>> {
        self.children_mut(|summary| {
            if index == 0 {
                LoopFlow::Break(summary)
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SingleContainer<A, F: RenderFrame, N: Node<A, F>> {
    pub node: N,
    _marker: PhantomData<(A, F)>
}

impl<A, F: RenderFrame, N: Node<A, F>> SingleContainer<A, F, N> {
    #[inline(always)]
    pub fn new(node: N) -> SingleContainer<A, F, N> {
        SingleContainer{ node, _marker: PhantomData }
    }
}

impl<A, F: RenderFrame, N: Node<A, F>> NodeContainer<F> for SingleContainer<A, F, N> {
    type Action = A;

    #[inline(always)]
    fn num_children(&self) -> usize {1}

    fn children<'a, G, R>(&'a self, mut for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a
    {
        let self_summary = NodeSummary {
            node: &self.node as &Node<A, F>,
            ident: NodeIdent::Num(0),
            rect: self.node.rect(),
            size_bounds: self.node.size_bounds(),
            update_tag: self.node.update_tag().clone(),
            index: 0
        };
        match for_each_child(self_summary) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a mut Node<Self::Action, F>>) -> LoopFlow<R>,
              Self::Action: 'a,
              F: 'a
    {
        let self_summary = NodeSummary {
            rect: self.node.rect(),
            size_bounds: self.node.size_bounds(),
            update_tag: self.node.update_tag().clone(),
            node: &mut self.node as &mut Node<A, F>,
            ident: NodeIdent::Num(0),
            index: 0
        };
        match for_each_child(self_summary) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }
}

pub trait GridLayout {
    fn hints(&self, node_ident: NodeIdent, node_index: usize, num_nodes: usize) -> Option<WidgetPos>;
    fn grid_size(&self, num_nodes: usize) -> GridSize;
}

pub trait ButtonHandler {
    type Action: 'static;

    fn on_click(&mut self) -> Option<Self::Action>;
}

impl<A: 'static + Clone> ButtonHandler for Option<A> {
    type Action = A;

    fn on_click(&mut self) -> Option<Self::Action> {
        self.clone()
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutHorizontal {
    pub widget_margins: Margins<i32>,
    pub widget_place: Align2
}

impl LayoutHorizontal {
    #[inline(always)]
    pub fn new(widget_margins: Margins<i32>, widget_place: Align2) -> LayoutHorizontal {
        LayoutHorizontal{ widget_margins, widget_place }
    }
}

impl GridLayout for LayoutHorizontal {
    fn hints(&self, _: NodeIdent, node_index: usize, num_nodes: usize) -> Option<WidgetPos> {
        match node_index >= num_nodes {
            true => None,
            false => Some(WidgetPos {
                node_span: NodeSpan::new(node_index as u32, 0),
                margins: self.widget_margins,
                place_in_cell: self.widget_place,
                ..WidgetPos::default()
            })
        }
    }

    #[inline]
    fn grid_size(&self, num_nodes: usize) -> GridSize {
        GridSize::new(num_nodes as u32, 1)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutVertical {
    pub widget_margins: Margins<i32>,
    pub widget_place: Align2
}

impl LayoutVertical {
    #[inline(always)]
    pub fn new(widget_margins: Margins<i32>, widget_place: Align2) -> LayoutVertical {
        LayoutVertical{ widget_margins, widget_place }
    }
}

impl GridLayout for LayoutVertical {
    fn hints(&self, _: NodeIdent, node_index: usize, num_nodes: usize) -> Option<WidgetPos> {
        match node_index >= num_nodes {
            true => None,
            false => Some(WidgetPos {
                node_span: NodeSpan::new(0, node_index as u32),
                margins: self.widget_margins,
                place_in_cell: self.widget_place,
                ..WidgetPos::default()
            })
        }
    }

    #[inline]
    fn grid_size(&self, num_nodes: usize) -> GridSize {
        GridSize::new(1, num_nodes as u32)
    }
}

#[derive(Debug, Clone)]
pub struct Button<H: ButtonHandler> {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    state: ButtonState,
    handler: H,
    string: RenderString,
    waiting_for_mouseover: bool,
    size_bounds: Cell<SizeBounds>
}

#[derive(Debug, Clone)]
pub struct Label {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    string: RenderString
}

#[derive(Debug, Clone)]
pub struct EditBox {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    string: EditString,
    size_bounds: Cell<SizeBounds>
}

#[derive(Debug, Clone)]
pub struct Group<C, L>
    where L: GridLayout
{
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    layout_engine: GridEngine,
    container: C,
    layout: L
}

pub struct DirectRender<R>
    where R: DirectRenderState
{
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    render_state: R
}

pub trait DirectRenderState {
    type RenderType;

    fn render(&self, _: &mut Self::RenderType);
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
            handler,
            string: RenderString::new(string),
            waiting_for_mouseover: false,
            size_bounds: Cell::new(SizeBounds::default())
        }
    }

    pub fn string(&self) -> &str {
        self.string.string()
    }

    pub fn string_mut(&mut self) -> &mut String {
        self.update_tag.mark_render_self();
        self.string.string_mut()
    }
}

impl Label {
    pub fn new(string: String) -> Label {
        Label {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            string: RenderString::new(string)
        }
    }

    pub fn string(&self) -> &str {
        self.string.string()
    }

    pub fn string_mut(&mut self) -> &mut String {
        self.update_tag.mark_render_self();
        self.string.string_mut()
    }
}

impl EditBox {
    pub fn new(string: String) -> EditBox {
        EditBox {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            string: EditString::new(RenderString::new(string)),
            size_bounds: Cell::new(SizeBounds::default())
        }
    }

    pub fn string(&self) -> &str {
        self.string.render_string.string()
    }

    pub fn string_mut(&mut self) -> &mut String {
        self.update_tag.mark_render_self();
        self.string.render_string.string_mut()
    }
}

impl<C, L> Group<C, L>
    where L: GridLayout
{
    pub fn new(container: C, layout: L) -> Group<C, L> {
        Group {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            layout_engine: GridEngine::new(),
            container, layout
        }
    }

    pub fn container(&self) -> &C {
        &self.container
    }

    pub fn container_mut(&mut self) -> &mut C {
        self.update_tag.mark_update_child().mark_update_layout();
        &mut self.container
    }
}

impl<R: DirectRenderState> DirectRender<R> {
    pub fn new(render_state: R) -> DirectRender<R> {
        DirectRender {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            render_state
        }
    }

    pub fn render_state(&self) -> &R {
        &self.render_state
    }

    pub fn render_state_mut(&mut self) -> &mut R {
        self.update_tag.mark_render_self();
        &mut self.render_state
    }
}

impl<F, H> Node<H::Action, F> for Button<H>
    where F: PrimFrame,
          H: ButtonHandler
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds.get()
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        let image_str = match self.state {
            ButtonState::Normal    => "Button::Normal",
            ButtonState::Hover     => "Button::Hover",
            ButtonState::Clicked   => "Button::Clicked",
            ButtonState::Disabled  => "Button::Disabled",
            ButtonState::Defaulted => "Button::Defaulted"
        };
        let mut size_bounds = self.size_bounds.get();
        size_bounds.min = frame.theme().node_theme(image_str).icon.map(|i| i.min_size()).unwrap_or(DimsBox::new2(0, 0));
        self.size_bounds.set(size_bounds);

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
                prim: Prim::String(&self.string)
            }
        ].iter().cloned());
    }

    fn register_timers(&self, register: &mut TimerRegister) {
        if self.waiting_for_mouseover {
            register.add_timer("mouseover_text", Duration::new(1, 0)/2, true);
        }
    }

    fn on_node_event(&mut self, event: NodeEvent, input_state: InputState, popups_opt: Option<ChildPopupsMut<H::Action, F>>, bubble_source: &[NodeIdent]) -> EventOps<H::Action, F> {
        use self::NodeEvent::*;

        let (mut action, focus) = (None, None);
        let mut popup = None;

        if bubble_source.len() == 0 {
            if let Some(mut popups) = popups_opt {
                // Remove mouseover text, if it exists
                match event {
                    MouseEnter{..} |
                    MouseExit{..} |
                    MouseMove{..} |
                    MouseDown{..} => {
                        popups.remove(NodeIdent::Str("mouseover_text"));
                    },
                    _ => ()
                }
            }

            let new_state = match event {
                MouseEnter{buttons_down_in_node, ..} |
                MouseExit{buttons_down_in_node, ..} => {
                    self.waiting_for_mouseover = false;
                    self.update_tag.mark_update_timer();

                    match (buttons_down_in_node.is_empty(), event) {
                        (true, MouseEnter{..}) => ButtonState::Hover,
                        (true, MouseExit{..}) => ButtonState::Normal,
                        (false, _) => self.state,
                        _ => unreachable!()
                    }
                },
                MouseMove{..} => {
                    self.waiting_for_mouseover = true;
                    self.update_tag.mark_update_timer();
                    self.state
                },
                MouseDown{..} => {
                    self.update_tag.mark_update_timer();
                    ButtonState::Clicked
                },
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
                MouseExitChild{..} => unreachable!(),
                GainFocus => ButtonState::Hover,
                LoseFocus => ButtonState::Normal,
                Char(_)     |
                KeyDown(..) |
                KeyUp(..)  => self.state,
                Timer{name: "mouseover_text", times_triggered: 1, ..} => {
                    self.waiting_for_mouseover = false;
                    self.update_tag.mark_update_timer();
                    popup = Some((
                        Box::new(Group::new(SingleContainer::new(Label::new("Hello Popup!".to_string())), LayoutHorizontal::default())) as Box<Node<_, F>>,
                        ::core::popup::PopupAttributes {
                            rect: BoundBox::new2(1, 1, 129, 129) + input_state.mouse_pos.to_vec(),
                            title: "".to_string(),
                            decorations: false,
                            tool_window: true,
                            focusable: false,
                            ident: NodeIdent::Str("mouseover_text")
                        }
                    ));
                    self.state
                },
                Timer{..} => self.state
            };

            if new_state != self.state {
                self.update_tag.mark_render_self();
                self.state = new_state;
            }
        }


        EventOps {
            action, focus,
            bubble: true,
            cursor_pos: None,
            cursor_icon: None,
            popup
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

impl<A, F> Node<A, F> for Label
    where F: PrimFrame
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives([
            ThemedPrim {
                theme_path: "Label",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::String(&self.string)
            }
        ].iter().cloned());
    }

    #[inline]
    fn on_node_event(&mut self, _: NodeEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[NodeIdent]) -> EventOps<A, F> {
        EventOps {
            action: None,
            focus: None,
            bubble: true,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    #[inline]
    fn subtrait(&self) -> NodeSubtrait<A, F> {
        NodeSubtrait::Node(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<A, F> {
        NodeSubtraitMut::Node(self)
    }
}

impl<A, F> Node<A, F> for EditBox
    where F: PrimFrame
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds.get()
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        let mut size_bounds = self.size_bounds.get();
        size_bounds.min = frame.theme().node_theme("EditBox").icon.map(|i| i.min_size()).unwrap_or(DimsBox::new2(0, 0));
        self.size_bounds.set(size_bounds);

        frame.upload_primitives([
            ThemedPrim {
                theme_path: "EditBox",
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
                theme_path: "EditBox",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::EditString(&self.string)
            }
        ].iter().cloned());
    }

    fn on_node_event(&mut self, event: NodeEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[NodeIdent]) -> EventOps<A, F> {
        use self::NodeEvent::*;
        use dct::buttons::MouseButton;

        let allow_char = |c| match c {
            '\t' |
            '\r' |
            '\n' => true,
            _ => !c.is_control()
        };
        let mut focus = None;
        let mut cursor_icon = None;
        match event {
            KeyDown(key, modifiers) => loop {
                let jump_to_word_boundaries = modifiers.contains(ModifierKeys::CTRL);
                match (key, modifiers) {
                    (Key::LArrow, _) => self.string.move_cursor_horizontal(
                        -1,
                        jump_to_word_boundaries,
                        modifiers.contains(ModifierKeys::SHIFT)
                    ),
                    (Key::RArrow, _) => self.string.move_cursor_horizontal(
                        1,
                        jump_to_word_boundaries,
                        modifiers.contains(ModifierKeys::SHIFT)
                    ),
                    (Key::UArrow, _) => self.string.move_cursor_vertical(-1, modifiers.contains(ModifierKeys::SHIFT)),
                    (Key::DArrow, _) => self.string.move_cursor_vertical(1, modifiers.contains(ModifierKeys::SHIFT)),
                    (Key::A, ModifierKeys::CTRL) => self.string.select_all(),
                    (Key::C, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let select_range = self.string.highlight_range();
                            clipboard.set_contents(self.string.render_string.string()[select_range].to_string()).ok();
                        }
                    },
                    (Key::V, ModifierKeys::CTRL) => {
                        if let Ok(clipboard_conents) = ClipboardContext::new().and_then(|mut c| c.get_contents()) {
                            self.string.insert_str(&clipboard_conents);
                        }
                    },
                    (Key::X, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let highlight_range = self.string.highlight_range();
                            clipboard.set_contents(self.string.render_string.string()[highlight_range.clone()].to_string()).ok();
                            if highlight_range.len() > 0 {
                                self.string.delete_chars(1, false);
                            }
                        }
                    },
                    (Key::Back, _) => self.string.delete_chars(-1, jump_to_word_boundaries),
                    (Key::Delete, _) => self.string.delete_chars(1, jump_to_word_boundaries),
                    _ => break
                }
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
                break;
            },
            Char(c) if allow_char(c) => {
                self.string.insert_char(c);
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
            }
            MouseDown{in_node: true, button, pos} => {
                focus = Some(FocusChange::Take);
                if button == MouseButton::Left {
                    self.string.select_on_line(Segment::new(pos, pos));
                    self.update_tag
                        .mark_render_self()
                        .mark_update_timer();
                }
            },
            MouseUp{button: MouseButton::Left, ..} => {
                self.update_tag.mark_render_self();
            }
            MouseDown{in_node: false, ..} => {
                focus = Some(FocusChange::Remove);
                self.string.draw_cursor = false;
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
            },
            MouseMove{new_pos, buttons_down_in_node, ..} => {
                if let Some(down) = buttons_down_in_node.iter().find(|d| d.button == MouseButton::Left) {
                    self.string.select_on_line(Segment::new(down.down_pos, new_pos));
                    self.update_tag.mark_render_self();
                }
            },
            MouseEnter{..} => cursor_icon = Some(CursorIcon::Text),
            MouseExit{..} => cursor_icon = Some(CursorIcon::default()),
            GainFocus  |
            LoseFocus => {
                self.string.deselect_all();
                self.update_tag.mark_update_timer();
            },
            Timer{name: "cursor_flash", times_triggered, ..} => {
                self.string.draw_cursor = times_triggered % 2 == 0;
                self.update_tag.mark_render_self();
            },
            _ => ()
        };
        EventOps {
            action: None,
            focus,
            bubble: true,
            cursor_pos: None,
            cursor_icon,
            popup: None
        }
    }

    fn register_timers(&self, register: &mut TimerRegister) {
        if self.update_tag.has_keyboard_focus() {
            register.add_timer("cursor_flash", Duration::new(1, 0)/2, true);
        }
    }

    #[inline]
    fn subtrait(&self) -> NodeSubtrait<A, F> {
        NodeSubtrait::Node(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<A, F> {
        NodeSubtraitMut::Node(self)
    }
}

impl<A, F, C, L> Node<A, F> for Group<C, L>
    where F: PrimFrame,
          C: NodeContainer<F, Action=A>,
          L: GridLayout
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        self.update_tag.mark_update_layout();
        &mut self.bounds
    }
    fn size_bounds(&self) -> SizeBounds {
        self.layout_engine.actual_size_bounds()
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
    fn on_node_event(&mut self, _: NodeEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[NodeIdent]) -> EventOps<A, F> {
        EventOps {
            action: None,
            focus: None,
            bubble: true,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
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

    fn accepts_focus(&self) -> OnFocus {
        OnFocus::FocusChild
    }
}

const CHILD_BATCH_SIZE: usize = 24;

impl<A, F, C, L> Parent<A, F> for Group<C, L>
    where F: PrimFrame,
          C: NodeContainer<F, Action=A>,
          L: GridLayout
{
    fn num_children(&self) -> usize {
        self.container.num_children()
    }

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

    fn child_by_index(&self, index: usize) -> Option<NodeSummary<&Node<A, F>>> {
        self.container.child_by_index(index)
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<NodeSummary<&mut Node<A, F>>> {
        self.container.child_by_index_mut(index)
    }

    fn update_child_layout(&mut self) {
        #[derive(Default)]
        struct HeapCache {
            update_heap_cache: UpdateHeapCache,
            hints_vec: Vec<WidgetPos>,
            rects_vec: Vec<Result<BoundBox<Point2<i32>>, SolveError>>
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

            let num_children = self.num_children();
            self.container.children::<_, ()>(|summary| {
                let mut layout_hints = self.layout.hints(summary.ident, summary.index, num_children).unwrap_or(WidgetPos::default());
                layout_hints.size_bounds = SizeBounds {
                    min: layout_hints.size_bounds.bound_rect(summary.size_bounds.min),
                    max: layout_hints.size_bounds.bound_rect(summary.size_bounds.max),
                };
                hints_vec.push(layout_hints);
                rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
                LoopFlow::Continue
            });

            self.layout_engine.desired_size = DimsBox::new2(self.bounds.width(), self.bounds.height());
            self.layout_engine.set_grid_size(self.layout.grid_size(num_children));
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            let mut rects_iter = rects_vec.drain(..);
            self.container.children_mut::<_, ()>(|summary| {
                match rects_iter.next() {
                    Some(rect) => *summary.node.rect_mut() = rect.unwrap_or(BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF)),
                    None => return LoopFlow::Break(())
                }
                LoopFlow::Continue
            });

            hints_vec.clear();
        })
    }
}

impl<A, F, R> Node<A, F> for DirectRender<R>
    where R: DirectRenderState + 'static,
          F: PrimFrame<DirectRender = R::RenderType>
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives(Some(ThemedPrim {
            theme_path: "DirectRender",
            min: Point2::new(
                RelPoint::new(-1.0, 0),
                RelPoint::new(-1.0, 0),
            ),
            max: Point2::new(
                RelPoint::new( 1.0, 0),
                RelPoint::new( 1.0, 0)
            ),
            prim: unsafe{ Prim::DirectRender(mem::transmute((&|render_type: &mut R::RenderType| self.render_state.render(render_type)) as &Fn(&mut R::RenderType))) }
        }).into_iter());
    }

    #[inline]
    fn on_node_event(&mut self, _: NodeEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[NodeIdent]) -> EventOps<A, F> {
        EventOps {
            action: None,
            focus: None,
            bubble: true,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    #[inline]
    fn subtrait(&self) -> NodeSubtrait<A, F> {
        NodeSubtrait::Node(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> NodeSubtraitMut<A, F> {
        NodeSubtraitMut::Node(self)
    }
}
