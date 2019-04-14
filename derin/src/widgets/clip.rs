// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    LoopFlow,
    event::{EventOps, WidgetEventSourced, InputState},
    widget::{WidgetIdent, WidgetRenderable, WidgetTag, WidgetInfo, WidgetInfoMut, Widget, Parent},
    render::{DisplayEngine, SubFrame},
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
    widget: W,
}

#[derive(Debug, Clone, Default)]
pub struct ClipTheme(());

impl<W> Clip<W> {
    /// Creates a new clip widget.
    pub fn new(widget: W) -> Clip<W> {
        Clip {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            widget,
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

impl<W> Widget for Clip<W>
    where W: Widget
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

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        // TODO: PASS FOCUS THROUGH SELF
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl<W> Parent for Clip<W>
    where W: Widget
{
    fn num_children(&self) -> usize {
        1
    }

    fn framed_child<R: Renderer>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, R>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.widget)),
            _ => None
        }
    }
    fn framed_child_mut<R: Renderer>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, R>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.widget)),
            _ => None
        }
    }

    fn framed_children<'a, R, G>(&'a self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfo<'a, R>) -> LoopFlow
    {
        let _ = for_each(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.widget));
    }

    fn framed_children_mut<'a, R, G>(&'a mut self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfoMut<'a, R>) -> LoopFlow
    {
        let _ = for_each(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.widget));
    }

    fn framed_child_by_index<R: Renderer>(&self, index: usize) -> Option<WidgetInfo<'_, R>> {
        match index {
            0 => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.widget)),
            _ => None
        }
    }
    fn framed_child_by_index_mut<R: Renderer>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, R>> {
        match index {
            0 => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.widget)),
            _ => None
        }
    }
}

impl<W, R> WidgetRenderable<R> for Clip<W>
    where W: Widget,
          R: Renderer
{
    type Theme = ClipTheme;

    fn theme(&self) -> ClipTheme {
        ClipTheme(())
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, _: &mut R::Layout) {
        let widget_rect = self.widget.rect();
        let size_bounds = self.widget.size_bounds();

        let dims_clipped = size_bounds.bound_rect(widget_rect.dims());
        if dims_clipped.dims() != widget_rect.dims() {
            *self.widget.rect_mut() = BoundBox::from(dims_clipped) + widget_rect.min().to_vec();
        }
    }
}

impl WidgetTheme for ClipTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {
        None
    }
}
