use cgmath::{Point2, Vector2};

pub trait Drawable<C: Composite = ()> {
    fn shader_data<'a>(&'a self) -> Shader<'a, C>;
}

pub enum Shader<'a, C: Composite> {
    Verts {
        vertices: &'a [Vertex],
        indices: &'a [u16]
    },

    Composite {
        rect: Rect,
        border: Border,
        foreground: C::Foreground,
        fill: C::Fill,
        backdrop: C::Backdrop
    },

    None
}

pub struct Vertex {
    pub pos: Point2<f32>,
    pub normal: Vector2<f32>,
    pub color: Color
}

pub trait Composite: Drawable<Self> 
        where Self: Sized {
    type Foreground: Drawable<Self>;
    type Fill: Drawable<Self>;
    type Backdrop: Drawable<Self>;

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
}

impl Drawable for () {
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        Shader::None
    }
}

impl Composite for () {
    type Foreground = ();
    type Fill = ();
    type Backdrop = ();

    fn foreground(&self) {}
    fn fill(&self) {}
    fn backdrop(&self) {}
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
