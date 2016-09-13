use freetype::{Library, RenderMode, Face, BitmapGlyph};
use freetype::face::{LoadFlag, KerningMode};

use cgmath::Vector2;

use std::path::PathBuf;
use std::collections::HashMap;
use std::str::Chars;
use std::rc::Rc;
use std::cell::RefCell;

use super::Point;
use super::gl::get_unique_id;

pub struct FontInfo {
    pub regular: PathBuf,
    pub italic: Option<PathBuf>,
    pub bold: Option<PathBuf>,
    pub bold_italic: Option<PathBuf>
}

#[derive(Clone)]
pub struct Font{
    raw_font: Rc<RefCell<RawFont>>,
    id: u64
}

impl Font {
    pub fn new(info: &FontInfo) -> Font {
        Font{
            raw_font: Rc::new(RefCell::new(RawFont::new(info))),
            id: get_unique_id()
        }
    }

    #[doc(hidden)]
    pub fn raw_font(&self) -> &RefCell<RawFont> {
        &self.raw_font
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

thread_local!{
    static FT_LIB: Library = Library::init().unwrap();
}

struct BitmapIter<'a, I: Iterator<Item=&'a char>> {
    face: &'a Face<'static>,
    char_iter: I
}

impl<'a, I: Iterator<Item=&'a char>> BitmapIter<'a, I> {
    fn new(face: &'a Face<'static>, char_iter: I) -> BitmapIter<'a, I> {
        BitmapIter {
            face: face,
            char_iter: char_iter
        }
    }
}

