use freetype::{Library, RenderMode, Face, BitmapGlyph};
use freetype::face::{LoadFlag, KerningMode};

use std::path::PathBuf;
use std::collections::HashSet;
use std::iter::FromIterator;
use std::str::Chars;

use super::Point;

#[derive(Debug)]
pub enum FontError {
    NoAtlasImage,
    UnloadedFontSize(u32),
    UnexpectedDpiChange{
        old_dpi: u32,
        new_dpi: u32
    }
}

pub enum FontStyle {
    Regular,
    Italic,
    Bold,
    BoldItalic
}

pub struct FontInfo {
    pub regular: PathBuf,
    pub italic: Option<PathBuf>,
    pub bold: Option<PathBuf>,
    pub bold_italic: Option<PathBuf>
}

thread_local!{
    static FT_LIB: Library = Library::init().unwrap();
}

struct BitmapIter<'a, I: Iterator<Item=char>> {
    face: &'a Face<'static>,
    char_iter: I
}

impl<'a, I: Iterator<Item=char>> BitmapIter<'a, I> {
    fn new(face: &'a Face<'static>, char_iter: I) -> BitmapIter<'a, I> {
        BitmapIter {
            face: face,
            char_iter: char_iter
        }
    }
}

impl<'a, I: Iterator<Item=char>> Iterator for BitmapIter<'a, I> {
    type Item = BitmapGlyph;

    fn next(&mut self) -> Option<BitmapGlyph> {
        if let Some(c) = self.char_iter.next() {
            let char_index = self.face.get_char_index(c as usize);
            self.face.load_glyph(char_index, LoadFlag::empty()).unwrap();
            let glyph = self.face.glyph().get_glyph().unwrap();
            let bitmap = glyph.to_bitmap(RenderMode::Normal, None).unwrap();

            Some(bitmap)
        } else {
            None
        }
    }
}

struct FontFaces {
    regular: Face<'static>,
    italic: Option<Face<'static>>,
    bold: Option<Face<'static>>,
    bold_italic: Option<Face<'static>>
}

pub struct Font {
    faces: FontFaces,
    atlas: FontAtlas,
}

impl Font {
    pub fn new(info: &FontInfo, dpi: u32) -> Font {
        FT_LIB.with(|lib| {
            let regular = lib.new_face(&info.regular, 0).unwrap();
            let italic = info.italic.as_ref().map(|p| lib.new_face(p, 0).unwrap());
            let bold = info.bold.as_ref().map(|p| lib.new_face(p, 0).unwrap());
            let bold_italic = info.bold_italic.as_ref().map(|p| lib.new_face(p, 0).unwrap());

            Font {
                faces: FontFaces {                
                    regular: regular,
                    italic: italic,
                    bold: bold,
                    bold_italic: bold_italic,
                },
                atlas: FontAtlas::new(dpi)
            }
        })
    }

    pub fn add_font_size(&mut self, font_size: u32) {
        if let Err(i) = self.atlas.font_sizes.binary_search(&font_size) {
            self.atlas.font_sizes.insert(i, font_size);
            self.atlas.recalculate_image(&self.faces);
        }
    }

    pub fn atlas_chars<'a, 's>(&'a self, string: &'s str, font_size: u32, dpi: u32) -> Result<AtlasCharIter<'a, 's>, FontError> {
        if self.atlas.pixels.len() == 0 {
            Err(FontError::NoAtlasImage)
        } else if self.atlas.dpi != dpi {
            Err(FontError::UnexpectedDpiChange {
                old_dpi: self.atlas.dpi,
                new_dpi: dpi
            })
        } else if let Err(_) = self.atlas.font_sizes.binary_search(&font_size) {
            Err(FontError::UnloadedFontSize(font_size))
        } else {
            Ok(AtlasCharIter {
                font: self,
                chars: string.chars(),
                last_char: '\0',
                offset: Point::new(0.0, 0.0),
                has_kerning: self.faces.regular.has_kerning(),
                dpi: dpi
            })
        }
    }

    pub fn atlas_image(&self) -> ImageRef {
        ImageRef {
            pixels: &self.atlas.pixels,
            width: self.atlas.width,
            height: self.atlas.height
        }
    }
}

pub struct ImageRef<'a> {
    pub pixels: &'a [u8],
    pub width: u32,
    pub height: u32
}

pub struct FontAtlas {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    charset: HashSet<char>,
    font_sizes: Vec<u32>,
    dpi: u32
}

impl FontAtlas {
    fn new(dpi: u32) -> FontAtlas {
        use std::char;

        FontAtlas {
            pixels: Vec::with_capacity(512 * 512),
            width: 0,
            height: 0,
            charset: HashSet::from_iter((32..127).into_iter().map(|d| char::from_u32(d).unwrap())),
            font_sizes: Vec::with_capacity(4),
            dpi: dpi
        }
    }

