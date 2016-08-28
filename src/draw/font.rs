use freetype::{Library, RenderMode, Face, BitmapGlyph};
use freetype::face::LoadFlag;

use std::path::PathBuf;
use std::collections::HashSet;

use std::iter::FromIterator;

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

            Some(glyph.to_bitmap(RenderMode::Normal, None).unwrap())
        } else {
            None
        }
    }
}

pub struct Font {
    regular_face: Face<'static>,
    italic_face: Option<Face<'static>>,
    bold_face: Option<Face<'static>>,
    bold_italic_face: Option<Face<'static>>,
}

pub struct FontAtlas {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    charset: HashSet<char>
}

impl FontAtlas {
    pub fn new(faces: &FontInfo) -> FontAtlas {
        use std::char;

        let width: u32 = 512;
        let mut height: u32 = 512;

        let mut bitmaps: Vec<BitmapGlyph> = Vec::with_capacity(128);
        let chars = HashSet::from_iter((32..127).into_iter().map(|d| char::from_u32(d).unwrap()));

        // Load the bitmaps from each of the fonts into the bitmaps vec
        let regular_face = FT_LIB.with(|lib| lib.new_face(&faces.regular, 0).unwrap());
        regular_face.set_char_size(0, 16*64, 72, 72).unwrap();
        bitmaps.extend(BitmapIter::new(&regular_face, chars.iter().map(|c| *c)));

        bitmaps.sort_by_key(|b| -(b.bitmap().rows() as isize));

        let mut pixels: Vec<u8> = vec![0; width as usize * height as usize];
        let mut heights: Vec<u32> = vec![0; width as usize];

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

                    if pixels_index + b.width() as usize > pixels.len() {
                        pixels.reserve((height * width) as usize);
                        height *= 2;
                        unsafe{ pixels.set_len((height * width) as usize) };
                    }

                    pixels[pixels_index..pixels_index + b.width() as usize]
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
        pixels.truncate((height * width) as usize);

        FontAtlas {
            pixels: pixels,
            width: width,
            height: height,
            charset: chars
        }
    }
}