impl<'a, I: Iterator<Item=&'a char>> Iterator for BitmapIter<'a, I> {
    type Item = BitmapGlyph;

    fn next(&mut self) -> Option<BitmapGlyph> {
        if let Some(c) = self.char_iter.next() {
            let c = *c;
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

pub struct RawFont {
    faces: FontFaces,
    atlas: FontAtlas,
}

impl RawFont {
    pub fn new(info: &FontInfo) -> RawFont {
        FT_LIB.with(|lib| {
            let regular = lib.new_face(&info.regular, 0).unwrap();
            let italic = info.italic.as_ref().map(|p| lib.new_face(p, 0).unwrap());
            let bold = info.bold.as_ref().map(|p| lib.new_face(p, 0).unwrap());
            let bold_italic = info.bold_italic.as_ref().map(|p| lib.new_face(p, 0).unwrap());

            RawFont {
                faces: FontFaces {                
                    regular: regular,
                    italic: italic,
                    bold: bold,
                    bold_italic: bold_italic,
                },
                atlas: FontAtlas::new()
            }
        })
    }

    /// Returns (iterator, whether or not the atlas image has been recalculated)
    pub fn word_iter<'a, 's>(&'a mut self, string: &'s str, font_size: u32, dpi: u32) -> (WordIter<'a, 's>, bool) {
        // Whether or not the font atlas image has been recalculated, and as such whether or not it should
        // be re-uploaded to the GPU.
        let mut new_atlas_image = false;

        if self.atlas.dpi != dpi {
            self.atlas.dpi = dpi;
            new_atlas_image = true;
        }
        if let Err(i) = self.atlas.font_sizes.binary_search(&font_size) {
            self.atlas.font_sizes.insert(i, font_size);
            new_atlas_image = true;
        }

        if new_atlas_image {
            self.atlas.recalculate_image(&self.faces);
        }

        (WordIter {
            font: self,
            chars: string.chars(),
            offset: Point::new(0.0, 0.0),
            has_kerning: self.faces.regular.has_kerning()
        }, new_atlas_image)
    }

    pub fn atlas_image(&self) -> ImageRef {
        ImageRef {
            pixels: &self.atlas.pixels,
            width: self.atlas.width,
            height: self.atlas.height
        }
    }

    pub fn height(&self, font_size: u32, dpi: u32) -> u16 {
        self.faces.regular.set_char_size(0, font_size as isize * 64, dpi, dpi).unwrap();
        let pixel_size = font_size * dpi / 72;
        pixel_size as u16 * self.faces.regular.height() as u16 / self.faces.regular.em_size() as u16
    }
}

pub struct ImageRef<'a> {
    pub pixels: &'a [u8],
    pub width: u32,
    pub height: u32
}

const ASCII_CHAR_ARRAY: &'static [char] = 
    &[' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
      '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
      '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 
      'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_', 
      '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 
      'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~'];

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
enum StyledChar {
    Regular(char),
    Italic(char),
    Bold(char),
    BoldItalic(char)
}

#[derive(Debug)]
struct AtlasCharInfo {
    topleft_offset: Point,
    image_rect: ImageRect,
    /// The size of the glyph, in pixels
    size: Vector2<f32>
}

pub struct FontAtlas {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    charmap: HashMap<StyledChar, AtlasCharInfo>,
    font_sizes: Vec<u32>,
    dpi: u32
}

impl FontAtlas {
    fn new() -> FontAtlas {
        FontAtlas {
            pixels: Vec::with_capacity(512 * 512),
            width: 0,
            height: 0,
            charmap: HashMap::new(),
            font_sizes: Vec::with_capacity(4),
            dpi: 0
        }
    }

    fn recalculate_image(&mut self, faces: &FontFaces) {
        use self::StyledChar::*;

        let width: u32 = 512;
        let mut height: u32 = 512;

        let mut bitmaps: Vec<(StyledChar, BitmapGlyph)> = Vec::with_capacity(128);

        self.charmap.clear();

        // Load the glyph bitmaps into the bitmap vector
        for size in self.font_sizes.iter().map(|s| *s) {
            faces.regular.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();

            bitmaps.extend(ASCII_CHAR_ARRAY.iter().map(|c| Regular(*c)).zip(BitmapIter::new(&faces.regular, ASCII_CHAR_ARRAY.iter())));

            if let Some(ref italic) = faces.italic {
                italic.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();
                bitmaps.extend(ASCII_CHAR_ARRAY.iter().map(|c| Italic(*c)).zip(BitmapIter::new(italic, ASCII_CHAR_ARRAY.iter())));
            }
            if let Some(ref bold) = faces.bold {
                bold.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();
                bitmaps.extend(ASCII_CHAR_ARRAY.iter().map(|c| Bold(*c)).zip(BitmapIter::new(bold, ASCII_CHAR_ARRAY.iter())));
            }
            if let Some(ref bold_italic) = faces.bold_italic {
                bold_italic.set_char_size(0, size as isize * 64, self.dpi, self.dpi).unwrap();
                bitmaps.extend(ASCII_CHAR_ARRAY.iter().map(|c| BoldItalic(*c)).zip(BitmapIter::new(bold_italic, ASCII_CHAR_ARRAY.iter())));
            }
        }

        // Sort the bitmap vector by descending height
        bitmaps.sort_by_key(|b| -(b.1.bitmap().rows() as isize));

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

        for &(c, b) in &bitmaps {
            let b_left = b.left();
            let b_top = b.top();
            let b = b.bitmap();

            if cursor + b.width() as u32 > width {
                cursor = 0;
            }

            let cursorus = cursor as usize;

            if 0 < b.width() {
                let max_height_in_range = *heights[cursorus..cursorus + b.width() as usize].iter().max().unwrap();

                for y in (0..b.rows()).map(|y| y as u32) {
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

                // Initially, the rect has pixels, not normalized texture coordinates. This is because we don't
                // know if the image's current resolution will actually be its final resolution. We convert these
                // into normalized texture coordinates after every glyph has been loaded into the image.
                let pixel_rect = ImageRect {
                    upleft: Point::new(cursor as f32, max_height_in_range as f32),
                    lowright: Point::new(cursor as f32 + b.width() as f32, max_height_in_range as f32 + b.rows() as f32)
                };

                let char_info = AtlasCharInfo {
                    topleft_offset: 
                        Point::new(
                            b_left as f32,
                            b_top as f32
                        ),
                    image_rect: pixel_rect,
                    size: Vector2::new(b.width() as f32, b.rows() as f32)
                };

                self.charmap.insert(c, char_info);

                let heights_slice = if cursor + b.width() as u32 > width {
                    &mut heights[cursorus..]
                } else {
                    &mut heights[cursorus..cursorus + b.width() as usize]
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

        // Because the final dimensions of the image are now known, we can safely convert the glyph rects from
        // pixel coordinates to normalized texture coordinates.
        for (_, aci) in self.charmap.iter_mut() {
            aci.image_rect.upleft.x /= width as f32;
            aci.image_rect.upleft.y /= height as f32;
            aci.image_rect.lowright.x /= width as f32;
            aci.image_rect.lowright.y /= height as f32;
        }

        self.width = width;
        self.height = height;
    }
}

pub struct WordIter<'a, 's> {
    font: &'a RawFont,
    chars: Chars<'s>,
    offset: Point,
    has_kerning: bool
}

impl<'a, 's> Iterator for WordIter<'a, 's> {
    type Item = Word<'a, 's>;

    fn next(&mut self) -> Option<Word<'a, 's>> {
        let word_offset = self.offset;
        let mut word_str = self.chars.as_str();
        let mut word_len_bytes = 0;
        let word_len_px: f32;

        {
            let mut reached_whitespace = false;
            let mut iter = CharVertIter {
                font: self.font,
                chars: self.chars.clone()
                    .take_while(|c| { // Take characters until we reach the end of the word, including the trailing whitespace
                        reached_whitespace |= c.is_whitespace();
                        !reached_whitespace ^ c.is_whitespace()
                    })
                    .map(|c| {word_len_bytes += c.len_utf8(); c}),
                last_char: '\0',
                offset: Point::new(0.0, 0.0),
                has_kerning: self.has_kerning
            };

            // Consume the character iterator without destroying it, so that we can get the physical length of
            // the word.
            for _ in &mut iter {}
            self.offset = self.offset + iter.offset;
            word_len_px = iter.offset.x / 64.0;
        }

        self.chars = word_str[word_len_bytes..].chars();
        word_str = &word_str[..word_len_bytes];

        if 0 != word_str.len() {
            Some(Word {
                font: self.font,
                word: word_str,
                word_len_px: word_len_px,
                has_kerning: self.has_kerning,
                offset: word_offset
            })
        } else {
            None
        }
    }
}

pub struct Word<'a, 's> {
    font: &'a RawFont,
    word: &'s str,
    word_len_px: f32,
    has_kerning: bool,
    offset: Point
}

impl<'a, 's> Word<'a, 's> {
    pub fn char_vert_iter(&self) -> CharVertIter<'a, Chars<'s>> {
        CharVertIter {
            font: self.font,
            chars: self.word.chars(),
            last_char: '\0',
            offset: Point::new(0.0, 0.0),
            has_kerning: self.has_kerning
        }
    }

    pub fn word(&self) -> &'s str {
        self.word
    }

    pub fn word_len_px(&self) -> f32 {
        self.word_len_px
    }

    pub fn offset(&self) -> Point {
        // Because the offset is stored as 64ths of a pixel, we must convert it into pixels
        self.offset / 64.0
    }
}


pub struct CharVertIter<'a, C> 
        where C: Iterator<Item = char> {
    font: &'a RawFont,
    chars: C,
    last_char: char,
    /// The offset from the first character in the string, stored as 1/64ths of a pixel. This value is
    /// converted to points upon return from the `next` function down below.
    offset: Point,
    has_kerning: bool
}

impl<'a, C> Iterator for CharVertIter<'a, C> 
        where C: Iterator<Item = char> {
    type Item = CharVert;

    fn next(&mut self) -> Option<CharVert> {
        if let Some(c) = self.chars.next() {
            let last_char_index = self.font.faces.regular.get_char_index(self.last_char as usize);
            let char_index = self.font.faces.regular.get_char_index(c as usize);
            self.font.faces.regular.load_glyph(char_index, LoadFlag::empty()).expect("Could not load glyph");

            if self.has_kerning {
                let kerning = self.font.faces.regular
                    .get_kerning(last_char_index, char_index, KerningMode::KerningDefault)
                    .expect("Failed to get font kerning");

                self.offset.x += kerning.x as f32;
                self.offset.y += kerning.y as f32;
            }

            let advance = self.font.faces.regular.glyph().advance();

            // Some characters don't have a glyph associated with them (e.g. spaces), so, if we can't find
            // a glyph, just skip it's retrieval and go the next glyph!
            if let Some(atlas_char_info) = self.font.atlas.charmap.get(&StyledChar::Regular(c)) {
                let vert = Some(CharVert{
                    image_rect: atlas_char_info.image_rect,
                    offset: self.offset / 64.0 + atlas_char_info.topleft_offset,
                    size: atlas_char_info.size
                });

                self.offset.x += advance.x as f32;
                self.offset.y += advance.y as f32;

                vert
            } else {
                self.offset.x += advance.x as f32;
                self.offset.y += advance.y as f32;

                self.next()
            }
        } else {
            None
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CharVert {
    pub image_rect: ImageRect,
    /// The number of pixels on the xy plane offset from the first character in the string
    pub offset: Point,
    /// The size, in pixels, of this individual character
    pub size: Vector2<f32>
}

#[derive(Debug, Clone, Copy)]
pub struct ImageRect {
    pub upleft: Point,
    pub lowright: Point
}

#[cfg(test)]
mod tests {
    // I don't know why I love calling this the "superstar pattern", but I do. Maybe because it's just an awesome name.
    use super::*;

    #[test]
    fn word_iter() {
        let font = Font::new(&FontInfo {
            regular: "./tests/DejaVuSans.ttf".into(),
            italic: None,
            bold: None,
            bold_italic: None
        });

        let mut font = font.raw_font().borrow_mut();
        let mut iter = font.word_iter("This is    a l❤vely test.\t", 12, 72).0.map(|w| w.word);
        assert_eq!(Some("This "), iter.next());
        assert_eq!(Some("is    "), iter.next());
        assert_eq!(Some("a "), iter.next());
        assert_eq!(Some("l❤vely "), iter.next());
        assert_eq!(Some("test.\t"), iter.next());
        assert_eq!(None, iter.next());
    }
}
