extern crate gl;
extern crate gl_raii;

pub trait Drawable {
    type Foreground: Drawable;
    type Fill: Drawable;
    type Backdrop: Drawable;

    fn rect(&self) -> Rect {
        Rect::new(
            Point::new(-1.0,  1.0),
            Point::new( 1.0, -1.0)
        )
    }

    fn mask(&self) -> Mask;
    fn border(&self) -> Border;
    fn foreground(&self) -> Self::Foreground;
    fn fill(&self) -> Self::Fill;
    fn backdrop(&self) -> Self::Backdrop;

    fn draw<S: Surface>(&self, &S);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    /// Upper-left corner of rectangle
    pub ul: Point,
    // Lower-right corner of rectangle
    pub lr: Point
}

impl Rect {
    pub fn new(ul: Point, lr: Point) -> Rect {
        Rect {
            ul: ul,
            lr: lr
        }
    }
}

pub struct Mask {}

pub struct Border {}

pub trait Surface {}

#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32
}

impl Point {
    pub fn new(x: f32, y: f32) -> Point {
        Point {
            x: x,
            y: y
        }
    }
}
