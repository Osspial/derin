# Derin - Derivable User Interface
[![Crates.io](https://img.shields.io/crates/v/derin.svg)](https://crates.io/crates/derin)
[![Docs](https://docs.rs/derin/badge.svg)](https://docs.rs/derin)

A UI library for Rust that makes creating a GUI as simple as declaring a `struct`.

## Setting up

```toml
[dependencies]

# The core library and APIs, used for creating and displaying widgets.
derin = "0.1"

# Adds #[derive(WidgetContainer)]. Is used to help create widget trees.
derin_macros = "0.1"
```

## Examples
Beyond looking at the API docs, you're encouraged to look at one of the provided examples, to see
more complex examples of how to use Derin. Those can be found in the [`derin/examples`](https://github.com/Osspial/derin/tree/master/derin/examples)
directory.

```rust
// A simple application that shows a click-able button to the user.
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
```

## License
Derin is made available under the Apache License, 2.0, viewable [in this repository](https://github.com/Osspial/derin/blob/master/LICENSE)
or on the [Apache Website](https://www.apache.org/licenses/LICENSE-2.0).

## Contribution
Unless explicitly stated otherwise, any contributions intentionally submitted for inclusion within
Derin will be licensed under the Apache-2.0 license, without any additional terms or conditions.
