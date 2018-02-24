use dct::cursor::CursorIcon;
use dct::buttons::{MouseButton, Key, ModifierKeys};
use cgmath::{Point2, Vector2};
use cgmath_geometry::BoundBox;
use tree::{Node, NodeIdent};
use arrayvec::ArrayVec;
use mbseq::{MouseButtonSequence, MouseButtonSequenceTrackPos};
use render::RenderFrame;

use std::time::{Instant, Duration};

#[derive(Default)]
pub struct EventOps<A, F: RenderFrame> {
    pub action: Option<A>,
    pub focus: Option<FocusChange>,
    pub bubble: bool,
    pub cursor_pos: Option<Point2<i32>>,
    pub cursor_icon: Option<CursorIcon>,
    pub popup: Option<(Box<Node<A, F>>, PopupCreate)>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupCreate {
    pub rect: BoundBox<Point2<i32>>,
    pub title: String,
    pub decorations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusChange {
    Next,
    Prev,
    Take,
    Remove
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseDown {
    pub button: MouseButton,
    pub down_pos: Point2<i32>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeEvent<'a> {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_node: &'a [MouseDown]
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_node: &'a [MouseDown]
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_node: &'a [MouseDown],
        child: NodeIdent
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_node: &'a [MouseDown],
        child: NodeIdent
    },
    MouseMove {
        old: Point2<i32>,
        new: Point2<i32>,
        in_node: bool,
        buttons_down: &'a [MouseDown],
        buttons_down_in_node: &'a [MouseDown]
    },
    MouseDown {
        pos: Point2<i32>,
        in_node: bool,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        in_node: bool,
        pressed_in_node: bool,
        down_pos: Point2<i32>,
        button: MouseButton
    },
    GainFocus,
    LoseFocus,
    Char(char),
    KeyDown(Key, ModifierKeys),
    KeyUp(Key, ModifierKeys),
    Timer {
        name: &'static str,
        start_time: Instant,
        last_trigger: Instant,
        frequency: Duration,
        times_triggered: u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeEventOwned {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_node: MouseButtonSequence,
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_node: MouseButtonSequence,
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
        in_node: bool,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        in_node: bool,
        pressed_in_node: bool,
        down_pos: Point2<i32>,
        button: MouseButton
    },
    GainFocus,
    LoseFocus,
    Char(char),
    KeyDown(Key, ModifierKeys),
    KeyUp(Key, ModifierKeys),
    Timer {
        name: &'static str,
        start_time: Instant,
        last_trigger: Instant,
        frequency: Duration,
        times_triggered: u64
    }
}

impl<'a> NodeEvent<'a> {
    pub fn translate(self, dir: Vector2<i32>) -> NodeEvent<'a> {
        match self {
            NodeEvent::MouseEnter{ enter_pos, buttons_down, buttons_down_in_node } =>
                NodeEvent::MouseEnter {
                    enter_pos: enter_pos + dir,
                    buttons_down,
                    buttons_down_in_node,
                },
            NodeEvent::MouseExit{ exit_pos, buttons_down, buttons_down_in_node } =>
                NodeEvent::MouseExit {
                    exit_pos: exit_pos + dir,
                    buttons_down,
                    buttons_down_in_node,
                },
            NodeEvent::MouseEnterChild{ enter_pos, child, buttons_down, buttons_down_in_node } =>
                NodeEvent::MouseEnterChild {
                    enter_pos: enter_pos + dir,
                    child,
                    buttons_down,
                    buttons_down_in_node,
                },
            NodeEvent::MouseExitChild{ exit_pos, child, buttons_down, buttons_down_in_node } =>
                NodeEvent::MouseExitChild {
                    exit_pos: exit_pos + dir,
                    child, buttons_down,
                    buttons_down_in_node,
                },
            NodeEvent::MouseMove{ old, new, in_node, buttons_down, buttons_down_in_node } =>
                NodeEvent::MouseMove {
                    old: old + dir, new: new + dir,
                    in_node, buttons_down,
                    buttons_down_in_node,
                },
            NodeEvent::MouseDown{ pos, in_node, button } =>
                NodeEvent::MouseDown {
                    pos: pos + dir,
                    in_node, button
                },
            NodeEvent::MouseUp{ pos, in_node, pressed_in_node, down_pos, button } =>
                NodeEvent::MouseUp {
                    pos: pos + dir,
                    down_pos: down_pos + dir,
                    in_node, pressed_in_node, button
                },
            NodeEvent::GainFocus => NodeEvent::GainFocus,
            NodeEvent::LoseFocus => NodeEvent::LoseFocus,
            NodeEvent::Char(c) => NodeEvent::Char(c),
            NodeEvent::KeyDown(k, modifiers) => NodeEvent::KeyDown(k, modifiers),
            NodeEvent::KeyUp(k, modifiers) => NodeEvent::KeyUp(k, modifiers),
            NodeEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered } =>
                NodeEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered }
        }
    }
}

