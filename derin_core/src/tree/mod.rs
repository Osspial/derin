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

pub(crate) mod dynamic;

use dynamic::ParentDyn;
use std::sync::Arc;
use std::cell::Cell;

use LoopFlow;
use cgmath::Point2;
use cgmath_geometry::BoundBox;

use mbseq::MouseButtonSequence;
use derin_common_types::buttons::MouseButton;
use derin_common_types::layout::SizeBounds;
use event::{WidgetEvent, EventOps, InputState};
use render::{RenderFrame, FrameRectStack};
use timer::TimerRegister;
use popup::ChildPopupsMut;
use bus::BusTerminal;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetIdent {
    Str(Arc<str>),
    Num(u32),
    StrCollection(Arc<str>, u32),
    NumCollection(u32, u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum MouseState {
    /// The mouse is hovering over the given widget.
    Hovering(Point2<i32>, MouseButtonSequence),
    /// The mouse isn't hovering over the widget, but the widget is still receiving mouse events.
    Tracking(Point2<i32>, MouseButtonSequence),
    /// The widget is not aware of the current mouse position and is receiving no events.
    Untracked
}

// #[derive(Debug)]
pub struct WidgetTag {
    update_sender: BusTerminal<UpdateEvent, WidgetID>
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WidgetState {
    widget_id: WidgetID,
    pub last_event_stamp: u32,
    pub mouse_state: MouseState,
    pub has_keyboard_focus: bool,
    pub child_event_recv: ChildEventRecv
}

impl Default for WidgetState {
    fn default() -> WidgetState {
        WidgetState {
            widget_id: WidgetID::new(),
            last_event_stamp: 0,
            mouse_state: MouseState::Untracked,
            has_keyboard_focus: false,
            child_event_recv: ChildEventRecv::empty()
        }
    }
}

impl MouseState {
    #[inline]
    pub fn mouse_button_sequence(&self) -> MouseButtonSequence {
        match *self {
            MouseState::Untracked => MouseButtonSequence::new(),
            MouseState::Hovering(_, mbseq) |
            MouseState::Tracking(_, mbseq) => mbseq
        }
    }
}

impl Clone for WidgetTag {
    /// This implementation doesn't actually clone - it creates a new instance. It is simply
    /// provided to allow deriving `Clone` on widgets using this.
    fn clone(&self) -> WidgetTag {
        WidgetTag::new()
    }
}

bitflags! {
    pub(crate) struct ChildEventRecv: u8 {
        const MOUSE_L        = 1 << 0;
        const MOUSE_R        = 1 << 1;
        const MOUSE_M        = 1 << 2;
        const MOUSE_X1       = 1 << 3;
        const MOUSE_X2       = 1 << 4;
        const MOUSE_HOVER    = 1 << 5;
        const KEYBOARD       = 1 << 6;

        const MOUSE_BUTTONS =
            Self::MOUSE_L.bits  |
            Self::MOUSE_R.bits  |
            Self::MOUSE_M.bits  |
            Self::MOUSE_X1.bits |
            Self::MOUSE_X2.bits;
    }
}

impl ChildEventRecv {
    #[inline]
    pub(crate) fn mouse_button_mask(button: MouseButton) -> ChildEventRecv {
        ChildEventRecv::from_bits_truncate(1 << (u8::from(button) - 1))
    }
}

impl From<MouseButtonSequence> for ChildEventRecv {
    #[inline]
    fn from(mbseq: MouseButtonSequence) -> ChildEventRecv {
        mbseq.into_iter().fold(ChildEventRecv::empty(), |child_event_recv, mb| child_event_recv | ChildEventRecv::mouse_button_mask(mb))
    }
}

impl<'a> From<&'a WidgetState> for ChildEventRecv {
    #[inline]
    fn from(widget_state: &'a WidgetState) -> ChildEventRecv {
        let widget_mb_flags = ChildEventRecv::from(widget_state.mouse_state.mouse_button_sequence());

        widget_mb_flags |
        match widget_state.mouse_state {
            MouseState::Hovering(_, _) => ChildEventRecv::MOUSE_HOVER,
            MouseState::Tracking(_, _)  |
            MouseState::Untracked   => ChildEventRecv::empty()
        } |
        match widget_state.has_keyboard_focus {
            true => ChildEventRecv::KEYBOARD,
            false => ChildEventRecv::empty()
        }
    }
}

macro_rules! id {
    (pub$(($vis:tt))* $Name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub$(($vis))* struct $Name(u32);

        impl $Name {
            #[inline]
            pub(crate) fn new() -> $Name {
                use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

                static ID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
                let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;

                $Name(id as u32)
            }

            #[allow(dead_code)]
            pub(crate) fn dummy() -> $Name {
                $Name(!0)
            }
        }
    }
}

id!(pub(crate) RootID);
id!(pub(crate) WidgetID);


/// Behavior when another widget attempts to focus a given widget.
///
/// Note that this is *ignored* if the attempt to focus came from the return value of this widget's
/// `on_widget_event` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OnFocus {
    /// Accept focus, and send a `GainFocus` event to this widget. Is default.
    Accept,
    /// Don't accept focus, and try to focus the next widget.
    Skip,
    FocusChild
}

/// Configures where to deliver focus when a child send a `FocusChange::Next` or `FocusChange::Prev`,
/// and there is no next/previous widget to deliver focus to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OnFocusOverflow {
    /// Go the the parent widget and attempt to deliver focus to the next widget on the parent's level.
    /// Is default.
    Continue,
    /// Wrap focus around, returning focus to the first/last widget on the current level.
    Wrap
}

pub trait Widget<A, F: RenderFrame> {
    fn widget_tag(&self) -> &WidgetTag;
    fn rect(&self) -> BoundBox<Point2<i32>>;
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>>;
    fn render(&mut self, frame: &mut FrameRectStack<F>);
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: InputState,
        popups: Option<ChildPopupsMut<A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F>;

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::default()
    }
    fn register_timers(&self, _register: &mut TimerRegister) {}
    fn accepts_focus(&self) -> OnFocus {
        OnFocus::default()
    }

    #[doc(hidden)]
    fn as_parent(&self) -> Option<&ParentDyn<A, F>> {
        ParentDyn::from_widget(self)
    }

    #[doc(hidden)]
    fn as_parent_mut(&mut self) -> Option<&mut ParentDyn<A, F>> {
        ParentDyn::from_widget_mut(self)
    }
}

