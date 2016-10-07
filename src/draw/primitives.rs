use super::{Shadable, ColorVert, Color, Rect, Complex};
use super::font::Font;
use super::gl::ShaderDataCollector;

use cgmath::{Matrix3, Rad};

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
                self.rect.upleft(),
                self.color
            ),
            ColorVert::new(
                self.rect.upright,
                self.color
            ),
            ColorVert::new(
                self.rect.lowright(),
                self.color
            ),
            ColorVert::new(
                self.rect.lowleft,
                self.color
            )
        ]);

        data.indices_extend_from_slice(&[
            [0, 1, 2],
            [2, 3, 0]
        ]);
    }
}

pub struct ColorEllipse {
    pub color: Color,
    pub rect: Rect,
    pub subdivs: Option<u16>
}

impl Shadable for ColorEllipse {
    fn shader_data(&self, mut data: ShaderDataCollector) {
        use std::f32::consts::PI;

        data.with_rect(self.rect);

        // Push the initial point, which we will build a triangle fan off of
        data.push_vert(ColorVert::new(
            Complex::new_rat(1.0, 0.0),
            self.color
        ));

        let subdivs = self.subdivs.unwrap_or(32);

        for (i, angle) in (1..subdivs).map(|a| a as f32 / subdivs as f32 * 2.0 * PI).enumerate() {
            let i = i as u16;

            data.push_vert(ColorVert::new(
                Complex::new_rat(angle.cos(), angle.sin()),
                self.color
            ));

            data.push_indices([0, i + 1, (i + 2) % subdivs]);
        }
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

        let top_color = self.nodes.as_ref().last().unwrap().color;
        let bottom_color = self.nodes.as_ref()[0].color;

        let angle_rad = self.angle * 2.0 * PI / 360.0;

        let angle_modulo = angle_rad % (PI / 2.0);
        let (sin, cos) = (angle_modulo.sin(), angle_modulo.cos());
        let scale = (sin + cos)/sin.hypot(cos);

        let scale_matrix = Matrix3::new(
            scale, 0.0, 0.0,
            0.0, scale, 0.0,
            0.0, 0.0, 1.0
        );

        data.with_rect(self.rect);
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

pub struct RadialGradient<N>
        where N: AsRef<[GradientNode]> {
    pub rect: Rect,
    pub nodes: N,
    pub ellipse_rect: Rect
}

impl<N> Shadable for RadialGradient<N>
        where N: AsRef<[GradientNode]> {
    fn shader_data(&self, mut data: ShaderDataCollector) {
        use std::f32::consts::PI;

        let nodes = self.nodes.as_ref();

        data.with_rect(self.rect);
        data.with_mask(&[
                Complex::new_rat(-1.0,  1.0),
                Complex::new_rat( 1.0,  1.0),
                Complex::new_rat(-1.0, -1.0),
                Complex::new_rat( 1.0, -1.0)
            ], &[[0, 1, 2], [2, 3, 1]]);
        // Draw the background rectangle
        ColorRect::new(nodes.last().unwrap().color, Default::default()).shader_data(data.take());

        let circle_resolution = 32;
        let offset_modulo = circle_resolution * 2;

        let offset_increment = nodes.len() as u16 - 1;

        const ELLIPSE_OFFSET: u16 = 1;

        let origin = self.ellipse_rect.center();

        // Push the center vertex
        data.push_vert(ColorVert::new(
            origin,
            nodes[0].color
        ));
        
        for (i, angle) in (0..circle_resolution).map(|i| (i as f32 / circle_resolution as f32) * 2.0 * PI).enumerate() {
            let i = i as u16;

            let ray_complex = Complex::new_rat(angle.cos(), angle.sin()) * self.ellipse_rect.dims() / 2.0;

            // The first node ellipse doesn't need multiple triangles per division, so we just push one
            // triangle for each division.
            data.push_vert(ColorVert::new(
                ray_complex * nodes[1].pos + origin,
                nodes[1].color
            ));

            data.push_indices([0, offset_increment * i + ELLIPSE_OFFSET, offset_increment * (i + 1) % offset_modulo + ELLIPSE_OFFSET]);

            // All other nodes need two triangles, so those are done here. Technically, they could be done with one
            // triangle but that would increase complexity (by requiring that those nodes be drawn first) and would
            // have increased overdraw, compared to the current implementation with zero overdraw in the ellipse.
            for (j, n) in nodes[2..].iter().enumerate() {
                let j = j as u16;

                data.push_vert(ColorVert::new(
                        ray_complex * n.pos + origin,
                        n.color
                    ));

                let node_offset = offset_increment * i + j + ELLIPSE_OFFSET;
                let next_node_offset = (node_offset + offset_increment) % offset_modulo;
                data.indices_extend_from_slice(&[
                    [node_offset, node_offset + 1, next_node_offset],
                    [next_node_offset, next_node_offset + 1, node_offset + 1]
                ]);
            }
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
