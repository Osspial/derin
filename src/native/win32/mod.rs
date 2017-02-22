mod wrapper;

use std::io::Error;

use ui::NodeProcessorAT;

pub struct NodeTraverser<'a, N>
        where N: 'a
{
    /// The branch that this instance of NodeTraverser is currently processing
    node: &'a mut N,
    force_child_updates: bool
}

impl<'a, N> NodeProcessorAT for NodeTraverser<'a, N>
        where N: 'a
{
    type Error = Error;
}
