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

#![feature(conservative_impl_trait, universal_impl_trait, range_contains, nll, specialization)]

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
mod meta_tracker;
mod offset_widget;
mod event_loop_ops;

use cgmath::{Point2, Vector2, Bounded};
use cgmath_geometry::DimsBox;

use std::marker::PhantomData;
use std::collections::VecDeque;

use tree::*;
pub use event_loop_ops::{EventLoopOps, EventLoopResult, PopupDelta};
use timer::TimerList;
use event::WidgetEvent;
use popup::{PopupID, PopupMap};
use render::{Renderer, RenderFrame};
use mbseq::MouseButtonSequenceTrackPos;
use widget_stack::WidgetStackBase;
use meta_tracker::MetaEventTracker;
use derin_common_types::buttons::{MouseButton, Key, ModifierKeys};
use derin_common_types::cursor::CursorIcon;

pub struct Root<A, N, F>
    where N: Widget<A, F> + 'static,
          A: 'static,
          F: RenderFrame + 'static
{
    id: RootID,
    mouse_pos: Point2<i32>,
    modifiers: ModifierKeys,
    cursor_icon: CursorIcon,
    mouse_buttons_down: MouseButtonSequenceTrackPos,

    actions: VecDeque<A>,
    widget_stack_base: WidgetStackBase<A, F>,
    force_full_redraw: bool,
    event_stamp: u32,
    widget_ident_stack: Vec<WidgetIdent>,
    meta_tracker: MetaEventTracker,
    timer_list: TimerList,
    pub root_widget: N,
    pub theme: F::Theme,
    popup_widgets: PopupMap<A, F>,
    _marker: PhantomData<*const F>
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
    WindowResize(DimsBox<Point2<u32>>),
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
    pub fn new(mut root_widget: N, theme: F::Theme, dims: DimsBox<Point2<u32>>) -> Root<A, N, F> {
        // TODO: DRAW ROOT AND DO INITIAL LAYOUT
        *root_widget.rect_mut() = dims.cast().unwrap_or(DimsBox::max_value()).into();
        Root {
            id: RootID::new(),
            mouse_pos: Point2::new(-1, -1),
            mouse_buttons_down: MouseButtonSequenceTrackPos::new(),
            modifiers: ModifierKeys::empty(),
            cursor_icon: CursorIcon::default(),
            actions: VecDeque::new(),
            widget_stack_base: WidgetStackBase::new(),
            force_full_redraw: false,
            event_stamp: 1,
            widget_ident_stack: Vec::new(),
            meta_tracker: MetaEventTracker::default(),
            timer_list: TimerList::new(None),
            root_widget, theme,
            popup_widgets: PopupMap::new(),
            _marker: PhantomData
        }
    }

    pub fn run_forever<R, G>(
        &mut self,
        mut gen_events: impl FnMut(&mut EventLoopOps<A, N, F, R, G>) -> Option<G>,
        mut on_action: impl FnMut(A, &mut N, &mut F::Theme) -> LoopFlow<G>,
        mut bubble_fallthrough: impl FnMut(WidgetEvent, &[WidgetIdent]) -> Option<A>,
        mut with_renderer: impl FnMut(Option<PopupID>, &mut FnMut(&mut R))
    ) -> Option<G>
        where R: Renderer<Frame=F>
    {
        gen_events(&mut EventLoopOps {
            root: self,
            on_action: &mut on_action,
            bubble_fallthrough: &mut bubble_fallthrough,
            with_renderer: &mut with_renderer
        })
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
