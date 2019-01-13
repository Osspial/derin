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
use crate::core::event::{EventOps, WidgetEvent, InputState, WidgetEventSourced};
use crate::core::tree::{WidgetIdent, WidgetTag, WidgetSummary, Widget, Parent};
use crate::core::render::RenderFrameClipped;

use crate::cgmath::{Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};
use derin_common_types::layout::SizeBounds;
use derin_common_types::buttons::MouseButton;

use crate::gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};
use crate::widgets::Clip;
use crate::widgets::assistants::SliderAssist;

use arrayvec::ArrayVec;

use std::f32;
use std::sync::Arc;

const SCROLL_BAR_SIZE: i32 = 16;

/// A widget that is used to apply scroll bars to a contained widget.
///
/// These bars are hidden by default, only appearing when the inner widget's minimum size is
/// greater than the scroll box's size.
#[derive(Debug, Clone)]
pub struct ScrollBox<W> {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
    slider_x: Option<SliderAssist>,
    slider_y: Option<SliderAssist>,
    clip: Clip<W>
}

impl<W> ScrollBox<W> {
    /// Creates a `ScrollBox` that scrolls the provided widget.
    pub fn new(widget: W) -> ScrollBox<W> {
        ScrollBox {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            clip: Clip::new(widget),
            slider_x: None,
            slider_y: None
        }
    }

    /// Retrieves the scrollable widget.
    pub fn widget(&self) -> &W {
        self.clip.widget()
    }

    /// Retrieves the scrollable widget, for mutation.
    pub fn widget_mut(&mut self) -> &mut W {
        self.clip.widget_mut()
    }

    fn child_summary<F>(&self) -> WidgetSummary<&Widget<F>>
        where W: Widget<F>,
              F: PrimFrame
    {
        WidgetSummary::new(CLIP_IDENT.clone(), 0, &self.clip as &Widget<F>)
    }

    fn child_summary_mut<F>(&mut self) -> WidgetSummary<&mut Widget<F>>
        where W: Widget<F>,
              F: PrimFrame
    {
        WidgetSummary::new_mut(CLIP_IDENT.clone(), 0, &mut self.clip as &mut Widget<F>)
    }
}

