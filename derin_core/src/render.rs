use cgmath::{Vector2, Point2};

pub struct DVertex {
    origin_offset: Vector2<f32>,
    pixel_loc: Point2<i16>,
    tex_uv: Point2<u32>,
    tex_tint: [u8; 3]
}
