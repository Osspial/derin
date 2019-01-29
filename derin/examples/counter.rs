// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin;
#[macro_use]
extern crate derin_macros;

use derin::{Window, WindowConfig, LoopFlow};
use derin::layout::{Margins, LayoutHorizontal};
use derin::widgets::{Button, Group, Label, Contents};
use derin::geometry::rect::DimsBox;

#[derive(WidgetContainer)]
#[derin(action = "i32")]
struct Counter {
    increment: Button<Option<i32>>,
    decrement: Button<Option<i32>>,
    label: Label
}

fn main() {
    let mut value = 0;
    let counter_ui = Group::new(
        Counter {
            increment: Button::new(Contents::Text("Increment".to_string()), Some(1)),
            decrement: Button::new(Contents::Text("Decrement".to_string()), Some(-1)),
            label: Label::new(Contents::Text(value.to_string()))
        },
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_config = WindowConfig {
        dimensions: Some(DimsBox::new2(400, 50)),
        title: "Counter Example".to_string(),
        ..WindowConfig::default()
    };

    let mut window = unsafe{ Window::new(window_config, counter_ui, theme).unwrap() };
    window.run_forever(
        |value_delta, counter_ui, _| {
            value += value_delta;
            *counter_ui.container_mut().label.contents_mut().as_text().unwrap() = value.to_string();
            println!("{}", value);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
