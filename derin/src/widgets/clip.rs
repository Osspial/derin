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

use crate::{
    core::{
        LoopFlow,
        event::{EventOps, WidgetEventSourced, InputState},
        widget::{WidgetIdent, WidgetTag, WidgetSummary, Widget, Parent},
        render::RenderFrameClipped,
    },
    gl_render::PrimFrame,
};

use crate::cgmath::EuclideanSpace;
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

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
        &mut self.widget
    }
}

impl<F, W> Widget<F> for Clip<W>
    where F: PrimFrame,
          W: Widget<F>
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
        self.widget_tag.request_relayout();
        &mut self.rect
    }

    fn render(&mut self, _: &mut RenderFrameClipped<F>) {}

    fn update_layout(&mut self, _: &F::Theme) {
        let widget_rect = self.widget.rect();
        let size_bounds = self.widget.size_bounds();

        let dims_clipped = size_bounds.bound_rect(widget_rect.dims());
        if dims_clipped.dims() != widget_rect.dims() {
            *self.widget.rect_mut() = BoundBox::from(dims_clipped) + widget_rect.min().to_vec();
        }
    }

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        // TODO: PASS FOCUS THROUGH SELF
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl<F, W> Parent<F> for Clip<W>
    where F: PrimFrame,
          W: Widget<F>
{
    fn num_children(&self) -> usize {
        1
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<F>>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget)),
            _ => None
        }
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<F>>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget)),
            _ => None
        }
    }

    fn children<'a, G>(&'a self, mut for_each: G)
        where G: FnMut(WidgetSummary<&'a Widget<F>>) -> LoopFlow
    {
        let _ = for_each(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget));
    }

    fn children_mut<'a, G>(&'a mut self, mut for_each: G)
        where G: FnMut(WidgetSummary<&'a mut Widget<F>>) -> LoopFlow
    {
        let _ = for_each(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget));
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<F>>> {
        match index {
            0 => Some(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget)),
            _ => None
        }
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<F>>> {
        match index {
            0 => Some(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget)),
            _ => None
        }
    }
}
