use dct::cursor::CursorIcon;
use dct::buttons::{MouseButton, Key, ModifierKeys};
use cgmath::{Point2, Vector2};
use tree::{Widget, WidgetIdent};
use arrayvec::ArrayVec;
use mbseq::{MouseButtonSequence, MouseButtonSequenceTrackPos};
use render::RenderFrame;
use popup::PopupAttributes;

use std::time::{Instant, Duration};

/// The set of operations to be performed after an event is processed by a widget.
#[derive(Default)]
pub struct EventOps<A, F: RenderFrame> {
    /// Deliver the given action to the Derin action loop.
    pub action: Option<A>,
    /// Change the keyboard focus to the given widget.
    ///
    /// Sending this results in the currently focused widget recieving a `LoseFocus` event and the
    /// newly focused widget recieving a `GainFocus` event, as long as the focus isn't being set to
    /// the currently focused widget, in which case no events are delivered.
    pub focus: Option<FocusChange>,
    /// Bubble the event to the parent widget.
    pub bubble: bool,
    /// Set the mouse cursor to the given position in the widget.
    pub cursor_pos: Option<Point2<i32>>,
    /// Set the mouse cursor's icon to the given icon.
    ///
    /// Note that this change is permanent, and isn't reset to the default cursor until another
    /// `cursor_icon` operation is recieved.
    pub cursor_icon: Option<CursorIcon>,
    /// Create a popup window with the given attributes.
    ///
    /// This *does not count as a child widget*, and events bubbled from the popup will not be
    /// delivered to the current widget.
    pub popup: Option<(Box<Widget<A, F>>, PopupAttributes)>
}

/// Changes the keyboard focus, removing the focus from another widget if necessary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FocusChange {
    /// Give keyboard focus to the widget after the widget sending a focus request.
    Next,
    /// Give keyboard focus to the widget before the widget sending a focus request.
    Prev,
    /// Give keyboard focus to the current widget.
    Take,
    /// Remove keyboard focus from the current widget.
    ///
    /// Note that, if another widget has keyboard focus, this event *does not remove focus from
    /// the other widget*. It only removes focus if the current widget has focus.
    Remove
}

/// Information regarding a pressed mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseDown {
    /// Which mouse button was pressed.
    pub button: MouseButton,
    /// The position at which the button was pressed, relative to the widget's origin.
    pub down_pos: Point2<i32>
}

/// The general state of user input devices when an event has occured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputState<'a> {
    /// The mouse buttons that have been pressed inside of the window.
    pub mouse_buttons_down: &'a [MouseDown],
    /// The mouse buttons that have been pressed inside of the widget.
    pub mouse_buttons_down_in_widget: &'a [MouseDown],
    /// The position of the mouse, relative to the widget's origin.
    pub mouse_pos: Point2<i32>,
    /// The modifier keys that have been pressed down.
    pub modifiers: ModifierKeys
}

/// Direct user input and timers, which are recieved and handled by widgets through the
/// `on_widget_event` function.
///
/// This is delivered to the widget in a few situtations:
/// * When the mouse is hovering over the widget, all mouse events are delivered.
/// * When the mouse was clicked within the widget, mouse movement events and `MouseUp` for the
///   pressed button are delivered.
/// * When the widget has recieved keyboard focus, all user input events are delivered.
/// * When the given amount of time has passed from a timer registered in `register_timers`, a
///  `Timer` event is delivered.
///
/// All point coordinates are given relative to the widget's origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetEvent<'a> {
    /// The mouse cursor has entered the widget at the given position.
    MouseEnter {
        /// The position where the mouse entered the widget.
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown]
    },
    /// The mouse cursor has exited the widget at the given position.
    MouseExit {
        /// The position where the mouse exited the widget.
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown]
    },
    /// The mouse cursor has entered a child at a given position.
    MouseEnterChild {
        /// The position where the cursor entered the child.
        enter_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown],
        /// The identifier of the child entered.
        child: WidgetIdent
    },
    /// The mouse cursor has exitd a child at a given position.
    MouseExitChild {
        /// The position where the cursor exited the child.
        exit_pos: Point2<i32>,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown],
        /// The identifier of the child entered.
        child: WidgetIdent
    },
    /// The mouse cursor has been moved to a new position.
    MouseMove {
        /// The position of the cursor, before it moved.
        old_pos: Point2<i32>,
        /// The position the cursor was moved to.
        new_pos: Point2<i32>,
        /// Whether or not the cursor was moved to a position within the widget.
        in_widget: bool,
        buttons_down: &'a [MouseDown],
        buttons_down_in_widget: &'a [MouseDown]
    },
    /// A mouse button has been pressed.
    MouseDown {
        /// The position of the cursor when the button was pressed.
        pos: Point2<i32>,
        /// Whether or not the button was pressed inside of the widget.
        ///
        /// If the widget doesn't have keyboard focus, this will always be `true`.
        in_widget: bool,
        /// The button that was pressed.
        button: MouseButton
    },
    /// A mouse button has been released.
    ///
    /// A `MouseUp` event will always be delivered for any given `MouseDown` event.
    MouseUp {
        /// The position of the cursor when the button was released.
        pos: Point2<i32>,
        /// Whether or not the button was released inside of the widget.
        in_widget: bool,
        /// Whether or not the button was pressed inside of the widget.
        pressed_in_widget: bool,
        /// The the position of the cursor when the button was pressed.
        down_pos: Point2<i32>,
        /// The button that was released.
        button: MouseButton
    },
    /// The widget has gained keyboard focus.
    GainFocus,
    /// The widget has lost keyboard focus.
    LoseFocus,
    /// The given character has been inputted by the user.
    ///
    /// This includes the effects of any modifier keys on the character - for example, if the `A` key
    /// is pressed while `Shift` is being held down, this will give the `'A'` character.
    Char(char),
    /// The given key has been pressed on the keyboard.
    KeyDown(Key, ModifierKeys),
    /// The given key has been released on the keyboard.
    KeyUp(Key, ModifierKeys),
    /// Enough time has elapsed for a registered timer to be triggered.
    Timer {
        /// The name of the timer.
        name: &'static str,
        /// The time at which the timer was registered.
        start_time: Instant,
        /// The time at which the timer was last triggered.
        last_trigger: Instant,
        /// The minimum duration which must pass between timer triggers.
        ///
        /// This isn't necessarily the actual time which has passed between triggers; a longer time
        /// may have elapsed since the last trigger.
        frequency: Duration,
        /// The number of times this timer has been triggered, not including this trigger.
        times_triggered: u64
    }
}

/// Non-borrowing equivalents of the `WidgetEvent` enum.
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
    /// Shift coordinates within the widget by the specified vector.
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
