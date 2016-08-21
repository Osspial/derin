use super::{Drawable, Shader, Vertex, Color, Rect};

use cgmath::{Vector2};

use std::cell::UnsafeCell;

pub struct ColorRect {
    pub color: Color,
    pub rect: Rect,
    verts: UnsafeCell<[Vertex; 4]>
}

impl ColorRect {
    pub fn new(color: Color, rect: Rect) -> ColorRect {
        use std::mem;
        ColorRect {
            color: color,
            rect: rect,
            verts: UnsafeCell::new(unsafe{ mem::zeroed() })
        }
    }
}

impl Drawable for ColorRect {
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        // Yes, this is writing to potentially pointed-to data. However, the data being written isn't at
        // all different from the data that would have been in verts anyway, so we can get away with that.
        let verts = &mut unsafe{ *self.verts.get() };
        *verts = [
            Vertex::new(
                self.rect.upleft,
                Vector2::new(-SQRT_2, SQRT_2),
                self.color
            ),
            Vertex::new(
                self.rect.upright(),
                Vector2::new(SQRT_2, SQRT_2),
                self.color
            ),
            Vertex::new(
                self.rect.lowright,
                Vector2::new(SQRT_2, -SQRT_2),
                self.color
            ),
            Vertex::new(
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
}

const SQRT_2: f32 = 0.70710678118;
