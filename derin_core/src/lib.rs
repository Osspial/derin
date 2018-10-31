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

#![feature(range_contains, nll, specialization)]

use cgmath_geometry::cgmath;
extern crate cgmath_geometry;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate derin_common_types;
extern crate arrayvec;
extern crate itertools;

pub mod timer;
#[macro_use]
pub mod tree;
pub mod event;
pub mod popup;
pub mod render;
mod mbseq;
mod widget_stack;
mod offset_widget;
mod event_loop_ops;
mod widget_tree;
mod event_dispatcher;
mod update_state;

use crate::cgmath::{Point2, Vector2, Bounded};
use cgmath_geometry::{D2, rect::DimsBox};

use std::collections::VecDeque;

use crate::tree::*;
pub use crate::event_loop_ops::{EventLoopResult, PopupDelta};
use crate::timer::TimerList;
use crate::popup::PopupMap;
use crate::render::RenderFrame;
use crate::mbseq::MouseButtonSequenceTrackPos;
use crate::widget_stack::WidgetStackBase;
use derin_common_types::buttons::{MouseButton, Key, ModifierKeys};
use derin_common_types::cursor::CursorIcon;

pub struct Root<A, N, F>
    where N: Widget<A, F> + 'static,
          A: 'static,
          F: RenderFrame + 'static
{
    // Event handing and dispatch
    id: RootID,
    widget_stack_base: WidgetStackBase<A, F>,
    pub actions: VecDeque<A>,
    event_stamp: u32,
    timer_list: TimerList,

    // Input State
    mouse_pos: Point2<i32>,
    mouse_buttons_down: MouseButtonSequenceTrackPos,
    pub modifiers: ModifierKeys,
    keys_down: Vec<Key>,

    // Render State
    cursor_icon: CursorIcon,
    needs_redraw: bool,

    // User data
    pub root_widget: N,
    pub theme: F::Theme,
    popup_widgets: PopupMap<A, F>,

    // Per-frame information
    set_cursor_pos: Option<Point2<i32>>,
    set_cursor_icon: Option<CursorIcon>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    MouseMove(Point2<i32>),
    MouseEnter(Point2<i32>),
    MouseExit(Point2<i32>),
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseScrollLines(Vector2<i32>),
    MouseScrollPx(Vector2<i32>),
    WindowResize(DimsBox<D2, u32>),
    KeyDown(Key),
    KeyUp(Key),
    Char(char),
    Timer,
    Redraw
}

/// Whether to continue or abort a loop.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopFlow<R> {
    /// Continue the loop.
    Continue,
    /// Abort the loop, returning the contained value.
    Break(R)
}

impl<A, N, F> Root<A, N, F>
    where N: Widget<A, F>,
          F: RenderFrame
{
    #[inline]
    pub fn new(mut root_widget: N, theme: F::Theme, dims: DimsBox<D2, u32>) -> Root<A, N, F> {
        // TODO: DRAW ROOT AND DO INITIAL LAYOUT
        *root_widget.rect_mut() = dims.cast().unwrap_or(DimsBox::max_value()).into();
        Root {
            id: RootID::new(),
            widget_stack_base: WidgetStackBase::new(),
            actions: VecDeque::new(),
            event_stamp: 1,
            timer_list: TimerList::new(None),

            mouse_pos: Point2::new(-1, -1),
            mouse_buttons_down: MouseButtonSequenceTrackPos::new(),
            modifiers: ModifierKeys::empty(),
            keys_down: Vec::new(),

            cursor_icon: CursorIcon::default(),
            needs_redraw: true,

            root_widget, theme,
            popup_widgets: PopupMap::new(),

            set_cursor_pos: None,
            set_cursor_icon: None,
        }
    }

}

impl<T> Into<Option<T>> for LoopFlow<T> {
    #[inline]
    fn into(self) -> Option<T> {
        match self {
            LoopFlow::Continue => None,
            LoopFlow::Break(t) => Some(t)
        }
    }
}
