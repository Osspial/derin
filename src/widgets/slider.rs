use widgets::assistants::SliderAssist;
use event::{EventOps, WidgetEvent, InputState, MouseButton};
use core::tree::{WidgetIdent, UpdateTag, Widget};
use core::render::{FrameRectStack, Theme};
use core::popup::ChildPopupsMut;
use theme::RescaleRules;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};

use gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

pub trait SliderHandler {
    type Action: 'static;

    fn on_move(&mut self, old_value: f32, new_value: f32) -> Option<Self::Action>;
}

#[derive(Debug, Clone)]
pub struct Slider<H: SliderHandler> {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,

    assist: SliderAssist,
    handler: H
}

impl<H: SliderHandler> Slider<H> {
    pub fn new(value: f32, step: f32, min: f32, max: f32, handler: H) -> Slider<H> {
        Slider {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            assist: SliderAssist {
                value, step, min, max,

                head_size: 0,
                bar_rect: BoundBox::new2(0, 0, 0, 0),
                head_click_pos: None,
                horizontal: true
            },
            handler
        }
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.assist.value
    }

    #[inline]
    pub fn range(&self) -> (f32, f32) {
        (self.assist.min, self.assist.max)
    }

    #[inline]
    pub fn value_mut(&mut self) -> &mut f32 {
        self.update_tag.mark_render_self();
        &mut self.assist.value
    }

    #[inline]
    pub fn range_mut(&mut self) -> (&mut f32, &mut f32) {
        self.update_tag.mark_render_self();
        (&mut self.assist.min, &mut self.assist.max)
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
        self.assist.round_to_step();
        let bar_margins = match frame.theme().widget_theme("Slider::Bar").image.map(|b| b.rescale) {
            Some(RescaleRules::Slice(margins)) => margins,
            _ => Default::default()
        };
        let head_rect = frame.theme().widget_theme("Slider::Head").image.map(|h| h.dims).unwrap_or(DimsBox::new2(0, 0));
        self.assist.head_size = head_rect.width() as i32;

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
                rect_px_out: Some(&mut self.assist.bar_rect)
            }
        ).into_iter());

        self.assist.bar_rect.min.x += bar_margins.left as i32;
        self.assist.bar_rect.max.x -= bar_margins.right as i32;
        self.assist.bar_rect.min.y += bar_margins.top as i32;
        self.assist.bar_rect.max.y -= bar_margins.bottom as i32;
        let bar_rect_center_y = self.assist.bar_rect.center().y;
        self.assist.bar_rect.min.y = bar_rect_center_y - (head_rect.height() / 2) as i32;
        self.assist.bar_rect.max.y = bar_rect_center_y + (head_rect.height() / 2) as i32;

        let head_rect = self.assist.head_rect();
        frame.upload_primitives(Some(
            ThemedPrim {
                theme_path: "Slider::Head",
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
            },
        ).into_iter());
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<H::Action, F>>, bubble_source: &[WidgetIdent]) -> EventOps<H::Action, F> {
        let mut action = None;
        if bubble_source.len() == 0 {
            let start_value = self.assist.value;
            match event {
                WidgetEvent::MouseDown{pos, in_widget: true, button: MouseButton::Left} => {
                    self.assist.click_head(pos);
                    self.update_tag.mark_render_self();
                },
                WidgetEvent::MouseMove{new_pos, ..} => {
                    self.assist.move_head(new_pos.x);
                },
                WidgetEvent::MouseUp{button: MouseButton::Left, ..} => {
                    self.assist.head_click_pos = None;
                    self.update_tag.mark_render_self();
                },
                _ => ()
            }
            if self.assist.value != start_value {
                action = self.handler.on_move(start_value, self.assist.value);
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
}
