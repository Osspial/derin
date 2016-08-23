pub mod primitive;
pub mod gl;

use std::ops::{Add, Div};

use self::gl::BufferData;

use cgmath::{Vector2};

pub trait Drawable: Shadable {
    fn buffer_data(&self) -> &BufferData;
}

pub trait Shadable {
    type Composite: Composite;

    fn shader_data<'a>(&'a self) -> Shader<'a, Self::Composite>;
    /// The number of times that this type's shader data has been updated. Note that this doesn't
    /// have to be exact - all that matters is that, if there has been an update since the last time
    /// this function was called, the number is greater than it was the previous call.
    fn num_updates(&self) -> u64;
}

pub trait Composite: Shadable 
        where Self: Sized {
    type Foreground: Shadable;
    type Fill: Shadable;
    type Backdrop: Shadable;

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

impl Shadable for () {
    type Composite = ();

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
    pub fn new(rel_x: f32, rel_y: f32, abs_x: f32, abs_y: f32) -> Complex {
        Complex {
            rel: Point::new(rel_x, rel_y),
            abs: Point::new(abs_x, abs_y)
        }
    }

    pub fn new_rel(x: f32, y: f32) -> Complex {
        Complex {
            rel: Point::new(x, y),
            abs: Point::new(0.0, 0.0)
        }
    }

    pub fn new_abs(x: f32, y: f32) -> Complex {
        Complex {
            rel: Point::new(0.0, 0.0),
            abs: Point::new(x, y)
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

impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y
        }
    }
}

impl Div<f32> for Point {
    type Output = Point;

    fn div(self, divisor: f32) -> Point {
        Point {
            x: self.x / divisor,
            y: self.y / divisor
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

    pub fn center(self) -> Complex {
        Complex {
            rel: (self.upleft.rel + self.lowright.rel) / 2.0,
            abs: (self.upleft.abs + self.lowright.abs) / 2.0
        }
    }
}

impl Default for Rect {
    fn default() -> Rect {
        Rect::new(
            Complex::new_rel(-1.0,  1.0),
            Complex::new_rel( 1.0, -1.0)
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

            Composite{ref foreground, ref fill, ref backdrop, ..} =>
                foreground.shader_data().count() +
                fill.shader_data().count() +
                backdrop.shader_data().count(),

            None => 0
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
