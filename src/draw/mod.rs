use cgmath::{Vector2};

pub trait Drawable<C: Composite = ()> {
    fn shader_data<'a>(&'a self) -> Shader<'a, C>;
}

pub enum Shader<'a, C: Composite> {
    Verts {
        verts: &'a [Vertex],
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

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: Point,
    pub normal: Vector2<f32>,
    pub color: Color
}

impl Vertex {
    #[inline]
    pub fn new(pos: Point, normal: Vector2<f32>, color: Color) -> Vertex {
        Vertex {
            pos: pos,
            normal: normal,
            color: color
        }
    }
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

impl<C: Composite> Drawable<C> for C {
    default fn shader_data<'a>(&'a self) -> Shader<'a, C> {
        Shader::Composite {
            rect: self.rect(),
            border: self.border(),
            foreground: self.foreground(),
            fill: self.fill(),
            backdrop: self.backdrop()
        }
    }
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

#[derive(Debug, Clone, Copy, Default)]
pub struct Complex {
    pub rel: f32,
    pub abs: f32
}

impl Complex {
    pub fn new(rel: f32, abs: f32) -> Complex {
        Complex {
            rel: rel,
            abs: abs
        }
    }

    pub fn new_rel(rel: f32) -> Complex {
        Complex {
            rel: rel,
            abs: 0.0
        }
    }

    pub fn new_abs(abs: f32) -> Complex {
        Complex {
            rel: 0.0,
            abs: abs
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: Complex,
    pub y: Complex
}

impl Point {
    pub fn new(x: Complex, y: Complex) -> Point {
        Point {
            x: x,
            y: y
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    /// Upper-left corner of rectangle
    pub upleft: Point,
    // Lower-right corner of rectangle
    pub lowright: Point
}

impl Rect {
    pub fn new(upleft: Point, lowright: Point) -> Rect {
        Rect {
            upleft: upleft,
            lowright: lowright
        }
    }

    /// Calculate upper-right corner of rectangle
    pub fn upright(self) -> Point {
        Point {
            x: self.upleft.x,
            y: self.lowright.y
        }
    }

    /// Calculate lower-left corner of rectangle
    pub fn lowleft(self) -> Point {
        Point {
            x: self.lowright.x,
            y: self.upleft.y
        }
    }
}

impl Default for Rect {
    fn default() -> Rect {
        let one = Complex::new_rel(1.0);
        let none = Complex::new_rel(-1.0);
        Rect::new(
            Point::new(none,  one),
            Point::new( one, none)
        )
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
