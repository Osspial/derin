// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use derin_common_types::cursor::CursorIcon;
use derin_common_types::buttons::{MouseButton, Key, ModifierKeys};
use crate::cgmath::{Point2, Vector2};
use crate::tree::{Widget, WidgetIdent};
use crate::render::RenderFrame;

use std::time::{Instant, Duration};

/// The set of operations to be performed after an event is processed by a widget.
#[derive(Default)]
pub struct EventOps<A> {
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
}

/// Changes the keyboard focus, removing the focus from another widget if necessary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FocusChange {
    /// Give keyboard focus to the widget after the widget sending a focus request.
    Next,
    /// Give keyboard focus to the widget before the widget sending a focus request.
    Prev,
    Parent,
    ChildIdent(WidgetIdent),
    ChildIndex(usize),
    /// Give keyboard focus to the current widget.
    Take,
    /// Remove keyboard focus from the current widget.
    ///
    /// Note that, if another widget has keyboard focus, this event *does not remove focus from
    /// the other widget*. It only removes focus if the current widget has focus.
    Remove,
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
    pub mouse_pos: Option<Point2<i32>>,
    /// The modifier keys that have been pressed down.
    pub modifiers: ModifierKeys,
    /// The keys that have been pressed inside of the window.
    pub keys_down: &'a [Key]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusSource {
    This,
    Parent,
    Child {
        ident: WidgetIdent,
        index: usize
    },
    Sibling {
        ident: WidgetIdent,
        delta: isize
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseHoverChange {
    /// The mouse cursor has entered the widget.
    Enter,
    /// The mouse cursor has exited the widget.
    Exit,
    /// The mouse cursor has entered a child.
    EnterChild(WidgetIdent),
    /// The mouse cursor has exited a child.
    ExitChild(WidgetIdent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetEventSourced<'a> {
    /// The event was dispatched directly to this widget.
    This(WidgetEvent),
    /// The event was dispatched to the specified child widget, and got bubbled up to this widget.
    Bubble(WidgetEvent, &'a [WidgetIdent])
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetEvent {
    /// The mouse cursor has been moved to a new position.
    MouseMove {
        /// The position of the cursor, before it moved.
        old_pos: Point2<i32>,
        /// The position the cursor was moved to.
        new_pos: Point2<i32>,
        /// Whether or not the cursor was moved to a position within the widget.
        ///
        /// If the new position is in a child widget, this value is `false`.
        in_widget: bool,
        hover_change: Option<MouseHoverChange>,
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
    MouseScrollLines(Vector2<i32>),
    MouseScrollPx(Vector2<i32>),
    /// The widget has gained keyboard focus.
    GainFocus(FocusSource),
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
    },
}

impl WidgetEventSourced<'_> {
    pub fn unwrap(self) -> WidgetEvent {
        match self {
            WidgetEventSourced::This(event) |
            WidgetEventSourced::Bubble(event, _) => event
        }
    }

    pub fn map(self, f: impl FnOnce(WidgetEvent) -> WidgetEvent) -> Self {
        match self {
            WidgetEventSourced::This(event) => WidgetEventSourced::This(f(event)),
            WidgetEventSourced::Bubble(event, bubble) => WidgetEventSourced::Bubble(f(event), bubble),
        }
    }
}

impl WidgetEvent {
    pub fn default_bubble(&self) -> bool {
        match *self {
            WidgetEvent::MouseScrollLines(..) |
            WidgetEvent::MouseScrollPx(..) |
            WidgetEvent::Char(..) |
            WidgetEvent::KeyDown(..) |
            WidgetEvent::KeyUp(..) => true,

            WidgetEvent::GainFocus(..) |
            WidgetEvent::LoseFocus |
            WidgetEvent::MouseMove{..} |
            WidgetEvent::MouseDown{..} |
            WidgetEvent::MouseUp{..} |
            WidgetEvent::Timer{..} => false
        }
    }

    /// Shift coordinates within the widget by the specified vector.
    pub fn translate(self, dir: Vector2<i32>) -> WidgetEvent {
        match self {
            WidgetEvent::MouseMove{ old_pos, new_pos, in_widget, hover_change } =>
                WidgetEvent::MouseMove {
                    old_pos: old_pos + dir, new_pos: new_pos + dir,
                    in_widget, hover_change,
                },
            WidgetEvent::MouseDown{ pos, in_widget, button } =>
                WidgetEvent::MouseDown {
                    pos: pos + dir,
                    in_widget, button,
                },
            WidgetEvent::MouseUp{ pos, in_widget, pressed_in_widget, down_pos, button } =>
                WidgetEvent::MouseUp {
                    pos: pos + dir,
                    down_pos: down_pos + dir,
                    in_widget, pressed_in_widget, button,
                },
            WidgetEvent::Char(..)              |
            WidgetEvent::LoseFocus             |
            WidgetEvent::GainFocus(..)         |
            WidgetEvent::Timer{..}             |
            WidgetEvent::KeyUp(..)             |
            WidgetEvent::KeyDown(..)           |
            WidgetEvent::MouseScrollPx(..)     |
            WidgetEvent::MouseScrollLines(..) =>
                self
        }
    }
}