    fn recalculate_image(&mut self, faces: &FontFaces) {
        let width: u32 = 512;
        let mut height: u32 = 512;

        let mut bitmaps: Vec<BitmapGlyph> = Vec::with_capacity(128);

        // Load the glyph bitmaps into the bitmap vector
        for size in self.font_sizes.iter().map(|s| *s) {
            faces.regular.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();
            bitmaps.extend(BitmapIter::new(&faces.regular, self.charset.iter().map(|c| *c)));

            if let Some(ref italic) = faces.italic {
                italic.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();
                bitmaps.extend(BitmapIter::new(italic, self.charset.iter().map(|c| *c)));
            }
            if let Some(ref bold) = faces.bold {
                bold.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();
                bitmaps.extend(BitmapIter::new(bold, self.charset.iter().map(|c| *c)));
            }
            if let Some(ref bold_italic) = faces.bold_italic {
                bold_italic.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();
                bitmaps.extend(BitmapIter::new(bold_italic, self.charset.iter().map(|c| *c)));
            }
        }

        // Sort the bitmap vector by descending height
        bitmaps.sort_by_key(|b| -(b.bitmap().rows() as isize));

        // If the currently-allocated pixels vector does not have the capacity for a font atlas with
        // the specified width and height, allocate a new vector. Otherwise, just re-use the current
        // vector.
        let pixels_capacity_min = width as usize * height as usize;
        if self.pixels.capacity() < pixels_capacity_min {
            self.pixels = vec![0; pixels_capacity_min];
        } else {
            unsafe{ self.pixels.set_len(pixels_capacity_min) };
        }

        // A vector of how many pixels in each column have been filled in
        let mut heights: Vec<u32> = vec![0; width as usize];

        // The x pixel location where we're going to be writing to the atlas image.
        let mut cursor: u32 = 0;

        for b in &bitmaps {
            let b = b.bitmap();

            if cursor + b.width() as u32 > width {
                cursor = 0;
            }

            let cursorus = cursor as usize;

            if 0 < b.width() {
                for y in (0..b.rows()).map(|y| y as u32) {
                    let max_height_in_range = heights[cursorus..cursorus + b.width() as usize].iter().max().unwrap();

                    let pixels_index = (y * width + cursor + max_height_in_range * width) as usize;
                    let bitmap_index = (y * b.width() as u32) as usize;

                    if pixels_index + b.width() as usize > self.pixels.len() {
                        self.pixels.reserve((height * width) as usize);
                        height *= 2;
                        unsafe{ self.pixels.set_len((height * width) as usize) };
                    }

                    self.pixels[pixels_index..pixels_index + b.width() as usize]
                        .copy_from_slice(&b.buffer()[bitmap_index..bitmap_index + b.width() as usize]);
                }

                let heights_slice = if cursor + b.width() as u32 > width {
                    &mut heights[cursorus..]
                } else {
                    &mut heights[cursorus..cursorus+ b.width() as usize]
                };

                let new_height = heights_slice.iter().max().unwrap() + b.rows() as u32;
                for h in heights_slice {
                    *h = new_height;
                }
            }

            cursor += b.width() as u32;

            if cursor > width {
                cursor = 0;
            }
        }

        height = *heights.iter().max().unwrap();
        self.pixels.truncate((height * width) as usize);

        self.width = width;
        self.height = height;
    }
}

pub struct AtlasCharIter<'a, 's> {
    font: &'a Font,
    chars: Chars<'s>,
    last_char: char,
    /// The offset from the first character in the string, stored as 1/64ths of a pixel. This value is
    /// converted to points upon return from the `next` function down below.
    offset: Point,
    has_kerning: bool,
    dpi: u32
}

impl<'a, 's> Iterator for AtlasCharIter<'a, 's> {
    type Item = AtlasChar;

    fn next(&mut self) -> Option<AtlasChar> {
        if let Some(c) = self.chars.next() {
            let last_char_index = self.font.faces.regular.get_char_index(self.last_char as usize);
            let char_index = self.font.faces.regular.get_char_index(c as usize);
            self.font.faces.regular.load_glyph(char_index, LoadFlag::empty()).expect("Could not load glyph");
            let dpi_scale = 1.0 / (self.dpi as f32 / 72.0);

            if self.has_kerning {
                let kerning = self.font.faces.regular
                    .get_kerning(last_char_index, char_index, KerningMode::KerningDefault)
                    .expect("Failed to get font kerning");

                self.offset.x += kerning.x as f32;
                self.offset.y += kerning.y as f32;
            }

            let advance = self.font.faces.regular.glyph().advance();
            self.offset.x += advance.x as f32;
            self.offset.y += advance.y as f32;

            Some(AtlasChar{
                character: c,
                offset: (self.offset / 64.0) * dpi_scale
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct AtlasChar {
    pub character: char,
    // pub rect: ImageRect,
    /// The number of points on the xy plane offset from the first character in the string
    pub offset: Point
}

pub struct ImageRect {
    pub upleft: Point,
    pub lowright: Point
}
