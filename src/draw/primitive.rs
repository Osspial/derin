use super::{Shadable, Shader, ColorVert, Color, Rect, Complex, LinearComplex};
use super::font::Font;

use cgmath::{Vector2};

use std::cell::{Cell, UnsafeCell};
use std::hash::{Hash, Hasher};

use twox_hash::XxHash;

pub struct ColorRect {
    pub color: Color,
    pub rect: Rect,
    num_updates: Cell<u64>,
    old_color: Cell<Color>,
    old_rect: Cell<Rect>,
    verts: UnsafeCell<[ColorVert; 4]>
}

impl ColorRect {
    pub fn new(color: Color, rect: Rect) -> ColorRect {
        use std::mem;
        ColorRect {
            color: color,
            rect: rect,
            num_updates: Cell::new(0),
            old_color: Cell::new(color),
            old_rect: Cell::new(rect),
            verts: UnsafeCell::new(unsafe{ mem::zeroed() })
        }
    }
}

impl Shadable for ColorRect {
    type Composite = ();
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        // Yes, this is writing to potentially pointed-to data. However, the data being written isn't at
        // all different from the data that would have been in verts anyway, so we can get away with that.
        let verts = unsafe{ &mut *self.verts.get() };
        *verts = [
            ColorVert::new(
                self.rect.upleft,
                Vector2::new(-SQRT_2, SQRT_2),
                self.color
            ),
            ColorVert::new(
                self.rect.upright(),
                Vector2::new(SQRT_2, SQRT_2),
                self.color
            ),
            ColorVert::new(
                self.rect.lowright,
                Vector2::new(SQRT_2, -SQRT_2),
                self.color
            ),
            ColorVert::new(
                self.rect.lowleft(),
                Vector2::new(-SQRT_2, -SQRT_2),
                self.color
            )
        ];

        const INDICES: &'static [u16] = 
            &[0, 1, 2,
              2, 3, 0];

        Shader::Verts {
            verts: unsafe{ &*self.verts.get() },
            indices: INDICES
        }
    }

    fn num_updates(&self) -> u64 {
        if self.old_color.get() != self.color ||
           self.old_rect.get() != self.rect {
            self.num_updates.set(self.num_updates.get() + 1);
            self.old_color.set(self.color);
            self.old_rect.set(self.rect);
        }

        self.num_updates.get()
    }
}

impl<'b> Shadable for &'b ColorRect {
    type Composite = ();
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        (*self).shader_data()
    }

    fn num_updates(&self) -> u64 {
        (*self).num_updates()
    }
}

const SQRT_2: f32 = 0.70710678118;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GradientNode {
    pub pos: f32,
    pub color: Color
}

impl GradientNode {
    pub fn new(pos: f32, color: Color) -> GradientNode {
        GradientNode {
            pos: pos,
            color: color
        }
    }
}

impl Hash for GradientNode {
    fn hash<H>(&self, state: &mut H) 
            where H: Hasher {
        use std::slice;
        use std::mem;

        let pos_bytes = unsafe{ slice::from_raw_parts(&self.pos as *const _ as *const u8, mem::size_of::<f32>()) };
        state.write(pos_bytes);
        self.color.hash(state);
    }
}

pub struct LinearGradient<N> 
        where N: AsRef<[GradientNode]> {
    pub rect: Rect,
    pub nodes: N,

    num_updates: Cell<u64>,
    old_rect: Cell<Rect>,
    old_nodes_hash: Cell<u64>,
    verts: UnsafeCell<Vec<ColorVert>>,
    indices: UnsafeCell<Vec<u16>>
}

impl<N> LinearGradient<N>
        where N: AsRef<[GradientNode]> {
    pub fn new(rect: Rect, nodes: N) -> LinearGradient<N> {
        let mut hasher = XxHash::default();
        nodes.as_ref().hash(&mut hasher);

        LinearGradient {
            rect: rect,
            nodes: nodes,

            num_updates: Cell::new(0),
            old_rect: Cell::new(rect),
            old_nodes_hash: Cell::new(hasher.finish()),
            verts: UnsafeCell::new(Vec::with_capacity(8)),
            indices: UnsafeCell::new(Vec::with_capacity(16))
        }
    }
}

