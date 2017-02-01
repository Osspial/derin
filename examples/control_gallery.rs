extern crate derin;
#[macro_use]
extern crate lazy_static;

use derin::ui::*;
use derin::ui::layout::{WidgetHints, NodeSpan, GridLayout, GridSize, TrackHints};
use derin::ui::intrinsics::*;
use derin::native::{Window, WindowConfig};

use std::slice::Iter as SliceIter;
use std::iter;
use std::iter::{Cloned, Repeat};

struct BasicParent {
    button0: TextButton,
    button1: TextButton,
    button2: TextButton,
    button3: TextButton
}

impl BasicParent {
    fn new() -> BasicParent {
        BasicParent {
            button0: TextButton::new("Hello World!".to_string()),
            button1: TextButton::new("Hello Again!".to_string()),
            button2: TextButton::new("Hello for a third time!".to_string()),
            button3: TextButton::new("More Hellos".to_string())
        }
    }
}

impl Node for BasicParent {
    fn type_name() -> &'static str {
        "BasicParent"
    }

    fn state_id(&self) -> u16 {
        self.button0.state_id() ^
        self.button1.state_id() ^
        self.button2.state_id() ^
        self.button3.state_id()
    }
}

impl<NP> ParentNode<NP> for BasicParent
        where NP: NodeProcessor<TextButton> {
    type Layout = BPLayout;

    fn children(&self, mut np: NP) -> Result<(), NP::Error> {
        np.add_child("button0", &self.button0)?;
        np.add_child("button1", &self.button1)?;
        np.add_child("button2", &self.button2)?;
        np.add_child("button3", &self.button3)?;
        Ok(())
    }

    fn child_layout(&self) -> BPLayout {
        BPLayout::default()
    }
}

#[derive(Default)]
struct BPLayout;

impl GridLayout for BPLayout {
    type WidgetHintsIter = Cloned<SliceIter<'static, WidgetHints>>;
    type ColHintsIter = Cloned<SliceIter<'static, TrackHints>>;
    type RowHintsIter = Repeat<TrackHints>;

    fn grid_size(&self) -> GridSize {
        GridSize::new(3, 2)
    }

    fn widget_hints(&self) -> Cloned<SliceIter<'static, WidgetHints>> {
        lazy_static!{
            static ref WIDGET_HINTS: [WidgetHints; 4] = [
                WidgetHints {
                    node_span: NodeSpan::new(0..2, 0),
                    ..WidgetHints::default()
                },
                WidgetHints {
                    node_span: NodeSpan::new(0, 1),
                    ..WidgetHints::default()
                },
                WidgetHints {
                    node_span: NodeSpan::new(1, 1),
                    ..WidgetHints::default()
                },
                WidgetHints {
                    node_span: NodeSpan::new(2, ..),
                    ..WidgetHints::default()
                }
            ];
        }

        WIDGET_HINTS.iter().cloned()
    }

    fn col_hints(&self) -> Cloned<SliceIter<'static, TrackHints>> {
        lazy_static!{
            static ref COL_HINTS: [TrackHints; 3] = [
                TrackHints::default(),
                TrackHints::default(),
                TrackHints {
                    min_size: 0,
                    fr_size: 0.0,
                    ..TrackHints::default()
                }
            ];
        }

        COL_HINTS.iter().cloned()
    }
    fn row_hints(&self) -> Repeat<TrackHints> {
        iter::repeat(TrackHints::default())
    }
}

fn main() {
    let mut window = Window::new(BasicParent::new(), WindowConfig::new()).unwrap();

    loop {
        window.process().unwrap();
    }
}