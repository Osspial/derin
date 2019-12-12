use cgmath_geometry::{
    D2,
    cgmath::Point2,
    rect::BoundBox,
};
use crate::rect_layout::{
    Rect, RectFill, ImageRectFill,
    theme::{Color, ImageId},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureVertex {
    pub position: Point2<f32>,
    pub texture_coordinate: Point2<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorVertex {
    pub position: Point2<f32>,
    pub color: Color,
}

/// A rect comprised of individual vertices.
///
/// Vertices are laid out as follows:
/// ```text
/// 0---1
/// |   |
/// 2---3
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VertexRect {
    Color(ColorVertexRect),
    Texture(TextureVertexRect),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorVertexRect([ColorVertex; 4]);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureVertexRect {
    pub image_id: ImageId,
    pub vertices: [TextureVertex; 4]
}

// 0-1
// |/|
// 2-3
pub const INDICES_CLOCKWISE: [u16; 6] = [0, 1, 2, 3, 2, 1];
pub const INDICES_COUNTERCLOCKWISE: [u16; 6] = [0, 2, 1, 3, 1, 2];

pub fn indices_clockwise_offset(offset: u16) -> [u16; 6] {
    array_offset(offset, INDICES_CLOCKWISE)
}

pub fn indices_counterclockwise_offset(offset: u16) -> [u16; 6] {
    array_offset(offset, INDICES_COUNTERCLOCKWISE)
}

impl ColorVertexRect {
    pub const INDICES_CLOCKWISE: [u16; 6] = INDICES_CLOCKWISE;
    pub const INDICES_COUNTERCLOCKWISE: [u16; 6] = INDICES_COUNTERCLOCKWISE;

    pub fn new(rect: BoundBox<D2, f32>, color: Color) -> ColorVertexRect {
        let rect_points = [
            Point2::new(rect.min.x, rect.min.y),
            Point2::new(rect.max.x, rect.min.y),
            Point2::new(rect.min.x, rect.max.y),
            Point2::new(rect.max.x, rect.max.y),
        ];

        ColorVertexRect([
            ColorVertex::new(rect_points[0], color),
            ColorVertex::new(rect_points[1], color),
            ColorVertex::new(rect_points[2], color),
            ColorVertex::new(rect_points[3], color),
        ])
    }

    pub fn indices_clockwise_offset(offset: u16) -> [u16; 6] {
        indices_clockwise_offset(offset)
    }

    pub fn indices_counterclockwise_offset(offset: u16) -> [u16; 6] {
        indices_counterclockwise_offset(offset)
    }

    pub fn map_to_array<V>(self, mut map: impl FnMut(ColorVertex) -> V) -> [V; 4] {
        [
            map(self.0[0]),
            map(self.0[1]),
            map(self.0[2]),
            map(self.0[3]),
        ]
    }
}

impl TextureVertexRect {
    pub const INDICES_CLOCKWISE: [u16; 6] = INDICES_CLOCKWISE;
    pub const INDICES_COUNTERCLOCKWISE: [u16; 6] = INDICES_COUNTERCLOCKWISE;

    pub fn new(rect: BoundBox<D2, f32>, fill: ImageRectFill) -> TextureVertexRect {
        let ImageRectFill{ image_id, subrect } = fill;

        let rect_points = [
            Point2::new(rect.min.x, rect.min.y),
            Point2::new(rect.max.x, rect.min.y),
            Point2::new(rect.min.x, rect.max.y),
            Point2::new(rect.max.x, rect.max.y),
        ];
        let texture_points = [
            Point2::new(subrect.min.x, subrect.min.y),
            Point2::new(subrect.max.x, subrect.min.y),
            Point2::new(subrect.min.x, subrect.max.y),
            Point2::new(subrect.max.x, subrect.max.y),
        ];

        TextureVertexRect {
            image_id,
            vertices: [
                TextureVertex::new(rect_points[0], texture_points[0]),
                TextureVertex::new(rect_points[1], texture_points[1]),
                TextureVertex::new(rect_points[2], texture_points[2]),
                TextureVertex::new(rect_points[3], texture_points[3]),
            ],
        }
    }

    pub fn indices_clockwise_offset(offset: u16) -> [u16; 6] {
        indices_clockwise_offset(offset)
    }

    pub fn indices_counterclockwise_offset(offset: u16) -> [u16; 6] {
        indices_counterclockwise_offset(offset)
    }

    pub fn map_to_array<V>(self, mut map: impl FnMut(TextureVertex, ImageId) -> V) -> [V; 4] {
        [
            map(self.vertices[1], self.image_id),
            map(self.vertices[0], self.image_id),
            map(self.vertices[2], self.image_id),
            map(self.vertices[3], self.image_id),
        ]
    }
}

impl VertexRect {
    pub const INDICES_CLOCKWISE: [u16; 6] = INDICES_CLOCKWISE;
    pub const INDICES_COUNTERCLOCKWISE: [u16; 6] = INDICES_COUNTERCLOCKWISE;

    pub fn new(rect: BoundBox<D2, f32>, fill: RectFill) -> VertexRect {
        match fill {
            RectFill::Color(color) => VertexRect::Color(ColorVertexRect::new(rect, color)),
            RectFill::Image(fill) => VertexRect::Texture(TextureVertexRect::new(rect, fill)),
        }
    }

    pub fn indices_clockwise_offset(offset: u16) -> [u16; 6] {
        indices_clockwise_offset(offset)
    }

    pub fn indices_counterclockwise_offset(offset: u16) -> [u16; 6] {
        indices_counterclockwise_offset(offset)
    }

    pub fn image_id(self) -> Option<ImageId> {
        match self {
            VertexRect::Color(_) => None,
            VertexRect::Texture(TextureVertexRect {image_id, ..}) => Some(image_id),
        }
    }

    pub fn map_color_to_array<V>(self, map_color: impl FnMut(ColorVertex) -> V) -> Result<[V; 4], VertexRect> {
        match self {
            VertexRect::Color(color_verts) => Ok(color_verts.map_to_array(map_color)),
            VertexRect::Texture{..} => Err(self)
        }
    }

    pub fn map_texture_to_array<V>(self, map_texture: impl FnMut(TextureVertex, ImageId) -> V) -> Result<[V; 4], VertexRect> {
        match self {
            VertexRect::Color(_) => Err(self),
            VertexRect::Texture(tvr) => Ok(tvr.map_to_array(map_texture)),
        }
    }

    pub fn map_unify_to_array<V>(
        self,
        map_color: impl FnMut(ColorVertex) -> V,
        map_texture: impl FnMut(TextureVertex, ImageId) -> V,
    ) -> [V; 4] {
        match self {
            VertexRect::Color(color_verts) => color_verts.map_to_array(map_color),
            VertexRect::Texture(tvr) => tvr.map_to_array(map_texture),
        }
    }
}

impl TextureVertex {
    pub fn new(position: Point2<f32>, texture_coordinate: Point2<i32>) -> TextureVertex {
        TextureVertex{ position, texture_coordinate }
    }
}

impl ColorVertex {
    pub fn new(position: Point2<f32>, color: Color) -> ColorVertex {
        ColorVertex{ position, color }
    }
}

impl From<Rect> for VertexRect {
    fn from(rect: Rect) -> VertexRect {
        VertexRect::new(rect.rect.cast().unwrap(), rect.fill)
    }
}

#[inline(always)]
fn array_offset(offset: u16, mut array: [u16; 6]) -> [u16; 6] {
    for i in &mut array {
        *i += offset;
    }

    array
}
