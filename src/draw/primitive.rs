use super::{Shadable, ColorVert, Color, Rect, Complex};
use super::font::Font;
use super::gl::ShaderDataCollector;

use cgmath::{Matrix3, Rad};

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
    fn shader_data(&self, mut data: ShaderDataCollector) {
        data.verts_extend_from_slice(&[
            ColorVert::new(
                self.rect.upleft,
                self.color
            ),
            ColorVert::new(
                self.rect.upright(),
                self.color
            ),
            ColorVert::new(
                self.rect.lowright,
                self.color
            ),
            ColorVert::new(
                self.rect.lowleft(),
                self.color
            )
        ]);

        data.indices_extend_from_slice(&[
            [0, 1, 2],
            [2, 3, 0]
        ]);
    }
}

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
    pub angle: f32
}

impl<N> LinearGradient<N>
        where N: AsRef<[GradientNode]> {
    pub fn new(rect: Rect, nodes: N, angle: f32) -> LinearGradient<N> {
        LinearGradient {
            rect: rect,
            nodes: nodes,
            angle: angle
        }
    }
}

impl<N> Shadable for LinearGradient<N>
        where N: AsRef<[GradientNode]> {
    fn shader_data(&self, mut data: ShaderDataCollector) {
        use std::f32::consts::PI;

        let top_color = self.nodes.as_ref()[0].color;
        let bottom_color = self.nodes.as_ref().last().unwrap().color;


        let angle_rad = self.angle * 2.0 * PI / 360.0;

        let angle_modulo = angle_rad % (PI / 2.0);
        let (sin, cos) = (angle_modulo.sin(), angle_modulo.cos());
        let scale = (sin + cos)/sin.hypot(cos);

        let scale_matrix = Matrix3::new(
            scale, 0.0, 0.0,
            0.0, scale, 0.0,
            0.0, 0.0, 1.0
        );

        data.with_transform(self.rect);
        data.with_mask(&[
                Complex::new_rat(-1.0,  1.0),
                Complex::new_rat( 1.0,  1.0),
                Complex::new_rat(-1.0, -1.0),
                Complex::new_rat( 1.0, -1.0)
            ], &[[0, 1, 2], [2, 3, 1]]);
        data.with_matrix(Matrix3::from_angle_z(Rad(angle_rad)) * scale_matrix);

        // Bottom left and right vertices
        data.push_vert(ColorVert {
            pos: Complex::new_rat(-1.0, -1.0),
            color: bottom_color
        });
        data.push_vert(ColorVert {
            pos: Complex::new_rat(1.0, -1.0),
            color: bottom_color
        });
        
        for n in self.nodes.as_ref().iter() {
            data.push_vert(ColorVert {
                pos: Complex::new_rat(-1.0, n.pos),
                color: n.color
            });
            data.push_vert(ColorVert {
                pos: Complex::new_rat(1.0, n.pos),
                color: n.color
            });
        }

        // Top left and right vertices
        data.push_vert(ColorVert {
            pos: Complex::new_rat(-1.0, 1.0),
            color: top_color
        });
        data.push_vert(ColorVert {
            pos: Complex::new_rat(1.0, 1.0),
            color: top_color
        });

        let mut last_pair: Option<(u16, u16)> = None;
        for pair in (0..self.nodes.as_ref().len() + 2).map(|i| (i as u16 * 2, i as u16 * 2 + 1)) {
            if let Some(last_pair) = last_pair {
                data.indices_extend_from_slice(&[
                    [pair.0, pair.1, last_pair.0], 
                    [last_pair.0, last_pair.1, pair.1]
                ]);
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
    fn shader_data(&self, mut data: ShaderDataCollector) {
        data.push_text(self.rect, self.text.as_ref(), self.color, &self.font, self.font_size);
    }
}