impl NodeEventOwned {
    pub fn as_borrowed<F, R>(&self, down_positions: &MouseButtonSequenceTrackPos, func: F) -> R
        where F: FnOnce(NodeEvent) -> R
    {
        let (mbd_array, mbdin_array): (ArrayVec<[_; 5]>, ArrayVec<[_; 5]>);
        match *self {
            NodeEventOwned::MouseEnter{ buttons_down, buttons_down_in_node, .. } |
            NodeEventOwned::MouseExit{ buttons_down, buttons_down_in_node, .. } |
            NodeEventOwned::MouseEnterChild{ buttons_down, buttons_down_in_node, .. } |
            NodeEventOwned::MouseExitChild{ buttons_down, buttons_down_in_node, .. } |
            NodeEventOwned::MouseMove{ buttons_down, buttons_down_in_node, .. } => {
                mbd_array = buttons_down.into_iter().filter_map(|button| down_positions.contains(button)).collect(); // RESUME HERE
                mbdin_array = buttons_down_in_node.into_iter().filter_map(|button| down_positions.contains(button)).collect();
            },
            _ => {
                mbd_array = ArrayVec::new();
                mbdin_array = ArrayVec::new();
            }
        }

        let event_borrowed = match *self {
            NodeEventOwned::MouseEnter{ enter_pos, .. } =>
                NodeEvent::MouseEnter {
                    enter_pos,
                    buttons_down: &mbd_array,
                    buttons_down_in_node: &mbdin_array,
                },
            NodeEventOwned::MouseExit{ exit_pos, .. } =>
                NodeEvent::MouseExit {
                    exit_pos,
                    buttons_down: &mbd_array,
                    buttons_down_in_node: &mbdin_array,
                },
            NodeEventOwned::MouseEnterChild{ enter_pos, child, .. } =>
                NodeEvent::MouseEnterChild {
                    enter_pos, child,
                    buttons_down: &mbd_array,
                    buttons_down_in_node: &mbdin_array,
                },
            NodeEventOwned::MouseExitChild{ exit_pos, child, .. } =>
                NodeEvent::MouseExitChild {
                    exit_pos, child,
                    buttons_down: &mbd_array,
                    buttons_down_in_node: &mbdin_array,
                },
            NodeEventOwned::MouseMove{ old, new, in_node, .. } =>
                NodeEvent::MouseMove {
                    old, new, in_node,
                    buttons_down: &mbd_array,
                    buttons_down_in_node: &mbdin_array,
                },
            NodeEventOwned::MouseDown{ pos, in_node, button } =>
                NodeEvent::MouseDown {
                    pos, in_node, button
                },
            NodeEventOwned::MouseUp{ pos, down_pos, in_node, pressed_in_node, button, .. } =>
                NodeEvent::MouseUp {
                    pos, down_pos, in_node, pressed_in_node, button
                },
            NodeEventOwned::GainFocus => NodeEvent::GainFocus,
            NodeEventOwned::LoseFocus => NodeEvent::LoseFocus,
            NodeEventOwned::Char(c) => NodeEvent::Char(c),
            NodeEventOwned::KeyDown(k, modifiers) => NodeEvent::KeyDown(k, modifiers),
            NodeEventOwned::KeyUp(k, modifiers) => NodeEvent::KeyUp(k, modifiers),
            NodeEventOwned::Timer{ name, start_time, last_trigger, frequency, times_triggered } =>
                NodeEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered }
        };
        func(event_borrowed)
    }
}

