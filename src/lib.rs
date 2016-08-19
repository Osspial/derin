extern crate gl;
extern crate gl_raii;

pub trait Drawable {
    type Foreground: Drawable;
    type Fill: Drawable;
    type Backdrop: Drawable;

    fn rect(&self) -> Rect;
    fn mask(&self) -> Mask;
    fn border(&self) -> Border;
    fn foreground(&self) -> Self::Foreground;
    fn fill(&self) -> Self::Fill;
    fn backdrop(&self) -> Self::Backdrop;

    fn draw<S: Surface>(&self, &S);
}

pub struct Rect {}

pub struct Mask {}

pub struct Border {}

pub trait Surface {}
