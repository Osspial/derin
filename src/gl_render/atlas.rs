use std::cmp;
// use std::collections::HashMap;

use cgmath::Vector2;
use cgmath_geometry::{Rectangle, OffsetRect, DimsRect};

use gl_raii::glsl::Nu8;
use gl_raii::colors::Rgba;

use dat::SkylineAtlas;

use theme::ThemeText;

pub struct Atlas {
    atlas: SkylineAtlas<Rgba<Nu8>>,
    // image_rects: hashmap,
    // glyph_rects: hashmap
}

impl Atlas {
    pub fn new() -> Atlas {
        Atlas {
            atlas: SkylineAtlas::new(Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(0)), DimsRect::new(1024, 1024)),
            // image_rects: (),
            // glyph_rects: ()
        }
    }

    pub fn dims(&self) -> DimsRect<u32> {
        self.atlas.dims()
    }

    pub fn pixels(&self) -> &[Rgba<Nu8>] {
        self.atlas.pixels()
    }

    /// Tell the atlas that a new frame has begun. This can be used to tell how old an image is, and
    /// to throw away pixel data that's been unused for a while.
    pub fn bump_frame_count(&mut self) {
        self.atlas.clear(None);
        // self.image_rects.clear();
        // self.glyph_rects.clear();
    }

    /// Retrieve an image from the atlas. `image_path` refers to the theme's name for the image,
    /// while `get_image` is used to add the image to the atlas in case it's not already stored.
    pub fn image_rect<'a, F>(&mut self, image_path: &str, get_image: F) -> OffsetRect<u32>
        where F: 'a + FnOnce(&str) -> (&'a [Rgba<Nu8>], DimsRect<u32>)
    {
        let (pixels, dims) = get_image(image_path);
        match self.atlas.add_image(dims, dims.into(), pixels) {
            Some(rect) => rect,
            None => {
                let new_width = cmp::max(dims.width(), self.atlas.dims().width());
                let new_height = self.atlas.dims().height() + cmp::max(self.atlas.dims().height(), dims.height());
                self.atlas.set_dims(
                    Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(0)),
                    DimsRect::new(new_width, new_height)
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
    pub fn glyph_rect<'a, F, I, J>(&mut self, style: &'a ThemeText, glyph_index: u32, get_glyph: F) -> (OffsetRect<u32>, Vector2<i32>)
        where F: FnOnce(&'a ThemeText, u32) -> (I, DimsRect<u32>, Vector2<i32>),
              I: 'a + IntoIterator<Item=J>,
              J: 'a + IntoIterator<Item=Rgba<Nu8>>
    {
        let (pixels, dims, bearing) = get_glyph(style, glyph_index);
        match self.atlas.add_image_pixels(dims, pixels) {
            Ok(rect) => (rect, bearing),
            Err(pixels) => {
                let new_width = cmp::max(dims.width(), self.atlas.dims().width());
                let new_height = self.atlas.dims().height() + cmp::max(self.atlas.dims().height(), dims.height());
                self.atlas.set_dims(
                    Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(0)),
                    DimsRect::new(new_width, new_height)
                );

                (self.atlas.add_image_pixels(dims, pixels).unwrap_or_else(|_| panic!("bad resize")), bearing)
            }
        }
    }
}
