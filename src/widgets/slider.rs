use event::{EventOps, WidgetEvent, InputState, MouseButton};
use core::tree::{WidgetIdent, UpdateTag, WidgetSubtrait, WidgetSubtraitMut, Widget};
use core::render::{FrameRectStack, Theme};
use core::popup::ChildPopupsMut;
use theme::RescaleRules;
use gullery::glsl::Ni32;

use cgmath::{EuclideanSpace, Point2};
use cgmath_geometry::{BoundBox, GeoBox};

use gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

#[derive(Debug, Clone)]
pub struct Slider {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,

    value: f32,

    min: f32,
    max: f32,

    slide_range_min: i32,
    slide_range_max: i32,
    head_offset: i32,

    slider_rect: BoundBox<Point2<i32>>,
    slider_click_pos_x: Option<i32>
}

impl Slider {
    pub fn new() -> Slider {
        Slider {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            value: 1.0,
            min: 0.0,
            max: 1.0,
            slide_range_min: 0,
            slide_range_max: 0,
            head_offset: 0,
            slider_rect: BoundBox::new2(0, 0, 0, 0),
            slider_click_pos_x: None
        }
    }

    fn set_value_px(&mut self, x_pos: i32) {
        if let Some(slider_click_pos_x) = self.slider_click_pos_x {
            self.value = (x_pos - slider_click_pos_x - (self.slide_range_min - self.head_offset)) as f32 / (self.slide_range_max - self.slide_range_min) as f32;
            self.value = self.value.min(self.max).max(self.min);
        }
    }
}

impl<A, F> Widget<A, F> for Slider
    where F: PrimFrame
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
    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, bubble_source: &[WidgetIdent]) -> EventOps<A, F> {
        if bubble_source.len() == 0 {
            let slide_bar_rect = BoundBox::new2(
                self.slide_range_min - self.head_offset, self.slider_rect.min.y,
                self.slide_range_max + self.head_offset, self.slider_rect.max.y
            );
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
                    self.update_tag.mark_render_self();
                },
                WidgetEvent::MouseMove{new_pos, ..}
                    if self.slider_click_pos_x.is_some() =>
                {
                    let slider_click_pos_x = self.slider_click_pos_x.unwrap();
                    self.set_value_px(new_pos.x);
                    self.update_tag.mark_render_self();
                },
                WidgetEvent::MouseUp{button: MouseButton::Left, ..} => {
                    self.slider_click_pos_x = None;
                    self.update_tag.mark_render_self();
                },
                _ => ()
            }
        }
        EventOps {
            action: None,
            focus: None,
            bubble: false,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    #[inline]
    fn subtrait(&self) -> WidgetSubtrait<A, F> {
        WidgetSubtrait::Widget(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> WidgetSubtraitMut<A, F> {
        WidgetSubtraitMut::Widget(self)
    }
}
