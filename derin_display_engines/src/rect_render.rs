// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod theme;
mod image_slice;
pub mod text;

use cgmath_geometry::{D2, rect::BoundBox};
use crate::EditString;
use theme::{Color, ImageId, WidgetStyle};

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
        subrect: BoundBox<D2, i32>,
    },
}

pub struct RectLayout {
}

impl RectLayout {
    pub fn new(theme: WidgetStyle, text: Option<EditString>) -> RectLayout {
        unimplemented!()
    }
}

impl Iterator for RectLayout {
    type Item = Rect;

    fn next(&mut self) -> Option<Rect> {
        unimplemented!()
    }
}
