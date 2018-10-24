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

use crate::core::LoopFlow;
use crate::core::event::{EventOps, WidgetEvent, InputState};
use crate::core::tree::{WidgetIdent, WidgetTag, WidgetSummary, Widget, Parent, OnFocus};
use crate::core::render::FrameRectStack;
use crate::core::popup::ChildPopupsMut;

use crate::cgmath::{EuclideanSpace, Point2};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use crate::gl_render::PrimFrame;

/// Assistant widget that is used to clip another widget
///
/// Allows a containing widget to ignore the inner widget's size bounds. Currently used in `ScrollBox`.
#[derive(Debug, Clone)]
pub struct Clip<W> {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
    widget: W
}

impl<W> Clip<W> {
    /// Creates a new clip widget.
    pub fn new(widget: W) -> Clip<W> {
        Clip {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            widget
        }
    }

    /// Retrieves the clipped widget.
    pub fn widget(&self) -> &W {
        &self.widget
    }

    /// Retrieves the clipped widget for mutation.
    pub fn widget_mut(&mut self) -> &mut W {
        self.widget_tag.mark_update_child().mark_update_layout();
        &mut self.widget
    }
}

impl<A, F, W> Widget<A, F> for Clip<W>
    where F: PrimFrame,
          W: Widget<A, F>
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        self.widget_tag.mark_update_layout();
        &mut self.rect
    }

    fn render(&mut self, _: &mut FrameRectStack<F>) {}

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        EventOps {
            action: None,
            focus: None,
            bubble: true,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    fn accepts_focus(&self) -> OnFocus {
        OnFocus::FocusChild
    }
}

impl<A, F, W> Parent<A, F> for Clip<W>
    where F: PrimFrame,
          W: Widget<A, F>
{
    fn num_children(&self) -> usize {
        1
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget)),
            _ => None
        }
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget)),
            _ => None
        }
    }

    fn children<'a, G, R>(&'a self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a Widget<A, F>>) -> LoopFlow<R>
    {
        let flow = for_each(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget));

        match flow {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<A, F>>) -> LoopFlow<R>
    {
        let flow = for_each(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget));

        match flow {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<A, F>>> {
        match index {
            0 => Some(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget)),
            _ => None
        }
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        match index {
            0 => Some(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget)),
            _ => None
        }
    }

    fn update_child_layout(&mut self) {
        let widget_rect = self.widget.rect();
        let size_bounds = self.widget.size_bounds();

        let dims_clipped = size_bounds.bound_rect(widget_rect.dims());
        if dims_clipped.dims() != widget_rect.dims() {
            *self.widget.rect_mut() = BoundBox::from(dims_clipped) + widget_rect.min().to_vec();
        }
    }
}
