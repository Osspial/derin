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

use self::dynamic::ParentDyn;
use crate::{
    LoopFlow,
    event::{WidgetEventSourced, EventOps, InputState},
    mbseq::MouseButtonSequence,
    render::{RenderFrame, RenderFrameClipped},
    timer::{TimerID, Timer},
    update_state::{UpdateStateShared, UpdateStateCell}
};
use derin_common_types::{
    cursor::CursorIcon,
    layout::SizeBounds,
};
use std::{
    cell::{Cell, RefCell},
    fmt,
    ops::Drop,
    rc::Rc,
    sync::Arc,
};
use cgmath_geometry::{
    D2, rect::BoundBox,
    cgmath::Point2,
};
use fnv::FnvHashMap;


pub(crate) const ROOT_IDENT: WidgetIdent = WidgetIdent::Num(0);
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

pub struct WidgetTag {
    update_state: RefCell<UpdateStateShared>,
    pub(crate) widget_id: WidgetID,
    pub(crate) timers: FnvHashMap<TimerID, Timer>,
    pub(crate) mouse_state: Cell<MouseState>,
}

impl fmt::Debug for WidgetTag {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_tuple("WidgetTag")
            .field(&self.widget_id)
            .finish()
    }
}

impl Clone for WidgetTag {
    /// This doesn't actually clone the `WidgetTag` - it just creates a new one and returns it. This
    /// function is provided primarily to allow widgets to cleanly derive `Clone`.
    fn clone(&self) -> WidgetTag {
        WidgetTag::new()
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

id!(pub WidgetID);


pub trait Widget<A, F: RenderFrame> {
    fn widget_tag(&self) -> &WidgetTag;
    fn rect(&self) -> BoundBox<D2, i32>;
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32>;
    fn render(&mut self, frame: &mut RenderFrameClipped<F>);
    fn on_widget_event(
        &mut self,
        event: WidgetEventSourced<'_>,
        input_state: InputState,
    ) -> EventOps<A>;

    fn update_layout(&mut self, _theme: &F::Theme) {}
    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::default()
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
    fn rect(&self) -> BoundBox<D2, i32> {
        W::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        W::rect_mut(self)
    }
    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        W::render(self, frame)
    }
    fn on_widget_event(
        &mut self,
        event: WidgetEventSourced<'_>,
        input_state: InputState,
    ) -> EventOps<A> {
        W::on_widget_event(self, event, input_state)
    }

    fn update_layout(&mut self, theme: &F::Theme) {
        W::update_layout(self, theme)
    }
    fn size_bounds(&self) -> SizeBounds {
        W::size_bounds(self)
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
    fn rect(&self) -> BoundBox<D2, i32> {
        W::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        W::rect_mut(self)
    }
    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        W::render(self, frame)
    }
    fn on_widget_event(
        &mut self,
        event: WidgetEventSourced<'_>,
        input_state: InputState,
    ) -> EventOps<A> {
        W::on_widget_event(self, event, input_state)
    }

    fn update_layout(&mut self, theme: &F::Theme) {
        W::update_layout(self, theme)
    }
    fn size_bounds(&self) -> SizeBounds {
        W::size_bounds(self)
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

    fn children<'a, G>(&'a self, for_each: G)
        where A: 'a,
              G: FnMut(WidgetSummary<&'a Widget<A, F>>) -> LoopFlow;
    fn children_mut<'a, G>(&'a mut self, for_each: G)
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<A, F>>) -> LoopFlow;
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


impl WidgetIdent {
    pub fn new_str(s: &str) -> WidgetIdent {
        WidgetIdent::Str(Arc::from(s))
    }

    pub fn new_str_collection(s: &str, i: u32) -> WidgetIdent {
        WidgetIdent::StrCollection(Arc::from(s), i)
    }
}

impl WidgetTag {
    #[inline]
    pub fn new() -> WidgetTag {
        WidgetTag {
            update_state: RefCell::new(UpdateStateShared::new()),
            widget_id: WidgetID::new(),
            timers: FnvHashMap::default(),
            mouse_state: Cell::new(MouseState::Untracked),
        }
    }

    #[inline]
    pub fn request_redraw(&mut self) -> &mut WidgetTag {
        self.update_state.get_mut().request_redraw(self.widget_id);
        self
    }

    #[inline]
    pub fn request_relayout(&mut self) -> &mut WidgetTag {
        self.update_state.get_mut().request_relayout(self.widget_id);
        self
    }

    pub fn timers(&self) -> &FnvHashMap<TimerID, Timer> {
        &self.timers
    }

    pub fn timers_mut(&mut self) -> &mut FnvHashMap<TimerID, Timer> {
        self.update_state.get_mut().request_update_timers(self.widget_id);
        &mut self.timers
    }

    pub fn set_cursor_pos(&mut self, cursor_pos: Point2<i32>) {
        unimplemented!()
    }

    pub fn set_cursor_icon(&mut self, cursor_icon: CursorIcon) {
        unimplemented!()
    }

    #[inline]
    pub fn has_keyboard_focus(&self) -> bool {
        unimplemented!()
    }

    #[inline]
    pub(crate) fn set_owning_update_state(&self, state: &Rc<UpdateStateCell>) {
        self.update_state.borrow_mut().set_owning_update_state(self.widget_id, state);
    }
}

impl Drop for WidgetTag {
    fn drop(&mut self) {
        self.update_state.get_mut().remove_from_tree(self.widget_id)
    }
}
