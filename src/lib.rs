extern crate dle;
extern crate dct;
extern crate derin_core;
extern crate cgmath;

#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

use derin_core::render::DVertex;
use derin_core::tree::{DrawTag, RawEvent, Node};

#[derive(Debug, Clone)]
pub struct Button {
    draw_tag: DrawTag,
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
            draw_tag: DrawTag::new(),
            state: ButtonState::Normal
        }
    }
}

impl<A> Node<A> for Button {
    #[inline]
    fn draw_tag(&self) -> &DrawTag {
        &self.draw_tag
    }

    fn render<F: FnMut(DVertex)>(&self, _for_each_vertex: F) {}

    fn on_raw_event(&mut self, event: RawEvent) -> Option<A> {
        use self::RawEvent::*;

        let new_state = match event {
            MouseEnter{..} => ButtonState::Hover,
            MouseExit{..} => ButtonState::Normal,
            MouseMove{..} => self.state,
            MouseClick{..} => ButtonState::Clicked,
            MouseRelease{in_node: true, ..} => ButtonState::Hover,
            MouseRelease{in_node: false, ..} => ButtonState::Normal,
            MouseEnterChild{..} |
            MouseExitChild{..} => unreachable!()
        };

        if new_state != self.state {
            self.draw_tag.mark_draw_self();
            self.state = new_state;
        }

        None
    }
}
