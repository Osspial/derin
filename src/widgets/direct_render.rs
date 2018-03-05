use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSubtrait, WidgetSubtraitMut, Widget, };
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;

use cgmath::Point2;
use cgmath_geometry::BoundBox;

use gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

use std::mem;

pub struct DirectRender<R>
    where R: DirectRenderState
{
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    render_state: R
}

pub trait DirectRenderState {
    type RenderType;

    fn render(&self, _: &mut Self::RenderType);
}

impl<R: DirectRenderState> DirectRender<R> {
    pub fn new(render_state: R) -> DirectRender<R> {
        DirectRender {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            render_state
        }
    }

    pub fn render_state(&self) -> &R {
        &self.render_state
    }

    pub fn render_state_mut(&mut self) -> &mut R {
        self.update_tag.mark_render_self();
        &mut self.render_state
    }
}

impl<A, F, R> Widget<A, F> for DirectRender<R>
    where R: DirectRenderState + 'static,
          F: PrimFrame<DirectRender = R::RenderType>
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

    fn render(&self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives(Some(ThemedPrim {
            theme_path: "DirectRender",
            min: Point2::new(
                RelPoint::new(-1.0, 0),
                RelPoint::new(-1.0, 0),
            ),
            max: Point2::new(
                RelPoint::new( 1.0, 0),
                RelPoint::new( 1.0, 0)
            ),
            prim: unsafe{ Prim::DirectRender(mem::transmute((&|render_type: &mut R::RenderType| self.render_state.render(render_type)) as &Fn(&mut R::RenderType))) }
        }).into_iter());
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
