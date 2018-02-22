extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate glutin;

use derin::dct::hints::Margins;
use derin::{Button, Group, Label, LayoutHorizontal};
use derin::core::LoopFlow;

#[derive(NodeContainer)]
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

    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(400, 50)
        .with_title("Counter Example");

    let mut window = unsafe{ derin::glutin_window::GlutinWindow::new(window_builder, counter_ui, theme).unwrap() };
    let _: Option<()> = window.run_forever(
        |value_delta, counter_ui, _| {
            value += value_delta;
            *counter_ui.container_mut().label.string_mut() = value.to_string();
            LoopFlow::Continue
        },
        |_, _| None
    );
}
