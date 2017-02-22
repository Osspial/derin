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

pub trait NodeProcessorAT: Sized {
    type Error;
}

pub trait NodeProcessor<N: Node<Self::WidgetDataWrapper>>: NodeProcessorAT {
    type WidgetDataWrapper: WidgetDataWrapper<N::Inner>;

    /// Add a child to the node processor.
    fn add_child<'a>(&'a mut self, ChildId, node: &'a N) -> Result<(), Self::Error>
            where N: Parent<Self>;
}

pub trait Node<W: WidgetDataWrapper<Self::Inner>> {
    type Inner;

    fn type_name(&self) -> &'static str;
    /// An identifier for the current state. Calling this function provides only one guarantee: that
    /// if `node_a != node_b`, `state_id(node_a) != state_id(node_b)`.
    fn state_id(&self) -> u16;

    fn data(&self) -> &W;
    fn data_mut(&mut self) -> &mut W;
}

pub trait WidgetDataWrapper<I> {
    fn from_widget_data(I) -> Self;
    fn inner(&self) -> &I;
    fn inner_mut(&mut self) -> &mut I;
    fn unwrap(self) -> I;
}

pub trait ActionNode<W: WidgetDataWrapper<Self::Inner>>: Node<W> {
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
