extern crate dle;
extern crate dct;
extern crate derin_core;
extern crate cgmath;
extern crate cgmath_geometry;

#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

use derin_core::tree::{UpdateTag, NodeEvent, Renderer, Node};

use cgmath_geometry::BoundRect;

#[derive(Debug, Clone)]
pub struct Button {
    update_tag: UpdateTag,
    bounds: BoundRect<u32>,
    state: ButtonState
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Normal,
    Hover,
    Clicked,
    Disabled,
    Defaulted
}

impl Button {
    pub fn new() -> Button {
        Button {
            update_tag: UpdateTag::new(),
            bounds: BoundRect::new(0, 0, 0, 0),
            state: ButtonState::Normal
        }
    }
}

impl<A, R> Node<A, R> for Button
    where R: Renderer
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    fn bounds(&self) -> BoundRect<u32> {
        self.bounds
    }

    fn render(&self, renderer: &mut R) {}

    fn on_node_event(&mut self, event: NodeEvent) -> Option<A> {
        use self::NodeEvent::*;

        let new_state = match event {
            MouseEnter{..} => ButtonState::Hover,
            MouseExit{..} => ButtonState::Normal,
            MouseMove{..} => self.state,
            MouseDown{..} => ButtonState::Clicked,
            MouseUp{in_node: true, ..} => ButtonState::Hover,
            MouseUp{in_node: false, ..} => ButtonState::Normal,
            MouseEnterChild{..} |
            MouseExitChild{..} => unreachable!()
        };

        if new_state != self.state {
            self.update_tag.mark_update_this();
            self.state = new_state;
        }

        None
    }
}
