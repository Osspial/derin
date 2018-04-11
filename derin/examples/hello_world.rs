// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate derin;

use derin::{Window, WindowConfig, LoopFlow};
use derin::widgets::{Button, Contents};
use derin::geometry::DimsBox;

fn main() {
    let print_string = "Prints to the console.";

    // Create a new window that displays a GUI on the desktop.
    let mut window = unsafe{ Window::new(
        // Set the attributes with which we'll create the window.
        WindowConfig {
            dimensions: Some(DimsBox::new2(400, 50)),
            title: "Derin's Hello World".to_string(),
            ..WindowConfig::default()
        },
        // Create a button that displays "Hello World" to the user, and passes `print_string`
        // to the UI's action loop.
        Button::new(Contents::Text("Hello, World!".to_string()), Some(print_string)),
        // Set the theme to the default theme.
        derin::theme::Theme::default()
    ).unwrap() };

    let _: Option<()> = window.run_forever(
        // Whenever an action is recieved from a widget, this function is called.
        |print_string, _, _| {
            // Print out strings passed to the action function.
            println!("{}", print_string);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
