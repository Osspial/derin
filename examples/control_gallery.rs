extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate glutin;

use derin::dct::hints::Margins;
use derin::{Button, EditBox, Group, Label, LayoutHorizontal, LayoutVertical};
use derin::core::LoopFlow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GalleryEvent {
    NewButton
}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<Option<GalleryEvent>>,
    nested: Group<NestedContainer, LayoutVertical>
}

#[derive(NodeContainer)]
#[derin(action = "GalleryEvent")]
struct NestedContainer {
    label: Label,
    edit_box: EditBox,
    #[derin(collection = "Button<Option<GalleryEvent>>")]
    buttons: Vec<Button<Option<GalleryEvent>>>
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new("Add Button".to_string(), Some(GalleryEvent::NewButton)),
            nested: Group::new(
                NestedContainer {
                    label: Label::new("Nested Container".to_string()),
                    edit_box: EditBox::new("A Text Box".to_string()),
                    buttons: Vec::new(),
                },
                LayoutVertical::new(Margins::new(8, 8, 8, 8), Default::default())
            )
        },
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(512, 512)
        .with_title("Derin Control Gallery");

    let mut window = unsafe{ derin::glutin_window::GlutinWindow::new(window_builder, group, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |event, root, _| {
            root.container_mut().nested.container_mut().buttons.push(Button::new("An added button".to_string(), None));
            println!("{:?}", event);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
