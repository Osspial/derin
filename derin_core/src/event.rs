use dct::cursor::CursorIcon;
use dct::buttons::{MouseButton, Key, ModifierKeys};
use cgmath::{Point2, Vector2};
use tree::{Widget, WidgetIdent};
use arrayvec::ArrayVec;
use mbseq::{MouseButtonSequence, MouseButtonSequenceTrackPos};
use render::RenderFrame;
use popup::PopupAttributes;

use std::time::{Instant, Duration};

#[derive(Default)]
pub struct EventOps<A, F: RenderFrame> {
    pub action: Option<A>,
    pub focus: Option<FocusChange>,
    pub bubble: bool,
    pub cursor_pos: Option<Point2<i32>>,
    pub cursor_icon: Option<CursorIcon>,
    pub popup: Option<(Box<Widget<A, F>>, PopupAttributes)>
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
pub struct InputState<'a> {
    pub mouse_buttons_down: &'a [MouseDown],
    pub mouse_buttons_down_in_widget: &'a [MouseDown],
    pub mouse_pos: Point2<i32>,
    pub modifiers: ModifierKeys
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetEvent<'a> {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown]
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown]
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown],
        child: WidgetIdent
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown],
        child: WidgetIdent
    },
    MouseMove {
        old_pos: Point2<i32>,
        new_pos: Point2<i32>,
        in_widget: bool,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown]
    },
    MouseDown {
        pos: Point2<i32>,
        in_widget: bool,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        in_widget: bool,
        pressed_in_widget: bool,
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
pub(crate) enum WidgetEventOwned {
    MouseEnter {
        enter_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_widget: MouseButtonSequence,
    },
    MouseExit {
        exit_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_widget: MouseButtonSequence,
    },
    MouseEnterChild {
        enter_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_widget: MouseButtonSequence,
        child: WidgetIdent
    },
    MouseExitChild {
        exit_pos: Point2<i32>,
        buttons_down: MouseButtonSequence,
        buttons_down_in_widget: MouseButtonSequence,
        child: WidgetIdent
    },
    MouseMove {
        old_pos: Point2<i32>,
        new_pos: Point2<i32>,
        in_widget: bool,
        buttons_down: MouseButtonSequence,
        buttons_down_in_widget: MouseButtonSequence
    },
    MouseDown {
        pos: Point2<i32>,
        in_widget: bool,
        button: MouseButton
    },
    MouseUp {
        pos: Point2<i32>,
        in_widget: bool,
        pressed_in_widget: bool,
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

impl<'a> WidgetEvent<'a> {
    pub fn translate(self, dir: Vector2<i32>) -> WidgetEvent<'a> {
        match self {
            WidgetEvent::MouseEnter{ enter_pos, buttons_down, buttons_down_in_widget } =>
                WidgetEvent::MouseEnter {
                    enter_pos: enter_pos + dir,
                    buttons_down,
                    buttons_down_in_widget,
                },
            WidgetEvent::MouseExit{ exit_pos, buttons_down, buttons_down_in_widget } =>
                WidgetEvent::MouseExit {
                    exit_pos: exit_pos + dir,
                    buttons_down,
                    buttons_down_in_widget,
                },
            WidgetEvent::MouseEnterChild{ enter_pos, child, buttons_down, buttons_down_in_widget } =>
                WidgetEvent::MouseEnterChild {
                    enter_pos: enter_pos + dir,
                    child,
                    buttons_down,
                    buttons_down_in_widget,
                },
            WidgetEvent::MouseExitChild{ exit_pos, child, buttons_down, buttons_down_in_widget } =>
                WidgetEvent::MouseExitChild {
                    exit_pos: exit_pos + dir,
                    child, buttons_down,
                    buttons_down_in_widget,
                },
            WidgetEvent::MouseMove{ old_pos, new_pos, in_widget, buttons_down, buttons_down_in_widget } =>
                WidgetEvent::MouseMove {
                    old_pos: old_pos + dir, new_pos: new_pos + dir,
                    in_widget, buttons_down,
                    buttons_down_in_widget,
                },
            WidgetEvent::MouseDown{ pos, in_widget, button } =>
                WidgetEvent::MouseDown {
                    pos: pos + dir,
                    in_widget, button
                },
            WidgetEvent::MouseUp{ pos, in_widget, pressed_in_widget, down_pos, button } =>
                WidgetEvent::MouseUp {
                    pos: pos + dir,
                    down_pos: down_pos + dir,
                    in_widget, pressed_in_widget, button
                },
            WidgetEvent::GainFocus => WidgetEvent::GainFocus,
            WidgetEvent::LoseFocus => WidgetEvent::LoseFocus,
            WidgetEvent::Char(c) => WidgetEvent::Char(c),
            WidgetEvent::KeyDown(k, modifiers) => WidgetEvent::KeyDown(k, modifiers),
            WidgetEvent::KeyUp(k, modifiers) => WidgetEvent::KeyUp(k, modifiers),
            WidgetEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered } =>
                WidgetEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered }
        }
    }
}