impl<F, W> Widget<F> for ScrollBox<W>
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
    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::default()
    }

    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        let mut primitives: ArrayVec<[_; 4]> = ArrayVec::new();

        if let Some(slider_x) = self.slider_x.clone() {
            primitives.push(ThemedPrim {
                theme_path: "ScrollBackground",
                min: Point2::new(
                    RelPoint::new(-1.0, slider_x.bar_rect.min.x),
                    RelPoint::new(-1.0, slider_x.bar_rect.min.y),
                ),
                max: Point2::new(
                    RelPoint::new(-1.0, slider_x.bar_rect.max.x),
                    RelPoint::new(-1.0, slider_x.bar_rect.max.y)
                ),
                prim: Prim::Image,
                rect_px_out: None
            });

            let head_rect = slider_x.head_rect();

            primitives.push(ThemedPrim {
                theme_path: "ScrollBar",
                min: Point2::new(
                    RelPoint::new(-1.0, head_rect.min.x),
                    RelPoint::new(-1.0, head_rect.min.y),
                ),
                max: Point2::new(
                    RelPoint::new(-1.0, head_rect.max.x),
                    RelPoint::new(-1.0, head_rect.max.y)
                ),
                prim: Prim::Image,
                rect_px_out: None
            });
        }
        if let Some(slider_y) = self.slider_y.clone() {
            primitives.push(ThemedPrim {
                theme_path: "ScrollBackground",
                min: Point2::new(
                    RelPoint::new(-1.0, slider_y.bar_rect.min.x),
                    RelPoint::new(-1.0, slider_y.bar_rect.min.y),
                ),
                max: Point2::new(
                    RelPoint::new(-1.0, slider_y.bar_rect.max.x),
                    RelPoint::new(-1.0, slider_y.bar_rect.max.y)
                ),
                prim: Prim::Image,
                rect_px_out: None
            });

            let head_rect = slider_y.head_rect();

            primitives.push(ThemedPrim {
                theme_path: "ScrollBar",
                min: Point2::new(
                    RelPoint::new(-1.0, head_rect.min.x),
                    RelPoint::new(-1.0, head_rect.min.y),
                ),
                max: Point2::new(
                    RelPoint::new(-1.0, head_rect.max.x),
                    RelPoint::new(-1.0, head_rect.max.y)
                ),
                prim: Prim::Image,
                rect_px_out: None
            });
        }

        frame.upload_primitives(primitives.into_iter());
    }

    fn update_layout(&mut self, _: &F::Theme) {
        let child_size_bounds = self.clip.widget().size_bounds();
        let mut child_dims: DimsBox<D2, _> = self.rect.dims();
        let mut offset = Vector2 {
            x: self.slider_x.as_ref().map(|s| s.value as i32).unwrap_or(0),
            y: self.slider_y.as_ref().map(|s| s.value as i32).unwrap_or(0)
        };

        let (mut has_x_scroll, mut has_y_scroll) = (false, false);
        for _ in 0..2 {
            let scroll_dims_x = child_dims.dims.x - SCROLL_BAR_SIZE * has_y_scroll as i32;
            let scroll_dims_y = child_dims.dims.y - SCROLL_BAR_SIZE * has_x_scroll as i32;
            child_dims.dims.x = scroll_dims_x.max(child_size_bounds.min.width());
            child_dims.dims.y = scroll_dims_y.max(child_size_bounds.min.height());

            has_x_scroll |= child_dims.width() != scroll_dims_x;
            has_y_scroll |= child_dims.height() != scroll_dims_y;
        }

        let clip_dims = DimsBox::new2(
            self.rect.width() - SCROLL_BAR_SIZE * has_y_scroll as i32,
            self.rect.height() - SCROLL_BAR_SIZE * has_x_scroll as i32,
        );

        offset.x = offset.x.min((child_dims.width() as u32).saturating_sub(clip_dims.width() as u32) as i32);
        offset.y = offset.y.min((child_dims.height() as u32).saturating_sub(clip_dims.height() as u32) as i32);

        let self_dims: DimsBox<D2, _> = self.rect.dims();
        self.slider_x = match has_x_scroll {
            false => None,
            true => Some(SliderAssist {
                value: offset.x as f32,
                step: f32::EPSILON,
                min: 0.0,
                max: (child_dims.width() - clip_dims.width()) as f32,

                head_size: 16.max(clip_dims.width().pow(2) / child_dims.width()), // TODO: PROPER HEIGHT CALCULATION
                bar_rect: BoundBox::new2(
                    0, self_dims.height() - SCROLL_BAR_SIZE,
                    clip_dims.width(), self_dims.height()
                ),
                head_click_pos: self.slider_x.as_ref().and_then(|s| s.head_click_pos),
                horizontal: true
            })
        };
        self.slider_y = match has_y_scroll {
            false => None,
            true => Some(SliderAssist {
                value: offset.y as f32,
                step: f32::EPSILON,
                min: 0.0,
                max: (child_dims.height() - clip_dims.height()) as f32,

                head_size: 16.max(clip_dims.height().pow(2) / child_dims.height()),
                bar_rect: BoundBox::new2(
                    self_dims.width() - SCROLL_BAR_SIZE, 0,
                    self_dims.width(), clip_dims.height()
                ),
                head_click_pos: self.slider_y.as_ref().and_then(|s| s.head_click_pos),
                horizontal: false
            })
        };

        *self.clip.rect_mut() = BoundBox::from(clip_dims);
        *self.clip.widget_mut().rect_mut() = BoundBox::from(child_dims) - offset;
    }
    #[inline]
    fn on_widget_event(&mut self, event: WidgetEventSourced, _: InputState) -> EventOps {
        // TODO: PASS FOCUS TO CHILD
        let values = |slider_x: &Option<SliderAssist>, slider_y: &Option<SliderAssist>|
            (slider_x.as_ref().map(|s| s.value), slider_y.as_ref().map(|s| s.value));
        let start_values = values(&self.slider_x, &self.slider_y);
        let mut allow_bubble = true;

        match event {
            WidgetEventSourced::This(ref event) => match event {
                WidgetEvent::MouseDown{pos, in_widget: true, button: MouseButton::Left} => {
                    if let Some(ref mut slider_x) = self.slider_x {
                        slider_x.click_head(*pos);
                    }
                    if let Some(ref mut slider_y) = self.slider_y {
                        slider_y.click_head(*pos);
                    }
                    self.widget_tag.request_redraw();
                },
                WidgetEvent::MouseMove{new_pos, ..} => {
                    if let Some(ref mut slider_x) = self.slider_x {
                        slider_x.move_head(new_pos.x);
                    }
                    if let Some(ref mut slider_y) = self.slider_y {
                        slider_y.move_head(new_pos.y);
                    }
                },
                WidgetEvent::MouseUp{button: MouseButton::Left, ..} => {
                    if let Some(ref mut slider_x) = self.slider_x {
                        slider_x.head_click_pos = None;
                    }
                    if let Some(ref mut slider_y) = self.slider_y {
                        slider_y.head_click_pos = None;
                    }
                    self.widget_tag.request_redraw();
                },
                _ => ()
            },
            WidgetEventSourced::Bubble(ref event, _) => match event {
                WidgetEvent::MouseScrollLines(dir) => {
                    allow_bubble = false;
                    if let Some(ref mut slider_x) = self.slider_x {
                        slider_x.value -= (24 * dir.x) as f32;
                        slider_x.round_to_step();
                    }
                    if let Some(ref mut slider_y) = self.slider_y {
                        slider_y.value -= (24 * dir.y) as f32;
                        slider_y.round_to_step();
                    }
                },
                WidgetEvent::MouseScrollPx(dir) => {
                    allow_bubble = false;
                    if let Some(ref mut slider_x) = self.slider_x {
                        slider_x.value -= dir.x as f32;
                        slider_x.round_to_step();
                    }
                    if let Some(ref mut slider_y) = self.slider_y {
                        slider_y.value -= dir.y as f32;
                        slider_y.round_to_step();
                    }
                },
                _ => ()
            }
        }

        if values(&self.slider_x, &self.slider_y) != start_values {
            self.widget_tag.request_redraw().request_relayout();
        }
        EventOps {
            focus: None,
            bubble: allow_bubble && event.default_bubble(),
        }
    }
}

lazy_static!{
    static ref CLIP_IDENT: WidgetIdent = WidgetIdent::Str(Arc::from("clip"));
}

impl<F, W> Parent<F> for ScrollBox<W>
    where F: PrimFrame,
          W: Widget<F>
{
    fn num_children(&self) -> usize {
        1
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<F>>> {
        match widget_ident {
            _ if widget_ident == *CLIP_IDENT => Some(self.child_summary()),
            _ => None
        }
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<F>>> {
        match widget_ident {
            _ if widget_ident == *CLIP_IDENT => Some(self.child_summary_mut()),
            _ => None
        }
    }

    fn children<'a, G>(&'a self, mut for_each: G)
        where G: FnMut(WidgetSummary<&'a Widget<F>>) -> LoopFlow
    {
        for_each(self.child_summary());
    }

    fn children_mut<'a, G>(&'a mut self, mut for_each: G)
        where G: FnMut(WidgetSummary<&'a mut Widget<F>>) -> LoopFlow
    {
        for_each(self.child_summary_mut());
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<F>>> {
        match index {
            0 => Some(self.child_summary()),
            _ => None
        }
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<F>>> {
        match index {
            0 => Some(self.child_summary_mut()),
            _ => None
        }
    }
}
