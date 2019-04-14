// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ops::RangeInclusive;
use derin_core::{
    widget::{WidgetTag, WidgetRenderable, Widget},
    render::{DisplayEngine, RendererLayout, SubFrame},
};
use derin_common_types::layout::SizeBounds;
use crate::{
    event::{EventOps, WidgetEvent, InputState, MouseButton, WidgetEventSourced},
};

use cgmath_geometry::{
    Lerp, D2,
    rect::{BoundBox, GeoBox, OffsetBox}
};

pub trait SliderHandler: 'static {
    type Action: 'static;

    fn on_move(&mut self, old_value: f32, new_value: f32) -> Option<Self::Action>;
}

/// A widget that lets the user select a value within a range of values.
///
/// The slider has three values that control the slider's behavior:
/// * `value`: Where the head is, in between the `min` and the `max`.
/// * `step`: Snaps the `value` to a given interval.
/// * `min` and `max`: Controls the minimum and maximum values that can be selected by the slider.
///
/// Whenever the slider's head is moved, the provided handler's [`on_move`] function is called.
///
/// [`on_move`]: ./trait.SliderHandler.html#tymethod.on_move
#[derive(Debug, Clone)]
pub struct Slider<H: SliderHandler> {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
    size_bounds: SizeBounds,

    handle: SliderHandle<H>,
}

#[derive(Debug, Clone, Default)]
pub struct SliderTheme(());
#[derive(Debug, Clone, Default)]
pub struct SliderHandleTheme(());

#[derive(Debug, Clone)]
struct SliderHandle<H: SliderHandler> {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
    size_bounds: SizeBounds,

    value: f32,
    step: f32,
    value_range: RangeInclusive<f32>,

    click_pos: Option<i32>,
    pixel_range: RangeInclusive<i32>,

    handler: H,
}

impl<H: SliderHandler> Slider<H> {
    /// Creates a new slider with the given `value`, `step`, `min`, `max`, and action handler.
    pub fn new(value: f32, step: f32, value_range: RangeInclusive<f32>, handler: H) -> Slider<H> {
        Slider {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            size_bounds: SizeBounds::default(),

            handle: SliderHandle {
                widget_tag: WidgetTag::new(),
                rect: BoundBox::new2(0, 0, 0, 0),
                size_bounds: SizeBounds::default(),

                value,
                step,
                value_range,

                click_pos: None,
                pixel_range: 0..=0,

                handler,
            },
        }
    }

    /// Retrieves the value stored in the slider.
    #[inline]
    pub fn value(&self) -> f32 {
        self.handle.value
    }

    /// Retrieves the range of possible values the slider can contain.
    #[inline]
    pub fn range(&self) -> RangeInclusive<f32> {
        self.handle.value_range.clone()
    }

    /// Retrieves the step, to which the value is snapped to.
    ///
    /// Calling this function forces the slider to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn step(&self) -> f32 {
        self.handle.step
    }

    /// Retrieves the value stored in the slider, for mutation.
    ///
    /// Calling this function forces the slider to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn value_mut(&mut self) -> &mut f32 {
        self.widget_tag.request_redraw().request_relayout();
        &mut self.handle.value
    }

    /// Retrieves the range of possible values the slider can contain, for mutation.
    ///
    /// Calling this function forces the slider to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn range_mut(&mut self) -> &mut RangeInclusive<f32> {
        self.widget_tag.request_redraw().request_relayout();
        &mut self.handle.value_range
    }

    /// Retrieves the step, to which the value is snapped to, for mutation.
    ///
    /// Calling this function forces the slider to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn step_mut(&mut self) -> &mut f32 {
        self.widget_tag.request_redraw().request_relayout();
        &mut self.handle.step
    }
}

impl<H> Widget for Slider<H>
    where H: SliderHandler
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

impl<H> Widget for SliderHandle<H>
    where H: SliderHandler
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
        &mut self.rect
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEventSourced, _: InputState) -> EventOps {
        if let WidgetEventSourced::This(ref event) = event {
            let start_value = self.value;
            match event {
                WidgetEvent::MouseDown{pos, in_widget: true, button: MouseButton::Left} => {
                    self.click_pos = Some(pos.x);
                    self.widget_tag.request_redraw();
                },
                WidgetEvent::MouseMove{new_pos, ..} => {
                    if let Some(click_pos) = self.click_pos {
                        let mut offset_rect = OffsetBox::from(self.rect);
                        offset_rect.origin.x += new_pos.x - click_pos;

                        if offset_rect.min().x < *self.pixel_range.start() {
                            offset_rect.origin.x = *self.pixel_range.start();
                        }
                        if offset_rect.max().x > *self.pixel_range.start() {
                            offset_rect.origin.x = *self.pixel_range.end() - offset_rect.dims.x;
                        }

                        let bar_len = *self.pixel_range.end() - *self.pixel_range.start();

                        let value_lerp_factor = offset_rect.center().x as f32 / bar_len as f32;
                        self.value = f32::lerp(*self.value_range.start(), *self.value_range.end(), value_lerp_factor);

                        // Snap the value to the step.
                        self.value = ((self.value - *self.value_range.start()) / self.step).round() * self.step + *self.value_range.start();

                        // Snap the head to the value
                        offset_rect.origin.x =
                            (
                                (
                                    (self.value - *self.value_range.start())
                                    / (*self.value_range.end() - *self.value_range.start())
                                )
                                * (bar_len - offset_rect.dims.x) as f32
                            ) as i32
                            + *self.pixel_range.start();

                        self.rect = BoundBox::from(offset_rect);
                    }
                },
                WidgetEvent::MouseUp{button: MouseButton::Left, pressed_in_widget: true, ..} => {
                    self.click_pos = None;
                    self.widget_tag.request_redraw();
                },
                _ => ()
            }
            if self.value != start_value {
                if let Some(message) = self.handler.on_move(start_value, self.value) {
                    self.widget_tag.broadcast_message(message);
                }
                self.widget_tag.request_redraw();
            }
        }
        EventOps {
            focus: None,
            bubble: event.default_bubble(),
        }
    }
}

impl<R, H> WidgetRenderable<R> for Slider<H>
    where R: Renderer,
          H: SliderHandler
{
    type Theme = SliderTheme;

    fn theme(&self) -> SliderTheme {
        SliderTheme(())
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, layout: &mut R::Layout) {
        let result = layout.finish();
        self.size_bounds = result.size_bounds;
        self.handle.pixel_range = result.content_rect.min.x..=result.content_rect.max.x;
    }
}

impl<R, H> WidgetRenderable<R> for SliderHandle<H>
    where R: Renderer,
          H: SliderHandler
{
    type Theme = SliderHandleTheme;

    fn theme(&self) -> SliderHandleTheme {
        SliderHandleTheme(())
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, layout: &mut R::Layout) {
        let result = layout.finish();
        self.size_bounds = result.size_bounds;
    }
}

impl WidgetTheme for SliderTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {
        None
    }
}

impl WidgetTheme for SliderHandleTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {
        None
    }
}
