extern crate tint;

use tint::ui::intrinsics::*;
use tint::native::{Window, WindowConfig};

fn main() {
	let mut window = Window::new(TextButton::new("Hello World"), WindowConfig::new()).unwrap();

	loop {
		window.process().unwrap();
	}
}