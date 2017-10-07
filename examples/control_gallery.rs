extern crate derin;
extern crate derin_core;
#[macro_use]
extern crate derin_macros;
extern crate dct;
extern crate cgmath;
extern crate cgmath_geometry;
extern crate glutin;

use dct::buttons::MouseButton;
use dct::hints::{WidgetHints, NodeSpan, GridSize, Margins};
use derin::{ButtonHandler, NodeLayout, Button, Group};
use derin::gl_render::GLRenderer;
use derin_core::{LoopFlow, Root, WindowEvent};
use derin_core::tree::NodeIdent;

use glutin::{Event, ControlFlow, WindowEvent as GWindowEvent, MouseButton as GMouseButton, ElementState};

use cgmath::Point2;
use cgmath_geometry::{DimsRect, Rectangle};

enum GalleryEvent {}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<BasicHandler>,
    nested: Group<NestedContainer, BasicLayoutVertical>
}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct NestedContainer {
    button0: Button<BasicHandler>,
    button1: Button<BasicHandler>
}

struct BasicHandler;
struct BasicLayout;
struct BasicLayoutVertical;

impl ButtonHandler for BasicHandler {
    type Action = GalleryEvent;

    fn on_click(&mut self) -> Option<GalleryEvent> {
        println!("clicked!");
        None
    }
}

impl NodeLayout for BasicLayout {
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetHints> {
        match node_ident {
            NodeIdent::Str("button") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 0),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetHints::default()
            }),
            NodeIdent::Str("nested") => Some(WidgetHints {
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

impl NodeLayout for BasicLayoutVertical {
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetHints> {
        match node_ident {
            NodeIdent::Str("button0") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 0),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetHints::default()
            }),
            NodeIdent::Str("button1") => Some(WidgetHints {
                node_span: NodeSpan::new(0, 1),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetHints::default()
            }),
            _ => None
        }
    }
    fn grid_size(&self) -> GridSize {
        GridSize::new(1, 2)
    }
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new(BasicHandler),
            nested: Group::new(NestedContainer {
                button0: Button::new(BasicHandler),
                button1: Button::new(BasicHandler)
            }, BasicLayoutVertical)
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
