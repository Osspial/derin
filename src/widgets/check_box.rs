use widgets::{Contents, ContentsInner};
use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};

use core::event::{EventOps, InputState, WidgetEvent};
use core::popup::ChildPopupsMut;
use core::tree::{WidgetIdent, UpdateTag, Widget};
use core::render::FrameRectStack;
use dct::layout::SizeBounds;

use gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame};

use arrayvec::ArrayVec;

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

        frame.upload_primitives(ArrayVec::from([
            ThemedPrim {
                theme_path: image_str,
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image,
                rect_px_out: Some(&mut self.check_rect)
            },
            self.contents.to_prim("CheckBox", None)
        ]).into_iter());
    }

    fn on_widget_event(&mut self, event: WidgetEvent, input_state: InputState, popups_opt: Option<ChildPopupsMut<A, F>>, bubble_source: &[WidgetIdent]) -> EventOps<A, F> {
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
