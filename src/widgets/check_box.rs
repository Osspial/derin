use widgets::{Contents, ContentsInner};
use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};

use core::event::{EventOps, InputState, WidgetEvent};
use core::popup::ChildPopupsMut;
use core::tree::{WidgetIdent, UpdateTag, Widget};
use core::render::FrameRectStack;
use dct::layout::SizeBounds;

use gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame};

#[derive(Debug, Clone)]
pub struct CheckBox {
    update_tag: UpdateTag,
    rect: BoundBox<Point2<i32>>,

    check_rect: BoundBox<Point2<i32>>,
    contents: ContentsInner,
    checked: bool,
}

impl CheckBox {
    pub fn new(contents: Contents<String>, checked: bool) -> CheckBox {
        CheckBox {
            update_tag: UpdateTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),

            check_rect: BoundBox::new2(0, 0, 0, 0),
            contents: contents.to_inner(),
            checked
        }
    }

    pub fn contents(&self) -> Contents<&str> {
        self.contents.borrow()
    }

    pub fn contents_mut(&mut self) -> Contents<&mut String> {
        self.update_tag.mark_render_self();
        self.contents.borrow_mut()
    }

    pub fn checked(&self) -> bool {
        self.checked
    }

    pub fn checked_mut(&mut self) -> &mut bool {
        self.update_tag.mark_render_self();
        &mut self.checked
    }
}

impl<A, F> Widget<A, F> for CheckBox
    where F: PrimFrame
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.rect
    }

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::new_min(DimsBox::new(self.check_rect.dims()))
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        let image_str = match self.checked {
            true => "CheckBox::Checked",
            false => "CheckBox::Empty"
        };

        let mut content_rect = BoundBox::new2(0, 0, 0, 0);
        frame.upload_primitives(Some(self.contents.to_prim("CheckBox", Some(&mut content_rect))));

        frame.upload_primitives(Some(
            match content_rect == BoundBox::new2(0, 0, 0, 0) {
                true => ThemedPrim {
                    min: Point2::new(
                        RelPoint::new(-1.0, 0),
                        RelPoint::new(-1.0, 0),
                    ),
                    max: Point2::new(
                        RelPoint::new( 1.0, 0),
                        RelPoint::new( 1.0, 0)
                    ),
                    prim: Prim::Image,
                    theme_path: image_str,
                    rect_px_out: Some(&mut self.check_rect)
                },
                false => ThemedPrim {
                    min: Point2::new(
                        RelPoint::new(-1.0, 0),
                        RelPoint::new(-1.0, content_rect.min().y),
                    ),
                    max: Point2::new(
                        RelPoint::new( 1.0, 0),
                        RelPoint::new(-1.0, content_rect.max().y),
                    ),
                    prim: Prim::Image,
                    theme_path: image_str,
                    rect_px_out: Some(&mut self.check_rect)
                }
            }
        ));
    }

    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        let new_checked = match event {
            WidgetEvent::MouseUp{in_widget: true, pressed_in_widget, ..} => {
                match pressed_in_widget {
                    true => !self.checked,
                    false => self.checked
                }
            },
            _ => self.checked
        };

        if new_checked != self.checked {
            self.update_tag.mark_render_self();
            self.checked = new_checked;
        }


        EventOps {
            action: None,
            focus: None,
            bubble: event.default_bubble(),
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }
}
