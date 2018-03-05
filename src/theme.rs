use png;
use gullery::colors::Rgba;
use gullery::glsl::Nu8;

use cgmath::Point2;
use cgmath_geometry::DimsBox;
use dct::layout::{Align, Align2, Margins};

use std::io;
use std::rc::Rc;
use std::path::Path;
use std::collections::HashMap;

use core::render::Theme as CoreTheme;
pub use dct::cursor::CursorIcon;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Image {
    pub pixels: Vec<Rgba<Nu8>>,
    pub dims: DimsBox<Point2<u32>>,
    pub rescale: RescaleRules
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RescaleRules {
    Stretch,
    /// Similar to stretch, but begins sampling image half a pixel away from the border. Can
    /// eliminate border artifacts in some scenarios.
    StretchOnPixelCenter,
    Slice(Margins<u16>)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineWrap {
    /// Disallow all line breaks, including explicit ones (such as from `'\n'`).
    None,
    /// Allow line breaks at break points, as defined by [UAX #14](https://unicode.org/reports/tr14/).
    Normal
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeText {
    pub face: ThemeFace,
    pub color: Rgba<Nu8>,
    pub highlight_bg_color: Rgba<Nu8>,
    pub highlight_text_color: Rgba<Nu8>,
    pub face_size: u32,
    pub tab_size: u32,
    pub justify: Align2,
    pub margins: Margins<u16>,
    pub line_wrap: LineWrap
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeWidget {
    pub text: Option<ThemeText>,
    pub icon: Option<Rc<Image>>,
}

/// Reference-counted face handle. This is cheap to clone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeFace {
    font_path: Rc<Path>,
    face_index: i32
}

pub struct Theme {
    map: HashMap<String, ThemeWidget>
}


impl ThemeFace {
    #[inline]
    pub fn new<P: AsRef<Path>>(path: P, face_index: i32) -> Result<ThemeFace, io::Error> {
        Ok(ThemeFace {
            font_path: path.as_ref().canonicalize()?.into(),
            face_index
        })
    }

    #[inline]
    pub fn font_path(&self) -> &Path {
        &self.font_path
    }

    #[inline]
    pub fn face_index(&self) -> i32 {
        self.face_index
    }
}

impl Theme {
    pub fn empty() -> Theme {
        Theme {
            map: HashMap::new()
        }
    }

    pub fn insert_widget(&mut self, key: String, theme: ThemeWidget) -> Option<ThemeWidget> {
        self.map.insert(key, theme)
    }
}

impl CoreTheme for Theme {
    type Key = str;
    type ThemeValue = ThemeWidget;

    fn widget_theme(&self, path: &str) -> ThemeWidget {
        self.map.get(path).cloned().unwrap_or(
            ThemeWidget {
                text: None,
                icon: None
            }
        )
    }
}

impl Default for Theme {
    fn default() -> Theme {
        let mut theme = Theme::empty();

        macro_rules! upload_image {
            ($name:expr, $path:expr, $dims:expr, $border:expr) => {{
                let image_png = png::Decoder::new(::std::io::Cursor::new(&include_bytes!($path)[..]));
                let (info, mut reader) = image_png.read_info().unwrap();
                // Allocate the output buffer.
                let mut image = vec![0; info.buffer_size()];
                // Read the next frame. Currently this function should only called once.
                // The default options
                reader.next_frame(&mut image).unwrap();
                theme.insert_widget(
                    $name.to_string(),
                    ThemeWidget {
                        text: Some(ThemeText {
                            // TODO: DON'T LOAD FROM SRC
                            face: ThemeFace::new("./src/default_theme_resources/DejaVuSans.ttf", 0).unwrap(),
                            color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                            highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                            highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                            face_size: 16 * 64,
                            tab_size: 8,
                            justify: Align2::new(Align::Start, Align::Start),
                            margins: Margins::new($border, $border, $border, $border),
                            line_wrap: LineWrap::None
                        }),
                        icon: Some(Rc::new(Image {
                            pixels: unsafe {
                                Vec::from_raw_parts(
                                    image.as_mut_ptr() as *mut _,
                                    image.len() / 4,
                                    image.capacity() / 4
                                )
                            },
                            dims: DimsBox::new2($dims, $dims),
                            rescale: RescaleRules::Slice(Margins::new($border, $border, $border, $border))
                        }))
                    }
                );

                ::std::mem::forget(image);
            }}
        }

        upload_image!("Group", "./default_theme_resources/group.png", 3, 1);
        upload_image!("Button::Normal", "./default_theme_resources/button.normal.png", 32, 4);
        upload_image!("Button::Hover", "./default_theme_resources/button.hover.png", 32, 4);
        upload_image!("Button::Clicked", "./default_theme_resources/button.clicked.png", 32, 4);
        upload_image!("EditBox", "./default_theme_resources/editbox.png", 8, 3);
        theme.insert_widget(
            "Label".to_string(),
            ThemeWidget {
                text: Some(ThemeText {
                    face: ThemeFace::new("./src/default_theme_resources/DejaVuSans.ttf", 0).unwrap(),
                    color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                    highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                    highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                    face_size: 16 * 64,
                    tab_size: 8,
                    justify: Align2::new(Align::Center, Align::Start),
                    margins: Margins::default(),
                    line_wrap: LineWrap::None
                }),
                icon: None
            }
        );

        theme
    }
}

impl Image {
    pub fn min_size(&self) -> DimsBox<Point2<i32>> {
        match self.rescale {
            RescaleRules::StretchOnPixelCenter |
            RescaleRules::Stretch => DimsBox::new2(0, 0),
            RescaleRules::Slice(margins) => DimsBox::new2(margins.width() as i32, margins.height() as i32)
        }
    }
}
