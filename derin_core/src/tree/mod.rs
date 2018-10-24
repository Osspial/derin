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

use crate::dynamic::ParentDyn;
use std::sync::Arc;
use std::cell::Cell;

use crate::LoopFlow;
use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::BoundBox};

use crate::mbseq::MouseButtonSequence;
use derin_common_types::buttons::MouseButton;
use derin_common_types::layout::SizeBounds;
use crate::event::{WidgetEvent, EventOps, InputState};
use crate::render::{RenderFrame, FrameRectStack};
use crate::timer::TimerRegister;
use crate::popup::ChildPopupsMut;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetIdent {
    Str(Arc<str>),
    Num(u32),
    StrCollection(Arc<str>, u32),
    NumCollection(u32, u32)
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Update {
    pub render_self: bool,
    pub update_child: bool,
    pub update_timer: bool,
    pub update_layout: bool,
    pub update_layout_post: bool
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

#[derive(Debug, Clone)]
pub struct WidgetTag {
    last_root: Cell<RootID>,
    widget_tag: Cell<UpdateTag>,
    pub(crate) widget_id: WidgetID,
    pub(crate) last_event_stamp: Cell<u32>,
    pub(crate) mouse_state: Cell<MouseState>,
    pub(crate) has_keyboard_focus: Cell<bool>,
    pub(crate) child_event_recv: Cell<ChildEventRecv>
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

impl<'a> From<&'a WidgetTag> for ChildEventRecv {
    #[inline]
    fn from(widget_tag: &'a WidgetTag) -> ChildEventRecv {
        let widget_mb_flags = ChildEventRecv::from(widget_tag.mouse_state.get().mouse_button_sequence());

        widget_mb_flags |
        match widget_tag.mouse_state.get() {
            MouseState::Hovering(_, _) => ChildEventRecv::MOUSE_HOVER,
            MouseState::Tracking(_, _)  |
            MouseState::Untracked   => ChildEventRecv::empty()
        } |
        match widget_tag.has_keyboard_focus.get() {
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
    fn rect(&self) -> BoundBox<D2, i32>;
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32>;
    fn render(&mut self, frame: &mut FrameRectStack<'_, F>);
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: InputState<'_>,
        popups: Option<ChildPopupsMut<'_, A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F>;

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::default()
    }
    fn register_timers(&self, _register: &mut TimerRegister<'_>) {}
    fn accepts_focus(&self) -> OnFocus {
        OnFocus::default()
    }

    #[doc(hidden)]
    fn as_parent(&self) -> Option<&dyn ParentDyn<A, F>> {
        ParentDyn::from_widget(self)
    }

    #[doc(hidden)]
    fn as_parent_mut(&mut self) -> Option<&mut dyn ParentDyn<A, F>> {
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
    fn rect(&self) -> BoundBox<D2, i32> {
        W::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        W::rect_mut(self)
    }
    fn render(&mut self, frame: &mut FrameRectStack<'_, F>) {
        W::render(self, frame)
    }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: InputState<'_>,
        popups: Option<ChildPopupsMut<'_, A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F> {
        W::on_widget_event(self, event, input_state, popups, source_child)
    }

    fn size_bounds(&self) -> SizeBounds {
        W::size_bounds(self)
    }
    fn register_timers(&self, register: &mut TimerRegister<'_>) {
        W::register_timers(self, register)
    }
    fn accepts_focus(&self) -> OnFocus {
        W::accepts_focus(self)
    }

    #[doc(hidden)]
    fn as_parent(&self) -> Option<&dyn ParentDyn<A, F>> {
        W::as_parent(self)
    }

    #[doc(hidden)]
    fn as_parent_mut(&mut self) -> Option<&mut dyn ParentDyn<A, F>> {
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
    fn rect(&self) -> BoundBox<D2, i32> {
        W::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        W::rect_mut(self)
    }
    fn render(&mut self, frame: &mut FrameRectStack<'_, F>) {
        W::render(self, frame)
    }
    fn on_widget_event(
        &mut self,
        event: WidgetEvent,
        input_state: InputState<'_>,
        popups: Option<ChildPopupsMut<'_, A, F>>,
        source_child: &[WidgetIdent]
    ) -> EventOps<A, F> {
        W::on_widget_event(self, event, input_state, popups, source_child)
    }

    fn size_bounds(&self) -> SizeBounds {
        W::size_bounds(self)
    }
    fn register_timers(&self, register: &mut TimerRegister<'_>) {
        W::register_timers(self, register)
    }
    fn accepts_focus(&self) -> OnFocus {
        W::accepts_focus(self)
    }

    #[doc(hidden)]
    fn as_parent(&self) -> Option<&dyn ParentDyn<A, F>> {
        W::as_parent(self)
    }

    #[doc(hidden)]
    fn as_parent_mut(&mut self) -> Option<&mut dyn ParentDyn<A, F>> {
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

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&dyn Widget<A, F>>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut dyn Widget<A, F>>>;

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&dyn Widget<A, F>>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut dyn Widget<A, F>>>;

    fn children<'a, G, R>(&'a self, for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a dyn Widget<A, F>>) -> LoopFlow<R>;
    fn children_mut<'a, G, R>(&'a mut self, for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut dyn Widget<A, F>>) -> LoopFlow<R>;

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

    pub fn to_dyn<A, F>(self) -> WidgetSummary<&'a dyn Widget<A, F>>
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

    pub fn to_dyn_mut<A, F>(self) -> WidgetSummary<&'a mut dyn Widget<A, F>>
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


bitflags!{
    pub(crate) struct UpdateTag: u8 {
        const RENDER_SELF = 1 << 0;
        const UPDATE_CHILD = 1 << 1;
        const UPDATE_LAYOUT = 1 << 2;
        const UPDATE_LAYOUT_POST = 1 << 3;
        const UPDATE_TIMER = 1 << 4;
        const RENDER_ALL = UpdateTag::RENDER_SELF.bits | UpdateTag::UPDATE_CHILD.bits;
    }
}

impl WidgetTag {
    #[inline]
    pub fn new() -> WidgetTag {
        WidgetTag {
            last_root: Cell::new(RootID::dummy()),
            widget_tag: Cell::new(UpdateTag::all()),
            widget_id: WidgetID::new(),
            last_event_stamp: Cell::new(0),
            mouse_state: Cell::new(MouseState::Untracked),
            has_keyboard_focus: Cell::new(false),
            child_event_recv: Cell::new(ChildEventRecv::empty())
        }
    }

    #[inline]
    pub fn mark_render_self(&mut self) -> &mut WidgetTag {
        self.widget_tag.set(self.widget_tag.get() | UpdateTag::RENDER_SELF);
        self
    }

    #[inline]
    pub fn mark_update_child(&mut self) -> &mut WidgetTag {
        self.widget_tag.set(self.widget_tag.get() | UpdateTag::UPDATE_CHILD);
        self
    }

    #[inline]
    pub fn mark_update_layout(&mut self) -> &mut WidgetTag {
        self.widget_tag.set(self.widget_tag.get() | UpdateTag::UPDATE_LAYOUT);
        self
    }


    #[inline]
    pub fn mark_update_layout_post(&mut self) -> &mut WidgetTag {
        self.widget_tag.set(self.widget_tag.get() | UpdateTag::UPDATE_LAYOUT_POST);
        self
    }

    #[inline]
    pub fn mark_update_timer(&mut self) -> &mut WidgetTag {
        self.widget_tag.set(self.widget_tag.get() | UpdateTag::UPDATE_TIMER);
        self
    }

    #[inline]
    pub fn has_keyboard_focus(&self) -> bool {
        self.has_keyboard_focus.get()
    }

    #[inline]
    pub(crate) fn mark_updated(&self, root_id: RootID) {
        self.last_root.set(root_id);
        self.widget_tag.set(UpdateTag::empty());
    }

    #[inline]
    pub(crate) fn unmark_update_layout(&self) {
        self.widget_tag.set(self.widget_tag.get() & !(UpdateTag::UPDATE_LAYOUT | UpdateTag::UPDATE_LAYOUT_POST));
    }

    #[inline]
    pub(crate) fn unmark_update_timer(&self) {
        self.widget_tag.set(self.widget_tag.get() & !UpdateTag::UPDATE_TIMER);
    }

    #[inline]
    pub(crate) fn mark_update_child_immutable(&self) {
        self.widget_tag.set(self.widget_tag.get() | UpdateTag::UPDATE_CHILD);
    }

    #[inline]
    pub(crate) fn needs_update(&self, root_id: RootID) -> Update {
        if root_id != self.last_root.get() {
            Update {
                render_self: true,
                update_child: true,
                update_timer: true,
                update_layout: true,
                update_layout_post: true
            }
        } else {
            let widget_tag = self.widget_tag.get();
            Update {
                render_self: widget_tag.contains(UpdateTag::RENDER_SELF),
                update_child: widget_tag.contains(UpdateTag::UPDATE_CHILD),
                update_timer: widget_tag.contains(UpdateTag::UPDATE_TIMER),
                update_layout: widget_tag.contains(UpdateTag::UPDATE_LAYOUT),
                update_layout_post: widget_tag.contains(UpdateTag::UPDATE_LAYOUT_POST),
            }
        }
    }
}

impl Update {
    pub fn needs_redraw(self) -> bool {
        let Update {
            render_self,
            update_child,
            update_timer: _,
            update_layout,
            update_layout_post
        } = self;

        render_self || update_child || update_layout || update_layout_post
    }
}
