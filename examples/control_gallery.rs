#![feature(never_type)]

extern crate derin;
extern crate derin_core;
extern crate dct;
extern crate cgmath;
extern crate cgmath_geometry;
extern crate glutin;

use dct::buttons::MouseButton;
use dct::hints::{WidgetHints, NodeSpan, GridSize, Margins};
use derin::{ButtonHandler, NodeContainer, NodeLayout, Button, Group};
use derin::gl_render::{GLRenderer, GLFrame};
use derin_core::{LoopFlow, Root, WindowEvent};
use derin_core::tree::{Node, NodeSummary, NodeIdent};

use glutin::{Event, ControlFlow, WindowEvent as GWindowEvent, MouseButton as GMouseButton, ElementState};

use cgmath::Point2;
use cgmath_geometry::{DimsRect, Rectangle};

enum GalleryEvent {}

struct BasicContainer {
    button0: Button<BasicHandler>,
    button1: Button<BasicHandler>
}

struct BasicHandler;
struct BasicLayout;

impl ButtonHandler for BasicHandler {
    type Action = GalleryEvent;

    fn on_click(&mut self) -> Option<GalleryEvent> {
        None
    }
}

impl NodeLayout for BasicLayout {
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetHints> {
        match node_ident {
            NodeIdent::Str("button0") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 0),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetHints::default()
            }),
            NodeIdent::Str("button1") => Some(WidgetHints {
                node_span: NodeSpan::new(1, 0),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetHints::default()
            }),
            _ => None
        }
    }
    fn grid_size(&self) -> GridSize {
        GridSize::new(2, 1)
    }
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button0: Button::new(BasicHandler),
            button1: Button::new(BasicHandler)
        },
        BasicLayout
    );

    let dims = DimsRect::new(512, 512);
    let mut events_loop = glutin::EventsLoop::new();
    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(dims.width(), dims.height())
        .with_title("Derin Control Gallery");

    let mut renderer = unsafe{ GLRenderer::new(&events_loop, window_builder).unwrap() };

    let mut root = Root::new(group, dims);
    root.run_forever(|for_each_event| {
        let mut ret: Option<()> = None;
        events_loop.run_forever(|glutin_event| {
            match glutin_event {
                Event::WindowEvent{event, ..} => {
                    let derin_event_opt: Option<WindowEvent> = match event {
                        GWindowEvent::MouseMoved{position, ..} => Some(WindowEvent::MouseMove(Point2::new(position.0 as i32, position.1 as i32))),
                        GWindowEvent::MouseEntered{..} => Some(WindowEvent::MouseEnter(Point2::new(0, 0))),
                        GWindowEvent::MouseLeft{..} => Some(WindowEvent::MouseExit(Point2::new(0, 0))),
                        GWindowEvent::MouseInput{state, button: g_button, ..} => {
                            let button = match g_button {
                                GMouseButton::Left => Some(MouseButton::Left),
                                GMouseButton::Right => Some(MouseButton::Right),
                                GMouseButton::Middle => Some(MouseButton::Middle),
                                GMouseButton::Other(1) => Some(MouseButton::X1),
                                GMouseButton::Other(2) => Some(MouseButton::X2),
                                GMouseButton::Other(_) => None
                            };
                            button.map(|b| match state {
                                ElementState::Pressed => WindowEvent::MouseDown(b),
                                ElementState::Released => WindowEvent::MouseUp(b)
                            })
                        }
                        GWindowEvent::Resized(width, height) => Some(WindowEvent::WindowResize(DimsRect::new(width, height))),
                        GWindowEvent::Closed => return ControlFlow::Break,
                        _ => None
                    };

                    if let Some(derin_event) = derin_event_opt {
                        match for_each_event(derin_event) {
                            LoopFlow::Break(b) => {
                                ret = Some(b);
                                return ControlFlow::Break;
                            },
                            LoopFlow::Continue => ()
                        }
                    }
                },
                Event::Awakened |
                Event::DeviceEvent{..} => ()
            }

            ControlFlow::Continue
        });

        ret
    }, |_| {LoopFlow::Continue}, &mut renderer);
}

impl NodeContainer for BasicContainer {
    type Action = GalleryEvent;
    type Frame = GLFrame;

    fn children<'a, G, R>(&'a self, mut for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a Node<Self::Action, Self::Frame>>) -> LoopFlow<R>,
              Self::Action: 'a,
              Self::Frame: 'a
    {
        let mut flow;
        flow = for_each_child(NodeSummary {
            ident: NodeIdent::Str("button0"),
            rect: <Button<_> as Node<Self::Action, Self::Frame>>::bounds(&self.button0),
            update_tag: <Button<_> as Node<Self::Action, Self::Frame>>::update_tag(&self.button0).clone(),
            node: &self.button0
        });
        if let LoopFlow::Break(b) = flow {
            return Some(b);
        }

        flow = for_each_child(NodeSummary {
            ident: NodeIdent::Str("button1"),
            rect: <Button<_> as Node<Self::Action, Self::Frame>>::bounds(&self.button1),
            update_tag: <Button<_> as Node<Self::Action, Self::Frame>>::update_tag(&self.button1).clone(),
            node: &self.button1
        });
        if let LoopFlow::Break(b) = flow {
            return Some(b);
        }

        None
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each_child: G) -> Option<R>
        where G: FnMut(NodeSummary<&'a mut Node<Self::Action, Self::Frame>>) -> LoopFlow<R>,
              Self::Action: 'a,
              Self::Frame: 'a
    {
        let mut flow;
        flow = for_each_child(NodeSummary {
            ident: NodeIdent::Str("button0"),
            rect: <Button<_> as Node<Self::Action, Self::Frame>>::bounds(&self.button0),
            update_tag: <Button<_> as Node<Self::Action, Self::Frame>>::update_tag(&self.button0).clone(),
            node: &mut self.button0
        });
        if let LoopFlow::Break(b) = flow {
            return Some(b);
        }

        flow = for_each_child(NodeSummary {
            ident: NodeIdent::Str("button1"),
            rect: <Button<_> as Node<Self::Action, Self::Frame>>::bounds(&self.button1),
            update_tag: <Button<_> as Node<Self::Action, Self::Frame>>::update_tag(&self.button1).clone(),
            node: &mut self.button1
        });
        if let LoopFlow::Break(b) = flow {
            return Some(b);
        }

        None
    }
}
