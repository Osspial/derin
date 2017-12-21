use dct::buttons::MouseButton;
use cgmath::Point2;
use tree::NodeIdent;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventOps<A> {
    pub action: Option<A>,
    pub bubble: bool
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeEvent<'a> {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton]
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton]
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton],
        child: NodeIdent
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton],
        child: NodeIdent
    },
    MouseMove {
        old: Point2<i32>,
        new: Point2<i32>,
        in_node: bool,
        buttons_down: &'a [MouseButton],
        buttons_down_in_node: &'a [MouseButton]
    },
    MouseDown {
        pos: Point2<i32>,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        in_node: bool,
        pressed_in_node: bool,
        button: MouseButton
    }
}

