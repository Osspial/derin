mod wrapper;

use user32;

use std::ptr;
use std::mem;

use boolinator::Boolinator;

use native::{NativeResult, NativeError};
use native::WindowConfig;

use ui::{ChildId, Node, ActionNode, Control, NodeProcessor, NodeProcessorAT, Parent};
use ui::intrinsics::{TextButton, TextLabel};
use ui::layout::{GridLayout, SingleNodeLayout};

use dle::Tr;


pub struct Window<N: ActionNode> {
    pub root: N,
    action: Option<N::Action>,
    self_ptr: *const Window<N>
}

impl<N: ActionNode> Window<N> {
    pub fn new(root: N, config: &WindowConfig) -> NativeResult<Window<N>> {
        unimplemented!()
    }
}


#[doc(hidden)]
pub struct NodeTraverser<'a, N: Parent<NodeTraverser<'a, N>> + 'a> {
    /// The branch that this instance of NodeTraverser is currently processing
    node: &'a mut N,
    force_child_updates: bool,

    child_widget_hints: <N::ChildLayout as GridLayout>::WidgetHintsIter
}

impl<'a, N: Parent<NodeTraverser<'a, N>> + 'a> NodeProcessorAT for NodeTraverser<'a, N> {
    type Error = NativeError;

    type TextButtonData = ();
    type TextLabelData = ();
    type WidgetGroupData = ();
}
