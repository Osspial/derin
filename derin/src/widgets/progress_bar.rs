// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    LoopFlow,
    widget::{Parent, Widget, WidgetInfo, WidgetInfoMut, WidgetIdent, WidgetTag, WidgetRenderable},
    render::{DisplayEngine, RendererLayout, SubFrame},
};
use derin_common_types::layout::SizeBounds;
use crate::{
    event::{EventOps, WidgetEventSourced, InputState},
};

use crate::cgmath::Point2;
use cgmath_geometry::{D2, Lerp, rect::BoundBox};


#[derive(Debug, Clone)]
pub struct ProgressBar {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
    size_bounds: SizeBounds,
    fill: ProgressBarFill,
    value: f32,
    min: f32,
    max: f32,
}

#[derive(Debug, Clone)]
struct ProgressBarFill {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
}

#[derive(Debug, Clone, Default)]
pub struct ProgressBarTheme(());
#[derive(Debug, Clone, Default)]
pub struct ProgressBarFillTheme(());

impl ProgressBar {
    /// Creates a new progress bar with the given `value`, `step`, `min`, `max`, and action handler.
    pub fn new(value: f32, min: f32, max: f32) -> ProgressBar {
        ProgressBar {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            size_bounds: SizeBounds::default(),
            fill: ProgressBarFill {
                widget_tag: WidgetTag::new(),
                rect: BoundBox::new2(0, 0, 0, 0)
            },
            value,
            min,
            max
        }
    }

    /// Retrieves the value stored in the progress bar.
    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Retrieves the range of possible values the progress bar can contain.
    #[inline]
    pub fn range(&self) -> (f32, f32) {
        (self.min, self.max)
    }

    /// Retrieves the value stored in the progress bar, for mutation.
    ///
    /// Calling this function forces the progress bar to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn value_mut(&mut self) -> &mut f32 {
        self.widget_tag.request_relayout().request_redraw();
        &mut self.value
    }

    /// Retrieves the range of possible values the progress bar can contain, for mutation.
    ///
    /// Calling this function forces the progress bar to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn range_mut(&mut self) -> (&mut f32, &mut f32) {
        self.widget_tag.request_relayout().request_redraw();
        (&mut self.min, &mut self.max)
    }
}

impl Widget for ProgressBar {
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
        &mut self.rect
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl Widget for ProgressBarFill {
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
        &mut self.rect
    }

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl Parent for ProgressBar {
    fn num_children(&self) -> usize {
        1
    }

    fn framed_child<R: Renderer>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, R>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.fill)),
            _ => None
        }
    }
    fn framed_child_mut<R: Renderer>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, R>> {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.fill)),
            _ => None
        }
    }

    fn framed_children<'a, R, G>(&'a self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfo<'a, R>) -> LoopFlow
    {
        let _ = for_each(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.fill));
    }

    fn framed_children_mut<'a, R, G>(&'a mut self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfoMut<'a, R>) -> LoopFlow
    {
        let _ = for_each(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.fill));
    }

    fn framed_child_by_index<R: Renderer>(&self, index: usize) -> Option<WidgetInfo<'_, R>> {
        match index {
            0 => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.fill)),
            _ => None
        }
    }
    fn framed_child_by_index_mut<R: Renderer>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, R>> {
        match index {
            0 => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.fill)),
            _ => None
        }
    }
}

impl<R> WidgetRenderable<R> for ProgressBar
    where R: Renderer
{
    type Theme = ProgressBarTheme;
    fn theme(&self) -> ProgressBarTheme {
        ProgressBarTheme(())
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, layout: &mut R::Layout) {
        let result = layout.finish();
        self.size_bounds = result.size_bounds;

        let lerp_factor = self.value / (self.max-self.min);
        self.fill.rect = BoundBox {
                min: result.content_rect.min,
                max: Point2::new(
                    i32::lerp(result.content_rect.min.x, result.content_rect.max.x, lerp_factor),
                    result.content_rect.max.y,
                ),
            };
    }
}

impl<R> WidgetRenderable<R> for ProgressBarFill
    where R: Renderer
{
    type Theme = ProgressBarFillTheme;
    fn theme(&self) -> ProgressBarFillTheme {
        ProgressBarFillTheme(())
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, _: &mut R::Layout) { }
}

impl WidgetTheme for ProgressBarTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {
        None
    }
}

impl WidgetTheme for ProgressBarFillTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {
        None
    }
}
