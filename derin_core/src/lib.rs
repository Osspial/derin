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
mod virtual_widget_tree;
mod event_translator;
mod update_state;

use crate::cgmath::{Point2, Vector2, Bounded};
use cgmath_geometry::{D2, rect::DimsBox};

use std::{
    collections::VecDeque,
    rc::Rc
};

pub use crate::event_loop_ops::{EventLoopResult, PopupDelta};
use crate::{
    tree::*,
    timer::TimerList,
    popup::PopupMap,
    render::RenderFrame,
    mbseq::MouseButtonSequenceTrackPos,
    widget_stack::WidgetStackBase,
    event_translator::EventTranslator,
    update_state::{UpdateStateBuffered},
    virtual_widget_tree::VirtualWidgetTree
};
use derin_common_types::buttons::{MouseButton, Key, ModifierKeys};
use derin_common_types::cursor::CursorIcon;

const MAX_FRAME_UPDATE_ITERATIONS: usize = 16;

pub struct Root<A, N, F>
    where N: Widget<A, F> + 'static,
          A: 'static,
          F: RenderFrame + 'static
{
    // Event handing and dispatch
    event_translator: EventTranslator<A, F>,

    // Input State
    input_state: InputState,

    // Render State
    render_state: RenderState,

    // User data
    pub root_widget: N,
    pub theme: F::Theme,
    popup_widgets: PopupMap<A, F>,

    // Per-frame information
}

struct InputState {
    mouse_pos: Option<Point2<i32>>,
    mouse_buttons_down: MouseButtonSequenceTrackPos,
    pub modifiers: ModifierKeys,
    keys_down: Vec<Key>,
    mouse_hover_widget: Option<WidgetID>
}

struct RenderState {
    cursor_icon: CursorIcon,
    needs_redraw: bool,
    frame_set_data: FrameSetData
}

struct FrameSetData {
    set_cursor_pos: Option<Point2<i32>>,
    set_cursor_icon: Option<CursorIcon>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    MouseMove(Point2<i32>),
    MouseEnter,
    MouseExit,
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
            event_translator: EventTranslator::new(root_widget.widget_tag().widget_id),

            input_state: InputState {
                mouse_pos: None,
                mouse_buttons_down: MouseButtonSequenceTrackPos::new(),
                modifiers: ModifierKeys::empty(),
                keys_down: Vec::new(),
                mouse_hover_widget: None
            },

            render_state: RenderState {
                cursor_icon: CursorIcon::default(),
                needs_redraw: true,
                frame_set_data: FrameSetData {
                    set_cursor_pos: None,
                    set_cursor_icon: None,
                }
            },

            root_widget, theme,
            popup_widgets: PopupMap::new(),
        }
    }

    pub fn drain_actions(&mut self) -> impl '_ + Iterator<Item=A> + ExactSizeIterator + DoubleEndedIterator {
        self.event_translator.drain_actions()
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
