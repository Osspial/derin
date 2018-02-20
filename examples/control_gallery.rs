extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate glutin;

use derin::dct::hints::{WidgetPos, NodeSpan, GridSize, Margins};
use derin::{ButtonHandler, NodeLayout, Button, EditBox, Group, Label};
use derin::core::LoopFlow;
use derin::core::tree::NodeIdent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GalleryEvent {
    ButtonClicked
}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<BasicHandler>,
    nested: Group<NestedContainer, BasicLayoutVertical>
}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct NestedContainer {
    label: Label,
    edit_box: EditBox,
    #[derin(collection = "Button<BasicHandler>")]
    buttons: Vec<Button<BasicHandler>>
}

struct BasicHandler;
struct BasicLayout;
struct BasicLayoutVertical;

impl ButtonHandler for BasicHandler {
    type Action = GalleryEvent;

    fn on_click(&mut self) -> Option<GalleryEvent> {
        Some(GalleryEvent::ButtonClicked)
    }
}

impl NodeLayout for BasicLayout {
    fn hints(&self, node_ident: NodeIdent, _: usize, _: usize) -> Option<WidgetPos> {
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
    fn grid_size(&self, _: usize) -> GridSize {
        GridSize::new(2, 1)
    }
}

impl NodeLayout for BasicLayoutVertical {
    fn hints(&self, _: NodeIdent, node_index: usize, num_nodes: usize) -> Option<WidgetPos> {
        match node_index >= num_nodes {
            true => None,
            false => Some(WidgetPos {
                node_span: NodeSpan::new(0, node_index as u32),
                margins: Margins::new(16, 16, 16, 16),
                ..WidgetPos::default()
            })
        }
    }
    fn grid_size(&self, num_nodes: usize) -> GridSize {
        GridSize::new(1, num_nodes as u32)
    }
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new("Add Button".to_string(), BasicHandler),
            nested: Group::new(NestedContainer {
                label: Label::new("Nested Container".to_string()),
                edit_box: EditBox::new("A Text Box".to_string()),
                buttons: Vec::new(),
            }, BasicLayoutVertical)
        },
        BasicLayout
    );
    let theme = derin::theme::Theme::default();

    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(512, 512)
        .with_title("Derin Control Gallery");

    let mut window = unsafe{ derin::glutin_window::GlutinWindow::new(window_builder, group, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |event, root, _| {
            root.container_mut().nested.container_mut().buttons.push(Button::new("An added button".to_string(), BasicHandler));
            println!("{:?}", event);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
