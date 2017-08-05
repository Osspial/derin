extern crate cgmath;
extern crate dct;

pub mod tree;
pub mod render;

use tree::{Node, Parent, RootID, ChildID};
use std::marker::PhantomData;

pub struct Root<A, N: Node<A>> {
    id: RootID,
    active_mouse_node: Vec<ChildID>,
    active_keyboard_node: Vec<ChildID>,
    pub root_node: N,
    _action_marker: PhantomData<A>
}

impl<A, N: Node<A>> Root<A, N> {
    #[inline]
    pub fn new(root_node: N) -> Root<A, N> {
        Root {
            id: RootID::new(),
            active_mouse_node: Vec::new(),
            active_keyboard_node: Vec::new(),
            root_node,
            _action_marker: PhantomData
        }
    }
}
