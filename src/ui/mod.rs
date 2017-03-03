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

pub trait NodeProcessor<W, N>: NodeProcessorAT
        where W: NodeDataWrapper<N::Inner>, N: Node<W>
{
    /// Add a child to the node processor.
    fn add_child<'a>(&'a mut self, ChildId, node: &'a mut N) -> Result<(), Self::Error>
            where N: Parent<Self>;
}

pub trait WrapperNodeProcessor<N>: NodeProcessor<<Self as WrapperNodeProcessor<N>>::NodeDataWrapper, N>
        where N: Node<Self::NodeDataWrapper>
{
    type NodeDataWrapper: NodeDataWrapper<N::Inner>;
}

pub trait Node<W: NodeDataWrapper<Self::Inner>> {
    type Inner;

    fn type_name(&self) -> &'static str;
    /// An identifier for the current state. Calling this function provides only one guarantee: that
    /// if `node_a != node_b`, `state_id(node_a) != state_id(node_b)`.
    fn state_id(&self) -> u16;

    fn data(&self) -> &W;
    fn data_mut(&mut self) -> &mut W;
}

pub trait NodeDataWrapper<I> {
    fn from_node_data(I) -> Self;
    fn inner(&self) -> &I;
    fn inner_mut(&mut self) -> &mut I;
    fn unwrap(self) -> I;
}

pub trait ActionNode<W: NodeDataWrapper<Self::Inner>>: Node<W> {
    type Action;
}

pub trait Parent<NP>
        where NP: NodeProcessorAT
{
    type ChildAction;
    type ChildLayout: GridLayout;

    fn children(&mut self, NP) -> Result<(), NP::Error>;
    fn child_layout(&self) -> Self::ChildLayout;
}

pub trait Control {
    type Action;

    fn on_mouse_event(&self, MouseEvent) -> Option<Self::Action> {None}
}

impl<W, N> NodeProcessor<W, N> for ()
        where W: NodeDataWrapper<N::Inner>, N: Node<W>
{
    fn add_child<'a>(&'a mut self, _: ChildId, _: &'a mut N) -> Result<(), ()> {Ok(())}
}

impl NodeProcessorAT for () {
    type Error = ();
}