impl WidgetEventOwned {
    pub fn as_borrowed<F, R>(&self, down_positions: &MouseButtonSequenceTrackPos, func: F) -> R
        where F: FnOnce(WidgetEvent) -> R
    {
        let (mbd_array, mbdin_array): (ArrayVec<[_; 5]>, ArrayVec<[_; 5]>);
        match *self {
            WidgetEventOwned::MouseEnter{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEventOwned::MouseExit{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEventOwned::MouseEnterChild{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEventOwned::MouseExitChild{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEventOwned::MouseMove{ buttons_down, buttons_down_in_widget, .. } => {
                mbd_array = buttons_down.into_iter().filter_map(|button| down_positions.contains(button)).collect(); // RESUME HERE
                mbdin_array = buttons_down_in_widget.into_iter().filter_map(|button| down_positions.contains(button)).collect();
            },
            _ => {
                mbd_array = ArrayVec::new();
                mbdin_array = ArrayVec::new();
            }
        }

        let event_borrowed = match *self {
            WidgetEventOwned::MouseEnter{ enter_pos, .. } =>
                WidgetEvent::MouseEnter {
                    enter_pos,
                    buttons_down: &mbd_array,
                    buttons_down_in_widget: &mbdin_array,
                },
            WidgetEventOwned::MouseExit{ exit_pos, .. } =>
                WidgetEvent::MouseExit {
                    exit_pos,
                    buttons_down: &mbd_array,
                    buttons_down_in_widget: &mbdin_array,
                },
            WidgetEventOwned::MouseEnterChild{ enter_pos, child, .. } =>
                WidgetEvent::MouseEnterChild {
                    enter_pos, child,
                    buttons_down: &mbd_array,
                    buttons_down_in_widget: &mbdin_array,
                },
            WidgetEventOwned::MouseExitChild{ exit_pos, child, .. } =>
                WidgetEvent::MouseExitChild {
                    exit_pos, child,
                    buttons_down: &mbd_array,
                    buttons_down_in_widget: &mbdin_array,
                },
            WidgetEventOwned::MouseMove{ old_pos, new_pos, in_widget, .. } =>
                WidgetEvent::MouseMove {
                    old_pos, new_pos, in_widget,
                    buttons_down: &mbd_array,
                    buttons_down_in_widget: &mbdin_array,
                },
            WidgetEventOwned::MouseDown{ pos, in_widget, button } =>
                WidgetEvent::MouseDown {
                    pos, in_widget, button
                },
            WidgetEventOwned::MouseUp{ pos, down_pos, in_widget, pressed_in_widget, button, .. } =>
                WidgetEvent::MouseUp {
                    pos, down_pos, in_widget, pressed_in_widget, button
                },
            WidgetEventOwned::GainFocus => WidgetEvent::GainFocus,
            WidgetEventOwned::LoseFocus => WidgetEvent::LoseFocus,
            WidgetEventOwned::Char(c) => WidgetEvent::Char(c),
            WidgetEventOwned::KeyDown(k, modifiers) => WidgetEvent::KeyDown(k, modifiers),
            WidgetEventOwned::KeyUp(k, modifiers) => WidgetEvent::KeyUp(k, modifiers),
            WidgetEventOwned::Timer{ name, start_time, last_trigger, frequency, times_triggered } =>
                WidgetEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered }
        };
        func(event_borrowed)
    }
}

impl<'a> From<WidgetEvent<'a>> for WidgetEventOwned {
    fn from(event: WidgetEvent<'a>) -> WidgetEventOwned {
        let (mbd_sequence, mbdin_sequence): (MouseButtonSequence, MouseButtonSequence);
        match event {
            WidgetEvent::MouseEnter{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEvent::MouseExit{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEvent::MouseEnterChild{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEvent::MouseExitChild{ buttons_down, buttons_down_in_widget, .. } |
            WidgetEvent::MouseMove{ buttons_down, buttons_down_in_widget, .. } => {
                mbd_sequence = buttons_down.into_iter().map(|d| d.button).collect();
                mbdin_sequence = buttons_down_in_widget.into_iter().map(|d| d.button).collect();
            },
            _ => {
                mbd_sequence = MouseButtonSequence::new();
                mbdin_sequence = MouseButtonSequence::new();
            }
        }

        match event {
            WidgetEvent::MouseEnter{ enter_pos, .. } =>
                WidgetEventOwned::MouseEnter {
                    enter_pos,
                    buttons_down: mbd_sequence,
                    buttons_down_in_widget: mbdin_sequence,
                },
            WidgetEvent::MouseExit{ exit_pos, .. } =>
                WidgetEventOwned::MouseExit {
                    exit_pos,
                    buttons_down: mbd_sequence,
                    buttons_down_in_widget: mbdin_sequence,
                },
            WidgetEvent::MouseEnterChild{ enter_pos, child, .. } =>
                WidgetEventOwned::MouseEnterChild {
                    enter_pos, child,
                    buttons_down: mbd_sequence,
                    buttons_down_in_widget: mbdin_sequence,
                },
            WidgetEvent::MouseExitChild{ exit_pos, child, .. } =>
                WidgetEventOwned::MouseExitChild {
                    exit_pos, child,
                    buttons_down: mbd_sequence,
                    buttons_down_in_widget: mbdin_sequence,
                },
            WidgetEvent::MouseMove{ old_pos, new_pos, in_widget, .. } =>
                WidgetEventOwned::MouseMove {
                    old_pos, new_pos, in_widget,
                    buttons_down: mbd_sequence,
                    buttons_down_in_widget: mbdin_sequence,
                },
            WidgetEvent::MouseDown{ pos, in_widget, button } =>
                WidgetEventOwned::MouseDown {
                    pos, in_widget, button
                },
            WidgetEvent::MouseUp{ pos, in_widget, pressed_in_widget, down_pos, button, .. } =>
                WidgetEventOwned::MouseUp {
                    pos, in_widget, pressed_in_widget, down_pos, button
                },
            WidgetEvent::GainFocus => WidgetEventOwned::GainFocus,
            WidgetEvent::LoseFocus => WidgetEventOwned::LoseFocus,
            WidgetEvent::Char(c) => WidgetEventOwned::Char(c),
            WidgetEvent::KeyDown(k, modifiers) => WidgetEventOwned::KeyDown(k, modifiers),
            WidgetEvent::KeyUp(k, modifiers) => WidgetEventOwned::KeyUp(k, modifiers),
            WidgetEvent::Timer{ name, start_time, last_trigger, frequency, times_triggered } =>
                WidgetEventOwned::Timer{ name, start_time, last_trigger, frequency, times_triggered }
        }
    }
}
