use widgets::{Contents, ContentsInner};
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSubtrait, WidgetSubtraitMut, Widget};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox};
use dct::layout::SizeBounds;

use gl_render::PrimFrame;

use std::cell::Cell;

#[derive(Debug, Clone)]
pub struct Label {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    contents: ContentsInner,
    min_size: Cell<DimsBox<Point2<i32>>>
}

impl Label {
    pub fn new(contents: Contents<String>) -> Label {
        Label {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            contents: contents.to_inner(),
            min_size: Cell::new(DimsBox::new2(0, 0))
        }
    }

    pub fn contents(&self) -> Contents<&str> {
        self.contents.borrow()
    }

    pub fn contents_mut(&mut self) -> Contents<&mut String> {
        self.update_tag.mark_render_self();
        self.contents.borrow_mut()
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
        SizeBounds::new_min(self.min_size.get())
    }

    fn render(&self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives([
            self.contents.to_prim("Label")
        ].iter().cloned());
        self.min_size.set(self.contents.min_size(frame.theme()));
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
