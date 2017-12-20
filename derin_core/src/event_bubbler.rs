use mbseq::MouseButtonSequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeEventStoredSequence {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_node: MouseButtonSequence
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_node: MouseButtonSequence
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_node: MouseButtonSequence,
        child: NodeIdent
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_node: MouseButtonSequence,
        child: NodeIdent
    },
    MouseMove {
        old: Point2<i32>,
        new: Point2<i32>,
        in_node: bool,
        buttons_down: MouseButtonSequence,
        buttons_down_in_node: MouseButtonSequence
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

pub struct
