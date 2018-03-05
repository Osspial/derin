use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSubtrait, WidgetSubtraitMut, Widget};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;

use cgmath::Point2;
use cgmath_geometry::BoundBox;
use dct::layout::SizeBounds;

use gl_render::{ThemedPrim, PrimFrame, RenderString, RelPoint, Prim};

#[derive(Debug, Clone)]
pub struct Label {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    string: RenderString
}

impl Label {
    pub fn new(string: String) -> Label {
        Label {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            string: RenderString::new(string)
        }
    }

    pub fn string(&self) -> &str {
        self.string.string()
    }

    pub fn string_mut(&mut self) -> &mut String {
        self.update_tag.mark_render_self();
        self.string.string_mut()
    }
}

impl<A, F> Widget<A, F> for Label
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

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::new_min(self.string.min_size())
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives([
            ThemedPrim {
                theme_path: "Label",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::String(&self.string)
            }
        ].iter().cloned());
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
