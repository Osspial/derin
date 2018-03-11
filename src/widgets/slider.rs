use widgets::{Contents, ContentsInner};
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSubtrait, WidgetSubtraitMut, Widget};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox};
use dct::layout::SizeBounds;

use gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
pub struct Slider {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    value: f32,
    min: f32,
    max: f32,
}

impl Slider {
    pub fn new() -> Slider {
        Slider {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            value: 0.0,
            min: 0.0,
            max: 1.0
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
        frame.upload_primitives(ArrayVec::from([
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
                prim: Prim::Image
            },
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
                prim: Prim::Image
            },
        ]).into_iter());
    }

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

    #[inline]
    fn subtrait(&self) -> WidgetSubtrait<A, F> {
        WidgetSubtrait::Widget(self)
    }

    #[inline]
    fn subtrait_mut(&mut self) -> WidgetSubtraitMut<A, F> {
        WidgetSubtraitMut::Widget(self)
    }
}
