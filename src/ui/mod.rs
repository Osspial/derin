pub mod intrinsics;
pub mod layout;

pub use dct::geometry;
use dct::events::MouseEvent;

use self::layout::GridLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildId {
    Str(&'static str),
    Num(u32)
}

pub trait NodeProcessor<N: Node>: Sized + NodeProcessorAT {
    /// Add a child to the node processor.
    ///
    /// Unsafe, because it cannot guarantee that `node` is truely immutable (due to `Cell`, `RefCell`,
    /// etc.). Derin does not support interior mutability, and mutating a node through interior
    /// mutability while it is being processed for events is undefined behavior.
    unsafe fn add_child(&mut self, ChildId, node: &N) -> Result<(), Self::Error>;
}

pub trait NodeProcessorAT: Sized {
    type Error;

    type TextButtonData: Default;
    type TextLabelData: Default;
    type WidgetGroupData: Default;
}

pub trait Node {
    type Data;

    fn type_name(&self) -> &'static str;
    /// An identifier for the current state. Calling this function provides only one guarantee: that
    /// if `node_a != node_b`, `state_id(node_a) != state_id(node_b)`.
    fn state_id(&self) -> u16;

    fn data(&self) -> &Self::Data;
    fn data_mut(&mut self) -> &mut Self::Data;
}

pub trait ActionNode: Node {
    type Action;
}

pub trait Parent<NP>
        where NP: NodeProcessorAT
{
    type ChildAction;
    type ChildLayout: GridLayout;

    fn children(&self, NP) -> Result<(), NP::Error>;
    fn child_layout(&self) -> Self::ChildLayout;
}

pub trait Control {
    type Action;

    fn on_mouse_event(&self, MouseEvent) -> Option<Self::Action> {None}
}


impl<N: Node> NodeProcessor<N> for () {
    unsafe fn add_child(&mut self, _: ChildId, _: &N) -> Result<(), ()> {Ok(())}
}

impl NodeProcessorAT for () {
    type Error = ();

    type TextButtonData = ();
    type TextLabelData = ();
    type WidgetGroupData = ();
}
