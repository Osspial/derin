extern crate derin;
extern crate dct;

use derin::ui::*;
use derin::ui::layout::VerticalLayout;
use derin::ui::intrinsics::*;
use derin::native::{Window, WindowConfig};
use dct::events::MouseEvent;

use std::borrow::Borrow;

struct AddButton(&'static str);

impl Borrow<str> for AddButton {
    fn borrow(&self) -> &str {
        self.0
    }
}

impl Button for AddButton {
    type Action = ();

    fn on_mouse_event(&self, _: MouseEvent) -> Option<()> {
        Some(())
    }
}

struct BasicParent {
    label: TextLabel<&'static str>,
    bar: ProgressBar,
    button0: TextButton<AddButton>,
    button_vec: Vec<TextButton<AddButton>>
}

impl BasicParent {
    fn new() -> BasicParent {
        BasicParent {
            label: TextLabel::new("A Label"),
            bar: ProgressBar::new(ProgBarStatus::Frac(0.5)),
            button0: TextButton::new(AddButton("Add Button")),
            button_vec: Vec::new()
        }
    }
}

impl<NP> Parent<NP> for BasicParent
        where NP: NodeProcessor<TextButton<AddButton>> +
                  NodeProcessor<ProgressBar> +
                  NodeProcessor<TextLabel<&'static str>> {
    type ChildLayout = VerticalLayout;
    type ChildAction = ();

    fn children(&mut self, np: &mut NP) -> Result<(), NP::Error> {
        np.add_child(ChildId::Str("label"), &mut self.label)?;
        np.add_child(ChildId::Str("bar"), &mut self.bar)?;
        np.add_child(ChildId::Str("button0"), &mut self.button0)?;
        for (i, button) in self.button_vec.iter_mut().enumerate() {
            np.add_child(ChildId::Num(i as u32), button)?;
        }
        Ok(())
    }

    fn child_layout(&self) -> VerticalLayout {
        VerticalLayout::new(self.button_vec.len() as u32 + 3)
    }
}

fn main() {
    let mut window = Window::new(WidgetGroup::new(BasicParent::new()), &WindowConfig::new());

    loop {
        window.wait_actions(|_| {false}).unwrap();
        window.root.button_vec.push(TextButton::new(AddButton("Another Button")));
    }
}
