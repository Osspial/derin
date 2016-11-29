extern crate derin;

use derin::ui::*;
use derin::ui::layout::VerticalLayout;
use derin::ui::intrinsics::*;
use derin::native::{Window, WindowConfig};

struct BasicParent {
    button0: TextButton,
    button1: TextButton
}

impl BasicParent {
    fn new() -> BasicParent {
        BasicParent {
            button0: TextButton::new("Hello World!".to_string()),
            button1: TextButton::new("Hello Again!".to_string())
        }
    }
}

impl Node for BasicParent {
    fn type_name() -> &'static str {
        "BasicParent"
    }

    fn state_id(&self) -> u16 {
        self.button0.state_id() ^
        self.button1.state_id()
    }
}

impl<NP> ParentNode<NP> for BasicParent
        where NP: NodeProcessor<TextButton> {
    type Layout = VerticalLayout;

    fn children(&mut self, mut np: NP) -> Result<(), NP::Error> {
        np.add_child("button0", &mut self.button0)?;
        np.add_child("button1", &mut self.button1)?;
        Ok(())
    }

    fn child_layout(&self) -> VerticalLayout {
        VerticalLayout::new(2)
    }
}

fn main() {
    let mut window = Window::new(BasicParent::new(), WindowConfig::new()).unwrap();

    loop {
        window.process().unwrap();
    }
}