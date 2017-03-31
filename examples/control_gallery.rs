extern crate derin;
extern crate dct;

use derin::ui::*;
use derin::ui::widgets::*;
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

impl<NPI> Parent<NPI> for BasicParent
        where NPI: NodeProcessorInit,
              NPI::GridProcessor: NodeProcessorGrid<TextButton<AddButton>> +
                                  NodeProcessorGrid<ProgressBar> +
                                  NodeProcessorGrid<TextLabel<&'static str>>
{
    type ChildAction = ();

    fn children(&mut self, npi: NPI) -> Result<(), NPI::Error> {
        use derin::ui::hints::{GridSize, NodeSpan, WidgetHints, TrackHints};
        use std::iter;

        let mut np = npi.init_grid(
            GridSize::new(1, 3 + self.button_vec.len() as u32),
            iter::empty(),
            iter::once(TrackHints {
                fr_size: 0.0,
                ..TrackHints::default()
            })
        );

        let label_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 0..1),
            ..WidgetHints::default()
        };
        np.add_child(ChildId::Str("label"), label_hints, &mut self.label)?;

        let bar_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 1..2),
            ..WidgetHints::default()
        };
        np.add_child(ChildId::Str("bar"),  bar_hints, &mut self.bar)?;

        let button0_hints = WidgetHints {
            node_span: NodeSpan::new(0..1, 2..3),
            ..WidgetHints::default()
        };
        np.add_child(ChildId::Str("button0"), button0_hints, &mut self.button0)?;

        for (i, button) in self.button_vec.iter_mut().enumerate() {
            let button_hints = WidgetHints {
                node_span: NodeSpan::new(0..1, 3+i as u32..3+i as u32+1),
                ..WidgetHints::default()
            };
            np.add_child(ChildId::Num(i as u32), button_hints, button)?;
        }
        Ok(())
    }
}

fn main() {
    let mut window = Window::new(WidgetGroup::new(BasicParent::new()), &WindowConfig::new());

    loop {
        window.wait_actions(|_| {false}).unwrap();
        window.root.button_vec.push(TextButton::new(AddButton("Another Button")));
    }
}