impl<N> Shadable for LinearGradient<N>
        where N: AsRef<[GradientNode]> {
    type Composite = ();
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        let verts = unsafe{ &mut *self.verts.get() };
        let indices = unsafe {&mut *self.indices.get() };
        verts.clear();
        indices.clear();

        for n in self.nodes.as_ref().iter() {
            let pos = LinearComplex::new_rat(
                n.pos * self.rect.height().rat / 2.0,
            );

            verts.push(ColorVert {
                pos: Complex::from_linears(self.rect.upleft.x(), pos),
                // TODO: Add proper normal calculation
                normal: Vector2::new(0.0, 0.0),
                color: n.color
            });
            verts.push(ColorVert {
                pos: Complex::from_linears(self.rect.lowright.x(), pos),
                // Ditto.
                normal: Vector2::new(0.0, 0.0),
                color: n.color
            });
        }

        let top_color = verts[0].color;
        let bottom_color = verts.last().unwrap().color;

        // Top left and right vertices
        verts.insert(0, ColorVert {
            pos: self.rect.upleft,
            normal: Vector2::new(0.0, 0.0),
            color: top_color
        });
        verts.insert(1, ColorVert {
            pos: self.rect.upright(),
            normal: Vector2::new(0.0, 0.0),
            color: top_color
        });

        // Bottom left and right vertices
        verts.push(ColorVert {
            pos: self.rect.lowleft(),
            normal: Vector2::new(0.0, 0.0),
            color: bottom_color
        });
        verts.push(ColorVert {
            pos: self.rect.lowright,
            normal: Vector2::new(0.0, 0.0),
            color: bottom_color
        });

        let mut last_pair: Option<(u16, u16)> = None;
        for pair in (0..verts.len() / 2).map(|i| (i as u16 * 2, i as u16 * 2 + 1)) {
            if let Some(last_pair) = last_pair {
                indices.extend_from_slice(&[pair.0, pair.1, last_pair.0, last_pair.0, last_pair.1, pair.1]);
            }

            last_pair = Some(pair);
        }

        Shader::Verts {
            verts: unsafe{ &*self.verts.get() },
            indices: unsafe{ &*self.indices.get() }
        }
    }

    fn num_updates(&self) -> u64 {
        let mut hasher = XxHash::default();
        self.nodes.as_ref().hash(&mut hasher);
        let nodes_hash = hasher.finish();

        if self.old_nodes_hash.get() != nodes_hash ||
           self.old_rect.get() != self.rect {
            self.old_nodes_hash.set(nodes_hash);
            self.old_rect.set(self.rect);
            self.num_updates.set(self.num_updates.get() + 1);
        }

        self.num_updates.get()
    }
}

pub struct TextBox<S: AsRef<str>> {
    pub rect: Rect,
    pub text: S,
    pub color: Color,
    pub font: Font,
    pub font_size: u32,

    num_updates: Cell<u64>,
    old_rect: Cell<Rect>,
    old_str_hash: Cell<u64>,
    old_color: Cell<Color>
}

impl<S: AsRef<str>> TextBox<S> {
    pub fn new(rect: Rect, text: S, color: Color, font: Font, font_size: u32) -> TextBox<S> {
        let mut string_hasher = XxHash::default();
        text.as_ref().hash(&mut string_hasher);

        TextBox {
            rect: rect,
            text: text,
            color: color,
            font: font,
            font_size: font_size,
            
            num_updates: Cell::new(0),
            old_rect: Cell::new(rect),
            old_str_hash: Cell::new(string_hasher.finish()),
            old_color: Cell::new(color)
        }
    }
}

impl<S: AsRef<str>> Shadable for TextBox<S> {
    type Composite = ();
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        Shader::Text {
            rect: self.rect,
            text: self.text.as_ref(),
            color: self.color,
            font: &self.font,
            font_size: self.font_size
        }
    }

    fn num_updates(&self) -> u64 {
        // We're using xxHash for the strings, as it is faster than FNV for larger bytecounts.
        let mut string_hasher = XxHash::default();
        self.text.as_ref().hash(&mut string_hasher);
        let str_hash = string_hasher.finish();

        if str_hash != self.old_str_hash.get() ||
           self.rect != self.old_rect.get() || 
           self.color != self.old_color.get() {
            self.num_updates.set(self.num_updates.get() + 1);
            self.old_rect.set(self.rect);
            self.old_str_hash.set(str_hash);
            self.old_color.set(self.color);
        }

        self.num_updates.get()
    }
}

impl<'b, S: AsRef<str>> Shadable for &'b TextBox<S> {
    type Composite = ();
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        (*self).shader_data()
    }

    fn num_updates(&self) -> u64 {
        (*self).num_updates()
    }
}
