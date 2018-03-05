extern crate derin;
#[macro_use]
extern crate derin_macros;

use derin::{LoopFlow, Window, WindowAttributes};
use derin::layout::{Margins, LayoutHorizontal, LayoutVertical};
use derin::widgets::{Button, EditBox, Group, Label};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GalleryEvent {
    NewButton
}

#[derive(WidgetContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<Option<GalleryEvent>>,
    nested: Group<NestedContainer, LayoutVertical>
}

#[derive(WidgetContainer)]
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

    let window_attributes = WindowAttributes {
        dimensions: Some((512, 512)),
        title: "Derin Control Gallery".to_string(),
        ..WindowAttributes::default()
    };

    let mut window = unsafe{ Window::new(window_attributes, group, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |event, root, _| {
            root.container_mut().nested.container_mut().buttons.push(Button::new("An added button".to_string(), None));
            println!("{:?}", event);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
