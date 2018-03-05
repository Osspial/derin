use dct::layout::{WidgetPos, GridSize, Margins, Align2, WidgetSpan};
use core::tree::NodeIdent;

pub trait GridLayout {
    fn hints(&self, node_ident: NodeIdent, node_index: usize, num_nodes: usize) -> Option<WidgetPos>;
    fn grid_size(&self, num_nodes: usize) -> GridSize;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutHorizontal {
    pub widget_margins: Margins<i32>,
    pub widget_place: Align2
}

impl LayoutHorizontal {
    #[inline(always)]
    pub fn new(widget_margins: Margins<i32>, widget_place: Align2) -> LayoutHorizontal {
        LayoutHorizontal{ widget_margins, widget_place }
    }
}

impl GridLayout for LayoutHorizontal {
    fn hints(&self, _: NodeIdent, node_index: usize, num_nodes: usize) -> Option<WidgetPos> {
        match node_index >= num_nodes {
            true => None,
            false => Some(WidgetPos {
                node_span: WidgetSpan::new(node_index as u32, 0),
                margins: self.widget_margins,
                place_in_cell: self.widget_place,
                ..WidgetPos::default()
            })
        }
    }

    #[inline]
    fn grid_size(&self, num_nodes: usize) -> GridSize {
        GridSize::new(num_nodes as u32, 1)
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutVertical {
    pub widget_margins: Margins<i32>,
    pub widget_place: Align2
}

impl LayoutVertical {
    #[inline(always)]
    pub fn new(widget_margins: Margins<i32>, widget_place: Align2) -> LayoutVertical {
        LayoutVertical{ widget_margins, widget_place }
    }
}

impl GridLayout for LayoutVertical {
    fn hints(&self, _: NodeIdent, node_index: usize, num_nodes: usize) -> Option<WidgetPos> {
        match node_index >= num_nodes {
            true => None,
            false => Some(WidgetPos {
                node_span: WidgetSpan::new(0, node_index as u32),
                margins: self.widget_margins,
                place_in_cell: self.widget_place,
                ..WidgetPos::default()
            })
        }
    }

    #[inline]
    fn grid_size(&self, num_nodes: usize) -> GridSize {
        GridSize::new(1, num_nodes as u32)
    }
}
