use cgmath::Point2;

pub trait Drawable {
    type Foreground: Drawable;
    type Fill: Drawable;
    type Backdrop: Drawable;

    fn rect(&self) -> Rect {
        Default::default()
    }

    // fn mask(&self) -> Mask;
    fn border(&self) -> Border {
        Default::default()
    }

    fn foreground(&self) -> Self::Foreground;
    fn fill(&self) -> Self::Fill;
    fn backdrop(&self) -> Self::Backdrop;

    fn draw<S: Surface>(&self, &S);
}


#[derive(Debug, Clone, Copy)]
pub struct Rect {
    /// Upper-left corner of rectangle
    pub ul: Point2<f32>,
    // Lower-right corner of rectangle
    pub lr: Point2<f32>
}

impl Default for Rect {
    fn default() -> Rect {
        Rect::new(
            Point2::new(-1.0,  1.0),
            Point2::new( 1.0, -1.0)
        )
    }
}

impl Rect {
    pub fn new(ul: Point2<f32>, lr: Point2<f32>) -> Rect {
        Rect {
            ul: ul,
            lr: lr
        }
    }
}

pub struct Mask {}

#[derive(Debug, Clone, Copy)]
pub enum Border {
    Solid {
        width: f32,
        color: Color
    },
    None
}

impl Default for Border {
    fn default() -> Border {
        Border::None
    }
}

pub trait Surface {}

#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8
}

impl Color {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color {
            r: r,
            g: g,
            b: b,
            a: a
        }
    }
}
