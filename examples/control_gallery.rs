extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate glutin;
extern crate png;

extern crate gl_raii;

use derin::dct::buttons::MouseButton;
use derin::dct::hints::{WidgetPos, NodeSpan, GridSize, Margins};
use derin::{ButtonHandler, NodeLayout, Button, Group};
use derin::gl_render::GLRenderer;
use derin::core::{LoopFlow, Root, WindowEvent};
use derin::core::tree::NodeIdent;
use derin::theme::{ThemeText, ThemeFace, RescaleRules};
use derin::geometry::{Point2, DimsRect, Rectangle};

use gl_raii::colors::Rgba;
use gl_raii::glsl::Nu8;

use std::rc::Rc;

use glutin::{Event, ControlFlow, WindowEvent as GWindowEvent, MouseButton as GMouseButton, ElementState};


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
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetPos> {
        match node_ident {
            NodeIdent::Str("button") => Some(WidgetPos {
                node_span: NodeSpan::new(0, 0),
                margins: Margins::new(16, 100, 16, 16),
                ..WidgetPos::default()
            }),
            NodeIdent::Str("nested") => Some(WidgetPos {
                node_span: NodeSpan::new(1, 0),
                margins: Margins::new(16, 16, 16, 100),
                ..WidgetPos::default()
            }),
            _ => None
        }
    }
    fn grid_size(&self) -> GridSize {
        GridSize::new(2, 1)
    }
}

impl NodeLayout for BasicLayoutVertical {
    fn hints(&self, node_ident: NodeIdent) -> Option<WidgetPos> {
        match node_ident {
            NodeIdent::Str("button0") => Some(WidgetPos {
                node_span: NodeSpan::new(0, 0),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetPos::default()
            }),
            NodeIdent::Str("button1") => Some(WidgetPos {
                node_span: NodeSpan::new(0, 1),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetPos::default()
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
            button: Button::new("Hello\n\tWorld!".to_string(), BasicHandler),
            nested: Group::new(NestedContainer {
                button0: Button::new("tr".to_string(), BasicHandler),
                button1: Button::new("br".to_string(), BasicHandler)
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

    let mut theme = derin::theme::Theme::new();

    macro_rules! upload_image {
        ($name:expr, $path:expr, $dims:expr, $border:expr) => {{
            let image_png = png::Decoder::new(std::io::Cursor::new(&include_bytes!($path)[..]));
            let (info, mut reader) = image_png.read_info().unwrap();
            // Allocate the output buffer.
            let mut image = vec![0; info.buffer_size()];
            // Read the next frame. Currently this function should only called once.
            // The default options
            reader.next_frame(&mut image).unwrap();
            theme.insert_node(
                $name.to_string(),
                derin::theme::ThemeNode {
                    text: Some(ThemeText {
                        face: ThemeFace::new("./tests/DejaVuSans.ttf", 0).unwrap(),
                        color: Rgba::new(Nu8(255), Nu8(100), Nu8(100), Nu8(255)),
                        face_size: 16 * 64
                    }),
                    icon: Some(Rc::new(derin::theme::Image {
                        pixels: unsafe {
                            Vec::from_raw_parts(
                                image.as_mut_ptr() as *mut _,
                                image.len() / 4,
                                image.capacity() / 4
                            )
                        },
                        dims: DimsRect::new($dims, $dims),
                        rescale: RescaleRules::Slice(Margins::new($border, $border, $border, $border))
                    }))
                }
            );

            ::std::mem::forget(image);
        }}
    }

    upload_image!("Group", "../group.png", 3, 1);
    upload_image!("Button::Normal", "../button.normal.png", 32, 4);
    upload_image!("Button::Hover", "../button.hover.png", 32, 4);
    upload_image!("Button::Clicked", "../button.clicked.png", 32, 4);


    let mut root = Root::new(group, theme, dims);
    root.run_forever(|for_each_event| {
        let mut ret: Option<()> = None;
        events_loop.run_forever(|glutin_event| {
            match glutin_event {
                Event::WindowEvent{event, ..} => {
                    let derin_event_opt: Option<WindowEvent> = match event {
                        GWindowEvent::CursorMoved{position, ..} => Some(WindowEvent::MouseMove(Point2::new(position.0 as i32, position.1 as i32))),
                        GWindowEvent::CursorEntered{..} => Some(WindowEvent::MouseEnter(Point2::new(0, 0))),
                        GWindowEvent::CursorLeft{..} => Some(WindowEvent::MouseExit(Point2::new(0, 0))),
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
                Event::Suspended(..) |
                Event::DeviceEvent{..} => ()
            }

            ControlFlow::Continue
        });

        ret
    }, |_, _, _| {LoopFlow::Continue}, &mut renderer);
}
