pub mod primitive;
pub mod gl;
pub mod font;

use std::ops::{Add, Sub, Mul, Div};

use self::gl::BufferData;
use self::font::Font;

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
    /// The Ratio component
    pub rat: Point,
    /// The [Point (as in the typographic unit)](https://en.wikipedia.org/wiki/Point_(typography)) component
    pub pts: Point
}

impl Complex {
    pub fn new(rat_x: f32, rat_y: f32, pts_x: f32, pts_y: f32) -> Complex {
        Complex {
            rat: Point::new(rat_x, rat_y),
            pts: Point::new(pts_x, pts_y)
        }
    }

    pub fn new_rat(x: f32, y: f32) -> Complex {
        Complex {
            rat: Point::new(x, y),
            pts: Point::new(0.0, 0.0)
        }
    }

    pub fn new_pts(x: f32, y: f32) -> Complex {
        Complex {
            rat: Point::new(0.0, 0.0),
            pts: Point::new(x, y)
        }
    }

    pub fn from_linears(x: LinearComplex, y: LinearComplex) -> Complex {
        Complex {
            rat: Point::new(x.rat, y.rat),
            pts: Point::new(x.pts, y.pts)
        }
    }

    pub fn x(self) -> LinearComplex {
        LinearComplex {
            rat: self.rat.x,
            pts: self.pts.x
        }
    }

    pub fn y(self) -> LinearComplex {
        LinearComplex {
            rat: self.rat.y,
            pts: self.pts.y
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LinearComplex {
    pub rat: f32,
    pub pts: f32
}

impl LinearComplex {
    pub fn new(rat: f32, pts: f32) -> LinearComplex {
        LinearComplex {
            rat: rat,
            pts: pts
        }
    }

    pub fn new_rat(rat: f32) -> LinearComplex {
        LinearComplex {
            rat: rat,
            pts: 0.0
        }
    }

    pub fn new_pts(pts: f32) -> LinearComplex {
        LinearComplex {
            rat: 0.0,
            pts: pts
        }
    }
}

impl Add for LinearComplex {
    type Output = LinearComplex;

    fn add(self, other: LinearComplex) -> LinearComplex {
        LinearComplex {
            rat: self.rat + other.rat,
            pts: self.pts + other.pts
        }
    }
}

impl Sub for LinearComplex {
    type Output = LinearComplex;

    fn sub(self, other: LinearComplex) -> LinearComplex {
        LinearComplex {
            rat: self.rat - other.rat,
            pts: self.pts - other.pts
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

impl Sub for Point {
    type Output = Point;

    fn sub(self, other: Point) -> Point {
        Point {
            x: self.x - other.x,
            y: self.y - other.y
        }
    }
}

impl Mul<f32> for Point {
    type Output = Point;

    fn mul(self, mult: f32) -> Point {
        Point {
            x: self.x * mult,
            y: self.y * mult
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

impl Mul<Vector2<f32>> for Point {
    type Output = Point;

    fn mul(self, mult: Vector2<f32>) -> Point {
        Point {
            x: self.x * mult.x,
            y: self.y * mult.y
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorVert {
    pub pos: Complex,
    // TODO: Remove the Vector2, replace with non-cgmath type, and get rid of mem::zeroed inits for ColorVert
    pub normal: Vector2<f32>,
    pub color: Color
}

impl ColorVert {
    #[inline]
    pub fn new(pos: Complex, normal: Vector2<f32>, color: Color) -> ColorVert {
        ColorVert {
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
            rat: Point::new(self.lowright.rat.x, self.upleft.rat.y),
            pts: Point::new(self.lowright.pts.x, self.upleft.pts.y)
        }
    }

    /// Calculate lower-left corner of rectangle
    pub fn lowleft(self) -> Complex {
        Complex {
            rat: Point::new(self.upleft.rat.x, self.lowright.rat.y),
            pts: Point::new(self.upleft.pts.x, self.lowright.pts.y)
        }
    }

    pub fn center(self) -> Complex {
        Complex {
            rat: (self.upleft.rat + self.lowright.rat) / 2.0,
            pts: (self.upleft.pts + self.lowright.pts) / 2.0
        }
    }

    pub fn width(self) -> LinearComplex {
        self.upleft.x() - self.lowright.x()
    }

    pub fn height(self) -> LinearComplex {
        self.upleft.y() - self.lowright.y()
    }
}

impl Default for Rect {
    fn default() -> Rect {
        Rect::new(
            Complex::new_rat(-1.0,  1.0),
            Complex::new_rat( 1.0, -1.0)
        )
    }
}

pub enum Shader<'a, C: 'a + Composite> {
    Transform {
        scale: Rect,
        operand: &'a Shadable<Composite = C>
    },

    Verts {
        verts: &'a [ColorVert],
        indices: &'a [u16]
    },

    Text {
        rect: Rect,
        text: &'a str,
        color: Color,
        font: &'a Font,
        font_size: u32
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

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
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
