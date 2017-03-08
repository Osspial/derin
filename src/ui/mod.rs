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

pub trait NodeProcessor<N: Node>: NodeProcessorAT {
    /// Add a child to the node processor.
    fn add_child<'a>(&'a mut self, ChildId, node: &'a mut N) -> Result<(), Self::Error>;
}

pub trait NodeDataRegistry<N>
        where N: Node<Wrapper = Self::NodeDataWrapper>
{
    type NodeDataWrapper: NodeDataWrapper<N::Inner>;
}

pub trait Node {
    type Wrapper: NodeDataWrapper<Self::Inner>;
    type Inner;
    type Action;

    fn type_name(&self) -> &'static str;

    fn wrapper(&self) -> &Self::Wrapper;
    fn wrapper_mut(&mut self) -> &mut Self::Wrapper;
}

pub trait NodeDataWrapper<I> {
    fn from_node_data(I) -> Self;
    fn inner(&self) -> &I;
    fn inner_mut(&mut self) -> &mut I;
    fn unwrap(self) -> I;
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

impl<N: Node> NodeProcessor<N> for () {
    fn add_child<'a>(&'a mut self, _: ChildId, _: &'a mut N) -> Result<(), ()> {Ok(())}
}

impl NodeProcessorAT for () {
    type Error = ();
}