impl<'a> From<NodeEvent<'a>> for NodeEventOwned {
    fn from(event: NodeEvent<'a>) -> NodeEventOwned {
        let (mbd_sequence, mbdin_sequence): (MouseButtonSequence, MouseButtonSequence);
        match event {
            NodeEvent::MouseEnter{ buttons_down, buttons_down_in_node, .. } |
            NodeEvent::MouseExit{ buttons_down, buttons_down_in_node, .. } |
            NodeEvent::MouseEnterChild{ buttons_down, buttons_down_in_node, .. } |
            NodeEvent::MouseExitChild{ buttons_down, buttons_down_in_node, .. } |
            NodeEvent::MouseMove{ buttons_down, buttons_down_in_node, .. } => {
                mbd_sequence = buttons_down.into_iter().map(|d| d.button).collect();
                mbdin_sequence = buttons_down_in_node.into_iter().map(|d| d.button).collect();
            },
            _ => {
                mbd_sequence = MouseButtonSequence::new();
                mbdin_sequence = MouseButtonSequence::new();
            }
        }

        match event {
            NodeEvent::MouseEnter{ enter_pos, .. } =>
                NodeEventOwned::MouseEnter {
                    enter_pos,
                    buttons_down: mbd_sequence,
                    buttons_down_in_node: mbdin_sequence,
                },
            NodeEvent::MouseExit{ exit_pos, .. } =>
                NodeEventOwned::MouseExit {
                    exit_pos,
                    buttons_down: mbd_sequence,
                    buttons_down_in_node: mbdin_sequence,
                },
            NodeEvent::MouseEnterChild{ enter_pos, child, .. } =>
                NodeEventOwned::MouseEnterChild {
                    enter_pos, child,
                    buttons_down: mbd_sequence,
                    buttons_down_in_node: mbdin_sequence,
                },
            NodeEvent::MouseExitChild{ exit_pos, child, .. } =>
                NodeEventOwned::MouseExitChild {
                    exit_pos, child,
                    buttons_down: mbd_sequence,
                    buttons_down_in_node: mbdin_sequence,
                },
            NodeEvent::MouseMove{ old, new, in_node, .. } =>
                NodeEventOwned::MouseMove {
                    old, new, in_node,
                    buttons_down: mbd_sequence,
                    buttons_down_in_node: mbdin_sequence,
                },
            NodeEvent::MouseDown{ pos, in_node, button } =>
                NodeEventOwned::MouseDown {
                    pos, in_node, button
                },
            NodeEvent::MouseUp{ pos, in_node, pressed_in_node, down_pos, button, .. } =>
                NodeEventOwned::MouseUp {
                    pos, in_node, pressed_in_node, down_pos, button
                },
            NodeEvent::GainFocus => NodeEventOwned::GainFocus,
            NodeEvent::LoseFocus => NodeEventOwned::LoseFocus,
            NodeEvent::Char(c) => NodeEventOwned::Char(c),
            NodeEvent::KeyDown(k, modifiers) => NodeEventOwned::KeyDown(k, modifiers),
            NodeEvent::KeyUp(k, modifiers) => NodeEventOwned::KeyUp(k, modifiers),
            NodeEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered } =>
                NodeEventOwned::Timer{ name, start_time, last_trigger, frequency, times_triggered }
        }
    }
}
