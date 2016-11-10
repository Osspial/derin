extern crate derin;

use derin::ui::intrinsics::*;
use derin::native::{Window, WindowConfig};

fn main() {
	let mut window = Window::new(TextButton::new("Hello World"), WindowConfig::new()).unwrap();

	loop {
		window.process().unwrap();
	}
}