use tree::WidgetIdent;
use cgmath::{EuclideanSpace, Point2};
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};
use dct::cursor::CursorIcon;
use dct::layout::SizeBounds;

pub trait Renderer {
    type Frame: RenderFrame;
    #[inline]
    fn force_full_redraw(&self) -> bool {false}
    fn set_cursor_pos(&mut self, pos: Point2<i32>);
    fn set_cursor_icon(&mut self, icon: CursorIcon);
    fn set_size_bounds(&mut self, size_bounds: SizeBounds);
    fn resized(&mut self, new_size: DimsBox<Point2<u32>>);
    fn make_frame(&mut self) -> (&mut Self::Frame, BoundBox<Point2<i32>>);
    fn finish_frame(&mut self, theme: &<Self::Frame as RenderFrame>::Theme);
}

pub trait RenderFrame: 'static {
    type Theme: Theme;
    type Primitive;

    fn upload_primitives<I>(
        &mut self,
        widget_ident: &[WidgetIdent],
        theme: &Self::Theme,
        transform: BoundBox<Point2<i32>>,
        clip: BoundBox<Point2<i32>>,
        prim_iter: I
    )
        where I: Iterator<Item=Self::Primitive>;
}

pub trait Theme {
    type Key: ?Sized;
    type ThemeValue;
    fn widget_theme(&self, key: &Self::Key) -> Self::ThemeValue;
}

pub struct FrameRectStack<'a, F: 'a + RenderFrame> {
    frame: &'a mut F,
    transform: BoundBox<Point2<i32>>,
    clip_rect: BoundBox<Point2<i32>>,

    theme: &'a F::Theme,

    pop_widget_ident: bool,
    widget_ident: &'a mut Vec<WidgetIdent>,
}

impl<'a, F: RenderFrame> FrameRectStack<'a, F> {
    #[inline]
    pub(crate) fn new(
        frame: &'a mut F,
        base_transform: BoundBox<Point2<i32>>,
        theme: &'a F::Theme,
        widget_ident_vec: &'a mut Vec<WidgetIdent>
    ) -> FrameRectStack<'a, F>
    {
        FrameRectStack {
            frame,
            transform: base_transform,
            clip_rect: base_transform,

            theme,

            pop_widget_ident: false,
            widget_ident: widget_ident_vec
        }
    }

    #[inline(always)]
    pub fn theme(&self) -> &F::Theme {
        self.theme
    }

    #[inline]
    pub fn upload_primitives<I>(&mut self, prim_iter: I)
        where I: IntoIterator<Item=F::Primitive>
    {
        let widget_ident = &self.widget_ident;
        self.frame.upload_primitives(widget_ident, self.theme, self.transform, self.clip_rect, prim_iter.into_iter())
    }

    #[inline]
    pub fn enter_child_rect<'b>(&'b mut self, child_rect: BoundBox<Point2<i32>>) -> Option<FrameRectStack<'b, F>> {
        let child_transform = child_rect + self.transform.min().to_vec();
        Some(FrameRectStack {
            frame: self.frame,
            transform: child_transform,
            clip_rect: self.clip_rect.intersect_rect(child_transform)?,

            theme: self.theme,
            widget_ident: self.widget_ident,
            pop_widget_ident: false,
        })
    }

    pub(crate) fn enter_child_widget<'b>(&'b mut self, child_ident: WidgetIdent) -> FrameRectStack<'b, F> {
        self.widget_ident.push(child_ident);
        FrameRectStack {
            frame: self.frame,
            transform: self.transform,
            clip_rect: self.clip_rect,
            theme: self.theme,
            widget_ident: self.widget_ident,
            pop_widget_ident: true,
        }
    }
}

impl<'a, F: RenderFrame> Drop for FrameRectStack<'a, F> {
    fn drop(&mut self) {
        if self.pop_widget_ident {
            self.widget_ident.pop().expect("Too many pops");
        }
    }
}
