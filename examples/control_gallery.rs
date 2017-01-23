extern crate derin;

use derin::ui::*;
use derin::ui::layout::{GridSlot, NodeSpan, PlaceInCell, Place, GridLayout};
use derin::ui::intrinsics::*;
use derin::native::{Window, WindowConfig};

use derin::ui::layout::GridSize;

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
        self.button2.state_id()
    }
}

impl<NP> ParentNode<NP> for BasicParent
        where NP: NodeProcessor<TextButton> {
    type Layout = BPLayout;

    fn children(&mut self, mut np: NP) -> Result<(), NP::Error> {
        np.add_child("button0", &mut self.button0)?;
        np.add_child("button1", &mut self.button1)?;
        np.add_child("button2", &mut self.button2)?;
        np.add_child("button3", &mut self.button3)?;
        Ok(())
    }

    fn child_layout(&self) -> BPLayout {
        BPLayout::default()
    }
}

#[derive(Default)]
struct BPLayout {
    index: usize
}

impl GridLayout for BPLayout {
    fn grid_size(&self) -> GridSize {
        GridSize::new(3, 2)
    }
}

impl Iterator for BPLayout {
    type Item = GridSlot;

    fn next(&mut self) -> Option<GridSlot> {
        self.index += 1;
        match self.index {
            1 => Some(GridSlot {
                node_span: NodeSpan::new(0..2, 0),
                place_in_cell: PlaceInCell::new(Place::Stretch, Place::Stretch)
            }),
            2 => Some(GridSlot {
                node_span: NodeSpan::new(0, 1),
                place_in_cell: PlaceInCell::new(Place::Stretch, Place::Stretch)
            }),
            3 => Some(GridSlot {
                node_span: NodeSpan::new(1, 1),
                place_in_cell: PlaceInCell::new(Place::Stretch, Place::Stretch)
            }),
            4 => Some(GridSlot {
                node_span: NodeSpan::new(2, ..),
                place_in_cell: PlaceInCell::new(Place::Stretch, Place::Stretch)
            }),
            _ => None
        }
    }
}

fn main() {
    let mut window = Window::new(BasicParent::new(), WindowConfig::new()).unwrap();

    loop {
        window.process().unwrap();
    }
}