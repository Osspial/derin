use super::{Shadable, ColorVert, Color, Rect, Complex};
use super::font::Font;
use super::gl::ShaderDataCollector;

use cgmath::{Vector2};

use std::hash::{Hash, Hasher};

pub struct ColorRect {
    pub color: Color,
    pub rect: Rect
}

impl ColorRect {
    pub fn new(color: Color, rect: Rect) -> ColorRect {
        ColorRect {
            color: color,
            rect: rect,
        }
    }
}

impl Shadable for ColorRect {
    fn shader_data(&self, data: &mut ShaderDataCollector) {
        data.verts_extend_from_slice(&[
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
        ]);

        data.indices_extend_from_slice(&[
            0, 1, 2,
            2, 3, 0
        ]);
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
    pub nodes: N
}

impl<N> LinearGradient<N>
        where N: AsRef<[GradientNode]> {
    pub fn new(rect: Rect, nodes: N) -> LinearGradient<N> {
        LinearGradient {
            rect: rect,
            nodes: nodes
        }
    }
}

impl<N> Shadable for LinearGradient<N>
        where N: AsRef<[GradientNode]> {
    fn shader_data(&self, data: &mut ShaderDataCollector) {
        let top_color = self.nodes.as_ref()[0].color;
        let bottom_color = self.nodes.as_ref().last().unwrap().color;

        let mut data_trans = data.push_transform(self.rect);

        // Bottom left and right vertices
        data_trans.push_vert(ColorVert {
            pos: Complex::new_rat(-1.0, -1.0),
            normal: Vector2::new(0.0, 0.0),
            color: bottom_color
        });
        data_trans.push_vert(ColorVert {
            pos: Complex::new_rat(1.0, -1.0),
            normal: Vector2::new(0.0, 0.0),
            color: bottom_color
        });
        
        for n in self.nodes.as_ref().iter() {
            data_trans.push_vert(ColorVert {
                pos: Complex::new_rat(-1.0, n.pos),
                // TODO: Add proper normal calculation
                normal: Vector2::new(0.0, 0.0),
                color: n.color
            });
            data_trans.push_vert(ColorVert {
                pos: Complex::new_rat(1.0, n.pos),
                // Ditto.
                normal: Vector2::new(0.0, 0.0),
                color: n.color
            });
        }

        // Top left and right vertices
        data_trans.push_vert(ColorVert {
            pos: Complex::new_rat(-1.0, 1.0),
            normal: Vector2::new(0.0, 0.0),
            color: top_color
        });
        data_trans.push_vert(ColorVert {
            pos: Complex::new_rat(1.0, 1.0),
            normal: Vector2::new(0.0, 0.0),
            color: top_color
        });

        let mut last_pair: Option<(u16, u16)> = None;
        for pair in (0..self.nodes.as_ref().len() + 2).map(|i| (i as u16 * 2, i as u16 * 2 + 1)) {
            if let Some(last_pair) = last_pair {
                data_trans.indices_extend_from_slice(&[pair.0, pair.1, last_pair.0, last_pair.0, last_pair.1, pair.1]);
            }

            last_pair = Some(pair);
        }
    }
}

pub struct TextBox<S: AsRef<str>> {
    pub rect: Rect,
    pub text: S,
    pub color: Color,
    pub font: Font,
    pub font_size: u32,
}

impl<S: AsRef<str>> TextBox<S> {
    pub fn new(rect: Rect, text: S, color: Color, font: Font, font_size: u32) -> TextBox<S> {
        TextBox {
            rect: rect,
            text: text,
            color: color,
            font: font,
            font_size: font_size,
        }
    }
}

impl<S: AsRef<str>> Shadable for TextBox<S> {
    fn shader_data(&self, data: &mut ShaderDataCollector) {
        data.push_text(self.rect, self.text.as_ref(), self.color, &self.font, self.font_size);
    }
}
