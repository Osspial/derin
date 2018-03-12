use event::{EventOps, WidgetEvent, InputState, MouseButton};
use core::tree::{WidgetIdent, UpdateTag, WidgetSubtrait, WidgetSubtraitMut, Widget};
use core::render::{FrameRectStack, Theme};
use core::popup::ChildPopupsMut;
use theme::RescaleRules;
use gullery::glsl::Ni32;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, GeoBox};

use gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

pub trait SliderHandler {
    type Action: 'static;

    fn on_move(&mut self, old_value: f32, new_value: f32) -> Option<Self::Action>;
}

#[derive(Debug, Clone)]
pub struct Slider<H: SliderHandler> {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,

    value: f32,

    min: f32,
    max: f32,
    step: f32,

    slide_range_min: i32,
    slide_range_max: i32,
    head_offset: i32,

    slider_rect: BoundBox<Point2<i32>>,
    slider_click_pos_x: Option<i32>,
    handler: H
}

impl<H: SliderHandler> Slider<H> {
    pub fn new(value: f32, min: f32, max: f32, step: f32, handler: H) -> Slider<H> {
        Slider {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            value, min, max, step,
            slide_range_min: 0,
            slide_range_max: 0,
            head_offset: 0,
            slider_rect: BoundBox::new2(0, 0, 0, 0),
            slider_click_pos_x: None,
            handler
        }
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    #[inline]
    pub fn range(&self) -> (f32, f32) {
        (self.min, self.max)
    }

    #[inline]
    pub fn value_mut(&mut self) -> &mut f32 {
        self.update_tag.mark_render_self();
        &mut self.value
    }

    #[inline]
    pub fn range_mut(&mut self) -> (&mut f32, &mut f32) {
        self.update_tag.mark_render_self();
        (&mut self.min, &mut self.max)
    }

    fn set_value_px(&mut self, x_pos: i32) {
        if let Some(slider_click_pos_x) = self.slider_click_pos_x {
            self.value = (x_pos - slider_click_pos_x - (self.slide_range_min - self.head_offset)) as f32
                / (self.slide_range_max - self.slide_range_min) as f32
                * (self.max - self.min);
            self.value = ((self.value - self.min) / self.step).round() * self.step + self.min;
            self.value = self.value.min(self.max).max(self.min);
        }
    }
}

impl<F, H> Widget<H::Action, F> for Slider<H>
    where F: PrimFrame,
          H: SliderHandler
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        if self.value != self.max && self.value != self.min {
            self.value = ((self.value - self.min) / self.step).round() * self.step + self.min;
        }
        self.value = self.value.min(self.max).max(self.min);
        let mut bar_rect = BoundBox::new2(0, 0, 0, 0);
        let bar_margins = match frame.theme().widget_theme("Slider::Bar").image.map(|b| b.rescale) {
            Some(RescaleRules::Slice(margins)) => margins,
            _ => Default::default()
        };
        let head_offset = frame.theme().widget_theme("Slider::Head").image.map(|h| h.dims.width() / 2).unwrap_or(0);
        self.head_offset = head_offset as i32;

        frame.upload_primitives(Some(
            ThemedPrim {
                theme_path: "Slider::Bar",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image,
                rect_px_out: Some(&mut bar_rect)
            }
        ).into_iter());

        bar_rect.min.x += bar_margins.left as i32 + head_offset as i32;
        bar_rect.max.x -= bar_margins.right as i32 + head_offset as i32;
        bar_rect.min.y += bar_margins.top as i32;
        bar_rect.max.y -= bar_margins.bottom as i32;
        self.slide_range_min = bar_rect.min.x;
        self.slide_range_max = bar_rect.max.x;

        let proprtion_along = Ni32::from_bounded((self.value - self.min) / (self.max - self.min));
        let x_loc = bar_rect.width() * proprtion_along + bar_rect.min.x;

        frame.upload_primitives(Some(
            ThemedPrim {
                theme_path: "Slider::Head",
                min: Point2::new(
                    RelPoint::new(-2.0, x_loc),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 0.0, x_loc),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image,
                rect_px_out: Some(&mut self.slider_rect)
            },
        ).into_iter());
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<H::Action, F>>, bubble_source: &[WidgetIdent]) -> EventOps<H::Action, F> {
        let mut action = None;
        if bubble_source.len() == 0 {
            let slide_bar_rect = BoundBox::new2(
                self.slide_range_min - self.head_offset, self.slider_rect.min.y,
                self.slide_range_max + self.head_offset, self.slider_rect.max.y
            );
            let start_value = self.value;
            match event {
                WidgetEvent::MouseDown{pos, in_widget: true, button: MouseButton::Left}
                    if self.slider_rect.contains(pos) =>
                {
                    self.slider_click_pos_x = Some(pos.x - self.slider_rect.min().x);
                    self.update_tag.mark_render_self();
                },
                WidgetEvent::MouseDown{pos, in_widget: true, button: MouseButton::Left}
                    if slide_bar_rect.contains(pos) =>
                {
                    self.slider_click_pos_x = Some(self.slider_rect.center().x - self.slider_rect.min().x);
                    self.set_value_px(pos.x);
                },
                WidgetEvent::MouseMove{new_pos, ..}
                    if self.slider_click_pos_x.is_some() =>
                {
                    self.set_value_px(new_pos.x);
                },
                WidgetEvent::MouseUp{button: MouseButton::Left, ..} => {
                    self.slider_click_pos_x = None;
                    self.update_tag.mark_render_self();
                },
                _ => ()
            }
            if self.value != start_value {
                action = self.handler.on_move(start_value, self.value);
                self.update_tag.mark_render_self();
            }
        }
        EventOps {
            action,
            focus: None,
            bubble: false,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    #[inline]
    fn subtrait(&self) -> WidgetSubtrait<H::Action, F> {
        WidgetSubtrait::Widget(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> WidgetSubtraitMut<H::Action, F> {
        WidgetSubtraitMut::Widget(self)
    }
}
