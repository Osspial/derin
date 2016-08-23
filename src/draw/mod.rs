pub mod primitive;
pub mod gl;

use self::gl::BufferData;

use cgmath::{Vector2};

pub trait Drawable<C: Composite = ()>: Shadable<C> {
    fn buffer_data(&self) -> &BufferData;
}

pub trait Shadable<C: Composite = ()> {
    fn shader_data<'a>(&'a self) -> Shader<'a, C>;
    /// The number of times that this type's shader data has been updated. Note that this doesn't
    /// have to be exact - all that matters is that, if there has been an update since the last time
    /// this function was called, the number is greater than it was the previous call.
    fn num_updates(&self) -> u64;
}

pub trait Composite: Shadable<Self> 
        where Self: Sized {
    type Foreground: Shadable<Self>;
    type Fill: Shadable<Self>;
    type Backdrop: Shadable<Self>;

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

impl<C: Composite> Shadable<C> for C {
    default fn shader_data<'a>(&'a self) -> Shader<'a, C> {
        Shader::Composite {
            rect: self.rect(),
            border: self.border(),
            foreground: self.foreground(),
            fill: self.fill(),
            backdrop: self.backdrop()
        }
    }

    default fn num_updates(&self) -> u64 {
        self.foreground().num_updates() +
        self.fill().num_updates() +
        self.backdrop().num_updates()
    }
}

impl Shadable for () {
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        Shader::None
    }

    fn num_updates(&self) -> u64 {
        0
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

pub trait Surface {
    fn draw<D: Drawable>(&mut self, &D);
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


#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Complex {
    pub rel: Point,
    pub abs: Point
}

impl Complex {
    pub fn new(rel: Point, abs: Point) -> Complex {
        Complex {
            rel: rel,
            abs: abs
        }
    }

    pub fn new_rel(rel: Point) -> Complex {
        Complex {
            rel: rel,
            abs: Point::new(0.0, 0.0)
        }
    }

    pub fn new_abs(abs: Point) -> Complex {
        Complex {
            rel: Point::new(0.0, 0.0),
            abs: abs
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
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

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub pos: Complex,
    pub normal: Vector2<f32>,
    pub color: Color
}

impl Vertex {
    #[inline]
    pub fn new(pos: Complex, normal: Vector2<f32>, color: Color) -> Vertex {
        Vertex {
            pos: pos,
            normal: normal,
            color: color
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Upper-left corner of rectangle
    pub upleft: Complex,
    // Lower-right corner of rectangle
    pub lowright: Complex
}

impl Rect {
    pub fn new(upleft: Complex, lowright: Complex) -> Rect {
        Rect {
            upleft: upleft,
            lowright: lowright
        }
    }

    /// Calculate upper-right corner of rectangle
    pub fn upright(self) -> Complex {
        Complex {
            rel: Point::new(self.lowright.rel.x, self.upleft.rel.y),
            abs: Point::new(self.lowright.abs.x, self.upleft.abs.y)
        }
    }

    /// Calculate lower-left corner of rectangle
    pub fn lowleft(self) -> Complex {
        Complex {
            rel: Point::new(self.upleft.rel.x, self.lowright.rel.y),
            abs: Point::new(self.upleft.abs.x, self.lowright.abs.y)
        }
    }
}

impl Default for Rect {
    fn default() -> Rect {
        Rect::new(
            Complex::new_rel(Point::new(-1.0,  1.0)),
            Complex::new_rel(Point::new( 1.0, -1.0))
        )
    }
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

impl<'a, C: Composite> Shader<'a, C> {
    pub fn count(&self) -> usize {
        use self::Shader::*;

        match *self {
            Verts{indices, ..} => indices.len(),
            None => 0,
            _ => unimplemented!()
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
