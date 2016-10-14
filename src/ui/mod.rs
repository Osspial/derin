use std::ops::{Deref, DerefMut};

pub enum MouseEvent {
    Click(MouseButton),
    Scroll(f32, f32)
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}


pub trait TreeCrawler {
    fn with_node<N: Node>(&mut self, ident: Option<&str>, N);
    fn with_control_node<C: Control>(&mut self, ident: Option<&str>, C);
    fn with_node_group<N: Node>(&mut self, ident: Option<&str>, NodeGroup<N>);
}

pub struct NodeGroup<N: Node> {
    group: N,
    num_updates: u64
}

impl<N: Node> NodeGroup<N> {
    pub fn new(group: N) -> NodeGroup<N> {
        NodeGroup {
            group: group,
            num_updates: 0
        }
    }

    pub fn num_updates(&self) -> u64 {
        self.num_updates
    }

    pub fn unwrap(self) -> N {
        self.group
    }
}

impl<N: Node> Deref for NodeGroup<N> {
    type Target = N;

    fn deref(&self) -> &N {
        &self.group
    }
}

impl<N: Node> DerefMut for NodeGroup<N> {
    fn deref_mut(&mut self) -> &mut N {
        self.num_updates += 1;
        &mut self.group
    }
}

impl<N: Node> AsRef<N> for NodeGroup<N> {
    fn as_ref(&self) -> &N {
        self
    }
}

impl<N: Node> AsMut<N> for NodeGroup<N> {
    fn as_mut(&mut self) -> &mut N {
        self
    }
}

pub trait Node {
    fn crawl_children<T: TreeCrawler>(&self, T);

    fn ty(&self) -> &str;
    fn state(&self) -> Option<&str> {None}
    fn class(&self) -> Option<&str> {None}
}

pub trait Control: Node {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Option<Self::Action> {None}
}
