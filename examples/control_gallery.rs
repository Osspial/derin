extern crate derin;
#[macro_use]
extern crate derin_macros;

use derin::{LoopFlow, Window, WindowAttributes};
use derin::layout::{Margins, LayoutHorizontal, LayoutVertical};
use derin::widgets::{Contents, Button, EditBox, Group, Label, Slider, SliderHandler, ScrollBox};

#[derive(Debug, Clone, Copy, PartialEq)]
enum GalleryEvent {
    NewButton,
    SliderMove(f32)
}

#[derive(WidgetContainer)]
#[derin(action = "GalleryEvent")]
struct BasicContainer {
    button: Button<Option<GalleryEvent>>,
    nested: ScrollBox<Group<NestedContainer, LayoutVertical>>
}

#[derive(WidgetContainer)]
#[derin(action = "GalleryEvent")]
struct NestedContainer {
    label: Label,
    edit_box: EditBox,
    slider: Slider<SliderH>,
    #[derin(collection = "Button<Option<GalleryEvent>>")]
    buttons: Vec<Button<Option<GalleryEvent>>>
}

struct SliderH;
impl SliderHandler for SliderH {
    type Action = GalleryEvent;
    fn on_move(&mut self, _: f32, new_value: f32) -> Option<GalleryEvent> {
        Some(GalleryEvent::SliderMove(new_value))
    }
}

fn main() {
    let group = Group::new(
        BasicContainer {
            button: Button::new(Contents::Text("New Button".to_string()), Some(GalleryEvent::NewButton)),
            nested: ScrollBox::new(Group::new(
                NestedContainer {
                    label: Label::new(Contents::Text("Nested Container".to_string())),
                    slider: Slider::new(1.0, 1.0, 0.0, 255.0, SliderH),
                    edit_box: EditBox::new("Edit Me!".to_string()),
                    buttons: Vec::new(),
                },
                LayoutVertical::new(Margins::new(8, 8, 8, 8), Default::default())
            ))
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
            if GalleryEvent::NewButton == event {
                root.container_mut().nested.widget_mut().container_mut().buttons.push(Button::new(Contents::Text("An added button".to_string()), None));
            }
            println!("{:?}", event);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