impl<'a, A, F, W> Widget<A, F> for &'a mut W
    where W: Widget<A, F> + ?Sized,
          F: RenderFrame
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        W::widget_tag(self)
    }
    fn rect(&self) -> BoundBox<Point2<i32>> {
        W::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        W::rect_mut(self)
    }
    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        W::render(self, frame)
    }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: InputState,
        popups: Option<ChildPopupsMut<A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F> {
        W::on_widget_event(self, event, input_state, popups, source_child)
    }

    fn size_bounds(&self) -> SizeBounds {
        W::size_bounds(self)
    }
    fn register_timers(&self, register: &mut TimerRegister) {
        W::register_timers(self, register)
    }
    fn accepts_focus(&self) -> OnFocus {
        W::accepts_focus(self)
    }

    #[doc(hidden)]
    fn as_parent(&self) -> Option<&ParentDyn<A, F>> {
        W::as_parent(self)
    }

    #[doc(hidden)]
    fn as_parent_mut(&mut self) -> Option<&mut ParentDyn<A, F>> {
        W::as_parent_mut(self)
    }
}


impl<A, F, W> Widget<A, F> for Box<W>
    where W: Widget<A, F> + ?Sized,
          F: RenderFrame
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        W::widget_tag(self)
    }
    fn rect(&self) -> BoundBox<Point2<i32>> {
        W::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        W::rect_mut(self)
    }
    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        W::render(self, frame)
    }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: InputState,
        popups: Option<ChildPopupsMut<A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F> {
        W::on_widget_event(self, event, input_state, popups, source_child)
    }

    fn size_bounds(&self) -> SizeBounds {
        W::size_bounds(self)
    }
    fn register_timers(&self, register: &mut TimerRegister) {
        W::register_timers(self, register)
    }
    fn accepts_focus(&self) -> OnFocus {
        W::accepts_focus(self)
    }

    #[doc(hidden)]
    fn as_parent(&self) -> Option<&ParentDyn<A, F>> {
        W::as_parent(self)
    }

    #[doc(hidden)]
    fn as_parent_mut(&mut self) -> Option<&mut ParentDyn<A, F>> {
        W::as_parent_mut(self)
    }
}

