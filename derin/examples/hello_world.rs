// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin;

use derin::{Window, WindowConfig, LoopFlow};
use derin::widgets::{Button, Contents};
use derin::geometry::rect::DimsBox;

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

    window.run_forever(
        // Whenever an action is recieved from a widget, this function is called.
        |print_string, _, _| {
            // Print out strings passed to the action function.
            println!("{}", print_string);
            LoopFlow::Continue
        },
        |_, _| None
    );
}
