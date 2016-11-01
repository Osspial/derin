pub mod primitives;
pub mod gl;
pub mod font;

use std::ops::{Add, Sub, Mul, Div, Deref, DerefMut};

use self::gl::{ShaderDataCollector};

use cgmath::{Vector2};


use fnv::FnvHasher;

use std::hash::BuildHasherDefault;
use std::collections::HashMap;



pub struct Widget<S: Shadable> {
    shadable: S,

    num_updates: u64,
    id: u64
}

impl<S: Shadable> Widget<S> {
    pub fn new(shadable: S) -> Widget<S> {
        Widget {
            shadable: shadable,

            num_updates: 0,
            id: ::get_unique_id()
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn num_updates(&self) -> u64 {
        self.num_updates
    }

    pub fn unwrap(self) -> S {
        self.shadable
    }
}

impl<S: Shadable> Deref for Widget<S> {
    type Target = S;

    fn deref(&self) -> &S {
        &self.shadable
    }
}

impl<S: Shadable> DerefMut for Widget<S> {
    fn deref_mut(&mut self) -> &mut S {
        self.num_updates += 1;
        &mut self.shadable
    }
}

impl<S: Shadable> AsRef<S> for Widget<S> {
    fn as_ref(&self) -> &S {
        self
    }
}

impl<S: Shadable> AsMut<S> for Widget<S> {
    fn as_mut(&mut self) -> &mut S {
        self
    }
}

pub trait Shadable {
    fn shader_data(&self, ShaderDataCollector);
}

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

impl Add for Complex {
    type Output = Complex;

    fn add(self, other: Complex) -> Complex {
        Complex {
            rat: self.rat + other.rat,
            pts: self.pts + other.pts
        }
    }
}

impl Sub for Complex {
    type Output = Complex;

    fn sub(self, other: Complex) -> Complex {
        Complex {
            rat: self.rat - other.rat,
            pts: self.pts - other.pts
        }
    }
}

impl Mul for Complex {
    type Output = Complex;

    fn mul(self, other: Complex) -> Complex {
        Complex {
            rat: self.rat * other.rat,
            pts: self.pts * other.pts
        }
    }
}

impl Mul<f32> for Complex {
    type Output = Complex;

    fn mul(self, mult: f32) -> Complex {
        Complex {
            rat: self.rat * mult,
            pts: self.pts * mult
        }
    }
}

impl Div<f32> for Complex {
    type Output = Complex;

    fn div(self, divisor: f32) -> Complex {
        Complex {
            rat: self.rat / divisor,
            pts: self.pts / divisor
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

impl Mul for Point {
    type Output = Point;

    fn mul(self, other: Point) -> Point {
        Point {
            x: self.x * other.x,
            y: self.y * other.y
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
    pub color: Color
}

impl ColorVert {
    #[inline]
    pub fn new(pos: Complex, color: Color) -> ColorVert {
        ColorVert {
            pos: pos,
            color: color
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Upper-left corner of rectangle
    pub lowleft: Complex,
    // Lower-right corner of rectangle
    pub upright: Complex
}

impl Rect {
    pub fn new(lowleft: Complex, upright: Complex) -> Rect {
        Rect {
            lowleft: lowleft,
            upright: upright
        }
    }

    /// Calculate upper-right corner of rectangle
    pub fn lowright(self) -> Complex {
        Complex {
            rat: Point::new(self.upright.rat.x, self.lowleft.rat.y),
            pts: Point::new(self.upright.pts.x, self.lowleft.pts.y)
        }
    }

    /// Calculate lower-left corner of rectangle
    pub fn upleft(self) -> Complex {
        Complex {
            rat: Point::new(self.lowleft.rat.x, self.upright.rat.y),
            pts: Point::new(self.lowleft.pts.x, self.upright.pts.y)
        }
    }

    pub fn center(self) -> Complex {
        Complex {
            rat: (self.lowleft.rat + self.upright.rat) / 2.0,
            pts: (self.lowleft.pts + self.upright.pts) / 2.0
        }
    }

    pub fn width(self) -> LinearComplex {
        self.upright.x() - self.lowleft.x()
    }

    pub fn height(self) -> LinearComplex {
        self.upright.y() - self.lowleft.y()
    }

    pub fn dims(self) -> Complex {
        self.upright - self.lowleft
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

type HasherType = BuildHasherDefault<FnvHasher>;

pub trait Renderer {
    type Processor: DataProcessor;

    fn processor(&mut self) -> Self::Processor;
}

pub trait DataProcessor {
    type DispData: Default;

    fn render_flags(&self) -> RenderFlags;
    fn update_data<S: Shadable>(&mut self, &S, &mut Self::DispData);
    fn render_data(&mut self, &Self::DispData);
}

bitflags! {
    pub flags RenderFlags: u64 {
        const FORCE_UPDATE = 0b1
    }
}

#[derive(Default)]
pub struct Display<R: Renderer> {
    id_map: HashMap<u64, IDMapEntry<<R::Processor as DataProcessor>::DispData>, HasherType>,
    renderer: R
}

impl<R: Renderer> Display<R> {
    pub fn new(renderer: R) -> Display<R> {
        Display {
            id_map: HashMap::default(),
            renderer: renderer
        }
    }

    pub fn dispatcher(&mut self) -> Dispatcher<R::Processor> {
        Dispatcher {
            id_map: &mut self.id_map,
            data_processor: self.renderer.processor()
        }
    }
}


struct IDMapEntry<D> {
    num_updates: u64,
    data: D
}

pub struct Dispatcher<'a, D: 'a + DataProcessor> {
    id_map: &'a mut HashMap<u64, IDMapEntry<D::DispData>, HasherType>,
    data_processor: D
}

impl<'a, D: 'a + DataProcessor> Dispatcher<'a, D> {
    pub fn draw<S: Shadable>(&mut self, widget: &Widget<S>) {
        use std::collections::hash_map::Entry;

        let render_flags = self.data_processor.render_flags();
        {
            // Whether or not to re-upload any data to the GPU buffers
            let update_buffers: bool;
            let id_map_entry_mut: &mut IDMapEntry<D::DispData>;

            match self.id_map.entry(widget.id()) {
                Entry::Occupied(mut entry) => {
                    update_buffers = !(widget.num_updates() == entry.get().num_updates);
                    entry.get_mut().num_updates = widget.num_updates();
                    id_map_entry_mut = entry.into_mut();
                }
                Entry::Vacant(entry)   => {
                    update_buffers = true;
                    id_map_entry_mut = entry.insert(IDMapEntry {
                        num_updates: widget.num_updates(),
                        data: Default::default()
                    });
                }
            }
            
            if render_flags.contains(FORCE_UPDATE) || update_buffers {
                self.data_processor.update_data(widget.as_ref(), &mut id_map_entry_mut.data);
            }
        }

        // Unfortunately, we can't just re-use the mutable reference to the id_map_entry, as we also need
        // to borrow the struct owning the entry as immutable. This workaround has a slight runtime cost,
        // so it's in the program's best interest to have this hack removed.
        let id_map_entry = self.id_map.get(&widget.id()).unwrap();
        self.data_processor.render_data(&id_map_entry.data);
    }
}
