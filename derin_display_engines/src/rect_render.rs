pub mod theme;
mod image_slice;

use cgmath_geometry::{D2, rect::BoundBox};
use crate::EditString;
use theme::{Color, ImageId, ThemeWidget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub rect: BoundBox<D2, i32>,
    pub fill: RectFill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RectFill {
    Color(Color),
    Image {
        image_id: ImageId,
        rect: BoundBox<D2, i32>,
    },
}

pub struct RectLayout {
}

impl RectLayout {
    pub fn new(theme: ThemeWidget, text: Option<EditString>) -> RectLayout {
        unimplemented!()
    }
}

impl Iterator for RectLayout {
    type Item = Rect;

    fn next(&mut self) -> Option<Rect> {
        unimplemented!()
    }
}
