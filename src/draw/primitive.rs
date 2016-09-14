use super::{Shadable, Shader, ColorVert, Color, Rect, LinearComplex};
use super::font::Font;

use cgmath::{Vector2};

use std::cell::{Cell, UnsafeCell};
use std::hash::{Hash, Hasher};

use fnv::FnvHasher;

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
        let mut hasher = FnvHasher::default();
        text.as_ref().hash(&mut hasher);

        TextBox {
            rect: rect,
            text: text,
            color: color,
            font: font,
            font_size: font_size,
            
            num_updates: Cell::new(0),
            old_rect: Cell::new(rect),
            old_str_hash: Cell::new(hasher.finish()),
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
        let mut hasher = FnvHasher::default();
        self.text.as_ref().hash(&mut hasher);
        let str_hash = hasher.finish();

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
