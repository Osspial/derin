extern crate derin;
#[macro_use]
extern crate derin_macros;

use derin::{Window, WindowAttributes, LoopFlow};
use derin::layout::{Margins, LayoutHorizontal};
use derin::widgets::{Button, Group, Label};

#[derive(WidgetContainer)]
#[derin(action = "i32")]
struct Counter {
    increment: Button<Option<i32>>,
    decrement: Button<Option<i32>>,
    label: Label
}

fn main() {
    let mut value = 0;
    let mut counter_ui = Group::new(
        Counter {
            increment: Button::new("Increment".to_string(), Some(1)),
            decrement: Button::new("Decrement".to_string(), Some(-1)),
            label: Label::new(value.to_string())
        },
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_attributes = WindowAttributes {
        dimensions: Some((400, 50)),
        title: "Counter Example".to_string(),
        ..WindowAttributes::default()
    };

    let mut window = unsafe{ Window::new(window_attributes, counter_ui, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |value_delta, counter_ui, _| {
            value += value_delta;
            *counter_ui.container_mut().label.string_mut() = value.to_string();
            LoopFlow::Continue
        },
        |_, _| None
    );
}
