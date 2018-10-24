// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cmp;
use std::collections::HashMap;

use crate::cgmath::{Point2, Vector2};
use cgmath_geometry::{D2, rect::{OffsetBox, DimsBox, GeoBox}};

use gullery::image_format::Rgba;

use derin_atlas::SkylineAtlas;

use crate::theme::ThemeFace;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GlyphKey {
    face_fingerprint: u64,
    size: u32,
    glyph_index: u32
}

pub struct Atlas {
    atlas: SkylineAtlas<Rgba<u8>>,
    white_rect: Option<OffsetBox<D2, u32>>,
    // image_rects: HashMap<(), OffsetBox<D2, u32>>,
    glyph_rects: HashMap<GlyphKey, (OffsetBox<D2, u32>, Vector2<i32>)>,
    // image_rects: hashmap,
    // glyph_rects: hashmap
}

impl Atlas {
    pub fn new() -> Atlas {
        Atlas {
            atlas: SkylineAtlas::new(Rgba::new(0, 0, 0, 0), DimsBox::new2(1024, 1024)),
            white_rect: None,
            // image_rects: HashMap::new(),
            glyph_rects: HashMap::new()
        }
    }

    pub fn dims(&self) -> DimsBox<D2, u32> {
        self.atlas.dims()
    }

    pub fn pixels(&self) -> &[Rgba<u8>] {
        self.atlas.pixels()
    }

    /// Tell the atlas that a new frame has begun. This can be used to tell how old an image is, and
    /// to throw away pixel data that's been unused for a while.
    pub fn bump_frame_count(&mut self) {
        self.atlas.clear(None);
        self.white_rect = None;
        // self.image_rects.clear();
        self.glyph_rects.clear();
    }

    pub fn white(&mut self) -> OffsetBox<D2, u32> {
        let white_pic = (
            &[Rgba::new(255, 255, 255, 255)][..],
            DimsBox::new2(1, 1)
        );
        self.white_rect.unwrap_or_else(|| self.image_rect("TODO: REPLACE WHEN STRINGS MATTER", || white_pic))
    }

    /// Retrieve an image from the atlas. `image_path` refers to the theme's name for the image,
    /// while `get_image` is used to add the image to the atlas in case it's not already stored.
    pub fn image_rect<'a, F>(&mut self, _image_path: &str, get_image: F) -> OffsetBox<D2, u32>
        where F: FnOnce() -> (&'a [Rgba<u8>], DimsBox<D2, u32>)
    {
        let (pixels, dims) = get_image();
        match self.atlas.add_image(dims, dims.into(), pixels) {
            Some(rect) => rect,
            None => {
                let new_width = cmp::max(dims.width(), self.atlas.dims().width());
                let new_height = self.atlas.dims().height() + cmp::max(self.atlas.dims().height(), dims.height());
                self.atlas.set_dims(
                    Rgba::new(0, 0, 0, 0),
                    DimsBox::new2(new_width, new_height)
                );

                self.atlas.add_image(dims, dims.into(), pixels).unwrap()
            }
        }
    }

    /// Retrieve a glyph and it's bearing from the atlas. `style` and `glyph_index` are used as keys for
    /// the glyph, while `get_glyph` is used to add the glyph to the atlas in case it's not already stored
    /// within the atlas.
    ///
    /// `get_glyph` returns `(pixel_buf, image_dims, glyph_bearing)`
    pub fn glyph_rect<'a, F, I, J>(&mut self, face: ThemeFace, face_size: u32, glyph_index: u32, get_glyph: F) -> (OffsetBox<D2, u32>, Vector2<i32>)
        where F: FnOnce() -> (I, DimsBox<D2, u32>, Vector2<i32>),
              I: 'a + IntoIterator<Item=J>,
              J: 'a + IntoIterator<Item=Rgba<u8>>
    {
        let key = GlyphKey {
            face_fingerprint: face.fingerprint(),
            size: face_size,
            glyph_index
        };

        let Atlas {
            ref mut glyph_rects,
            ref mut atlas,
            ..
        } = *self;
        *glyph_rects.entry(key).or_insert_with(|| {
            let (pixels, dims, bearing) = get_glyph();
            match atlas.add_image_pixels(dims, pixels) {
                Ok(rect) => (rect, bearing),
                Err(pixels) => {
                    let new_width = cmp::max(dims.width(), atlas.dims().width());
                    let new_height = atlas.dims().height() + cmp::max(atlas.dims().height(), dims.height());
                    atlas.set_dims(
                        Rgba::new(0, 0, 0, 0),
                        DimsBox::new2(new_width, new_height)
                    );

                    (atlas.add_image_pixels(dims, pixels).unwrap_or_else(|_| panic!("bad resize")), bearing)
                }
            }
        })
    }
}