#[derive(Debug, Clone)]
pub struct WidgetSummary<W: ?Sized> {
    pub ident: WidgetIdent,
    pub index: usize,
    pub widget: W,
}

pub trait Parent<A, F: RenderFrame>: Widget<A, F> {
    fn num_children(&self) -> usize;

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>>;

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<A, F>>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<A, F>>>;

    fn children<'a, G, R>(&'a self, for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a Widget<A, F>>) -> LoopFlow<R>;
    fn children_mut<'a, G, R>(&'a mut self, for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<A, F>>) -> LoopFlow<R>;

    fn update_child_layout(&mut self);

    fn on_child_focus_overflow(&self) -> OnFocusOverflow {
        OnFocusOverflow::default()
    }
}

impl<'a, W: ?Sized> WidgetSummary<&'a W> {
    pub fn new<A, F>(ident: WidgetIdent, index: usize, widget: &'a W) -> WidgetSummary<&'a W>
        where W: Widget<A, F>,
              F: RenderFrame
    {
        WidgetSummary {
            ident,
            index,
            widget
        }
    }

    pub fn to_dyn<A, F>(self) -> WidgetSummary<&'a Widget<A, F>>
        where W: Widget<A, F>,
              F: RenderFrame
    {
        WidgetSummary {
            ident: self.ident,
            index: self.index,
            widget: dynamic::to_widget_object(self.widget)
        }
    }
}

impl<'a, W: ?Sized> WidgetSummary<&'a mut W> {
    pub fn new_mut<A, F>(ident: WidgetIdent, index: usize, widget: &'a mut W) -> WidgetSummary<&'a mut W>
        where W: Widget<A, F>,
              F: RenderFrame
    {
        WidgetSummary {
            ident,
            index,
            widget
        }
    }

    pub fn to_dyn_mut<A, F>(self) -> WidgetSummary<&'a mut Widget<A, F>>
        where W: Widget<A, F>,
              F: RenderFrame
    {
        WidgetSummary {
            ident: self.ident,
            index: self.index,
            widget: dynamic::to_widget_object_mut(self.widget)
        }
    }
}

impl Default for OnFocus {
    #[inline(always)]
    fn default() -> OnFocus {
        OnFocus::Accept
    }
}

impl Default for OnFocusOverflow {
    #[inline(always)]
    fn default() -> OnFocusOverflow {
        OnFocusOverflow::Continue
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateEvent {
    /// Render the widget.
    Render,
    /// Scan children for updates.
    Child,
    /// Run the layout update function.
    Layout,
    /// Run the timer update function.
    Timer,
}

impl WidgetTag {
    #[inline]
    pub fn new() -> WidgetTag {
        WidgetTag {
            update_sender: BusTerminal::new(WidgetID::new())
        }
    }

    #[inline]
    pub fn mark_render_self(&mut self) -> &mut WidgetTag {
        self.update_sender.send(UpdateEvent::Render);
        self
    }

    #[inline]
    pub fn mark_update_child(&mut self) -> &mut WidgetTag {
        self.update_sender.send(UpdateEvent::Child);
        self
    }

    #[inline]
    pub fn mark_update_layout(&mut self) -> &mut WidgetTag {
        self.update_sender.send(UpdateEvent::Layout);
        self
    }


    #[inline]
    pub fn mark_update_timer(&mut self) -> &mut WidgetTag {
        self.update_sender.send(UpdateEvent::Timer);
        self
    }

    #[inline]
    pub fn has_keyboard_focus(&self) -> bool {
        enum Query {
            HasKeyboardFocus(Option<bool>)
        }

        let mut has_keyboard_focus = Query::HasKeyboardFocus(None);
        self.update_sender.ask(&mut has_keyboard_focus);
        let Query::HasKeyboardFocus(has_keyboard_focus) = has_keyboard_focus;
        has_keyboard_focus.unwrap_or(false)
    }

    #[inline(always)]
    pub(crate) fn widget_id(&self) -> WidgetID {
        self.update_sender.id()
    }
}
