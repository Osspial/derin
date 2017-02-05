extern crate derin;

use derin::ui::*;
use derin::ui::layout::VerticalLayout;
use derin::ui::intrinsics::*;
use derin::native::{Window, WindowConfig};

struct AddButton(&'static str);

impl AsRef<str> for AddButton {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl Control for AddButton {
    type Action = ();

    fn on_mouse_event(&self, _: MouseEvent) -> Option<()> {
        Some(())
    }
}

struct BasicParent {
    label: TextLabel<&'static str>,
    button0: TextButton<AddButton>,
    button_vec: Vec<TextButton<AddButton>>
}

impl BasicParent {
    fn new() -> BasicParent {
        BasicParent {
            label: TextLabel::new("A Label"),
            button0: TextButton::new(AddButton("Add Button")),
            button_vec: Vec::new()
        }
    }
}

impl Node for BasicParent {
    fn type_name(&self) -> &'static str {
        "BasicParent"
    }

    fn state_id(&self) -> u16 {
        let mut state_id = self.label.state_id() ^ self.button0.state_id();
        for button in &self.button_vec {
            state_id ^= button.state_id();
        }
        state_id
    }
}

impl ActionNode for BasicParent {
    type Action = ();
}

impl<NP> ParentNode<NP> for BasicParent
        where NP: NodeProcessor<TextButton<AddButton>> +
                  NodeProcessor<TextLabel<&'static str>> {
    type Layout = VerticalLayout;

    fn children(&self, mut np: NP) -> Result<(), NP::Error> {
        unsafe {
            np.add_child(ChildId::Str("label"), &self.label)?;
            np.add_child(ChildId::Str("button0"), &self.button0)?;
            for (i, button) in self.button_vec.iter().enumerate() {
                np.add_child(ChildId::Num(i as u32), button)?;
            }
            Ok(())
        }
    }

    fn child_layout(&self) -> VerticalLayout {
        VerticalLayout::new(self.button_vec.len() as u32 + 2)
    }
}

fn main() {
    let mut window = Window::new(BasicParent::new(), &WindowConfig::new()).unwrap();

    loop {
        for _ in window.wait_actions().unwrap() {break}
        window.root.button_vec.push(TextButton::new(AddButton("Add Button")));
    }
}