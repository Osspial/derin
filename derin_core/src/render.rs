use tree::NodeIdent;
use cgmath::Point2;
use cgmath_geometry::BoundBox;

pub trait Renderer {
    type Frame: RenderFrame;
    #[inline]
    fn force_full_redraw(&self) -> bool {false}
    fn make_frame(&mut self) -> (&mut Self::Frame, <Self::Frame as RenderFrame>::Transform);
    fn finish_frame(&mut self, theme: &<Self::Frame as RenderFrame>::Theme);
}

pub trait RenderFrame {
    type Transform: Copy;
    type Theme: Theme;
    type Primitive: Copy;

    fn upload_primitives<I>(&mut self, node_ident: &[NodeIdent], theme: &Self::Theme, transform: &Self::Transform, prim_iter: I)
        where I: Iterator<Item=Self::Primitive>;
    fn child_rect_transform(self_transform: &Self::Transform, child_rect: BoundBox<Point2<u32>>) -> Self::Transform;
}

pub trait Theme {
    type Key: ?Sized;
    type ThemeValue;
    fn node_theme(&self, key: &Self::Key) -> Self::ThemeValue;
}

pub struct FrameRectStack<'a, F: 'a + RenderFrame> {
    frame: &'a mut F,
    transform: F::Transform,

    theme: &'a F::Theme,

    pop_node_ident: bool,
    node_ident: &'a mut Vec<NodeIdent>,
}

impl<'a, F: RenderFrame> FrameRectStack<'a, F> {
    #[inline]
    pub(crate) fn new(
        frame: &'a mut F,
        base_transform: F::Transform,
        theme: &'a F::Theme,
        node_ident_vec: &'a mut Vec<NodeIdent>
    ) -> FrameRectStack<'a, F>
    {
        FrameRectStack {
            frame,
            transform: base_transform,

            theme,

            pop_node_ident: false,
            node_ident: node_ident_vec
        }
    }

    #[inline]
    pub fn upload_primitives<I>(&mut self, prim_iter: I)
        where I: Iterator<Item=F::Primitive>
    {
        let node_ident = &self.node_ident;
        self.frame.upload_primitives(node_ident, self.theme, &self.transform, prim_iter)
    }

    #[inline]
    pub fn enter_child_rect<'b>(&'b mut self, child_rect: BoundBox<Point2<u32>>) -> FrameRectStack<'b, F> {
        FrameRectStack {
            frame: self.frame,
            transform: F::child_rect_transform(&self.transform, child_rect),
            theme: self.theme,
            node_ident: self.node_ident,
            pop_node_ident: false,
        }
    }

    pub(crate) fn enter_child_node<'b>(&'b mut self, child_ident: NodeIdent) -> FrameRectStack<'b, F> {
        self.node_ident.push(child_ident);
        FrameRectStack {
            frame: self.frame,
            transform: self.transform,
            theme: self.theme,
            node_ident: self.node_ident,
            pop_node_ident: true,
        }
    }
}

impl<'a, F: RenderFrame> Drop for FrameRectStack<'a, F> {
    fn drop(&mut self) {
        if self.pop_node_ident {
            self.node_ident.pop().expect("Too many pops");
        }
    }
}
