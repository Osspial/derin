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

//! Types used to specify how widgets should be drawn.

use png;
use gullery::colors::Rgba;
use gullery::glsl::Nu8;

use cgmath::Point2;
use cgmath_geometry::DimsBox;
use derin_common_types::layout::{Align, Align2, Margins, SizeBounds};

use std::io;
use std::rc::Rc;
use std::path::Path;
use std::collections::HashMap;
use std::hash::{Hash, Hasher, BuildHasher};
use std::collections::hash_map::RandomState;


use core::render::Theme as CoreTheme;
pub use derin_common_types::cursor::CursorIcon;

pub mod color {
    pub use gullery::colors::Rgba;
    pub use gullery::glsl::Nu8;
}

/// An RGBA representation of an image.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Image {
    pub pixels: Vec<Rgba<Nu8>>,
    pub dims: DimsBox<Point2<u32>>,
    pub rescale: RescaleRules,
    pub size_bounds: SizeBounds
}

/// The algorithm used to rescale an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RescaleRules {
    /// Rescale the image by uniformily stretching it out, from its edges.
    Stretch,
    /// Similar to stretch, but begins sampling image half a pixel away from the border. Can
    /// eliminate border artifacts in some scenarios.
    StretchOnPixelCenter,
    /// Perform nine-slicing on the provided image, stretching out the center of the image while
    /// keeping the borders of the image a constant size.
    Slice(Margins<u16>),
    Align(Align2)
}

/// The algorithm used to determine where line breaks occur in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineWrap {
    /// Disallow all line breaks, including explicit ones (such as from `'\n'`).
    None,
    /// Allow line breaks at break points, as defined by [UAX #14](https://unicode.org/reports/tr14/).
    Normal
}

/// Collection of information used to determine how to render text in a widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeText {
    /// A handle to the font face used to draw the text.
    pub face: ThemeFace,
    /// The color to draw text.
    pub color: Rgba<Nu8>,
    /// The color of the highlight when highlighting text.
    pub highlight_bg_color: Rgba<Nu8>,
    /// The color of highlighted text.
    pub highlight_text_color: Rgba<Nu8>,
    /// The size of the text being drawn, in 64ths of a [point].
    ///
    /// [point]: https://en.wikipedia.org/wiki/Point_(typography)
    pub face_size: u32,
    /// The number of spaces contained within a tab stop.
    pub tab_size: u32,
    /// The horizontal and vertical justification of the text.
    pub justify: Align2,
    /// The number of pixels on the sides of a draw box in which text shouldn't be drawn.
    pub margins: Margins<u16>,
    /// The line wrapping algorithm.
    pub line_wrap: LineWrap
}

/// The text style and image used to draw a widget with a given style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeWidget {
    pub text: Option<ThemeText>,
    pub image: Option<Rc<Image>>,
}

/// Reference-counted face handle. This is cheap to clone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeFace {
    Path(ThemeFacePath),
    Buffer(ThemeFaceBuffer)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeFacePath {
    font_path: Rc<Path>,
    face_index: i32,
    fingerprint: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeFaceBuffer {
    font_buffer: Rc<[u8]>,
    face_index: i32,
    fingerprint: u64,
}

pub struct Theme {
    map: HashMap<String, ThemeWidget>
}


impl ThemeFace {
    #[inline]
    pub fn face_index(&self) -> i32 {
        match *self {
            ThemeFace::Path(ThemeFacePath{face_index, ..}) |
            ThemeFace::Buffer(ThemeFaceBuffer{face_index, ..}) => face_index
        }
    }

    #[inline]
    pub fn fingerprint(&self) -> u64 {
        match *self {
            ThemeFace::Path(ThemeFacePath{fingerprint, ..}) |
            ThemeFace::Buffer(ThemeFaceBuffer{fingerprint, ..}) => fingerprint
        }
    }
}

lazy_static!{
    static ref FINGERPRINT_STATE: RandomState = RandomState::new();
}

impl ThemeFacePath {
    /// Create a new face, referencing the font file at the provided path.
    #[inline]
    pub fn new<P: AsRef<Path>>(path: P, face_index: i32) -> Result<ThemeFacePath, io::Error> {
        let font_path: Rc<Path> = path.as_ref().canonicalize()?.into();
        let mut hasher = FINGERPRINT_STATE.build_hasher();
        font_path.hash(&mut hasher);
        face_index.hash(&mut hasher);
        let fingerprint = hasher.finish();

        Ok(ThemeFacePath {
            font_path,
            face_index,
            fingerprint
        })
    }

    /// Retrieve the path of the font file.
    #[inline]
    pub fn font_path(&self) -> &Path {
        &self.font_path
    }

    /// Gets the index of the face within the font file.
    #[inline]
    pub fn face_index(&self) -> i32 {
        self.face_index
    }

    #[inline]
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }
}

impl ThemeFaceBuffer {
    /// Create a new face from the given buffer.
    #[inline]
    pub fn new(font_buffer: Rc<[u8]>, face_index: i32) -> ThemeFaceBuffer {
        let mut hasher = FINGERPRINT_STATE.build_hasher();
        font_buffer.hash(&mut hasher);
        face_index.hash(&mut hasher);
        let fingerprint = hasher.finish();

        ThemeFaceBuffer {
            font_buffer,
            face_index,
            fingerprint
        }
    }

    /// Retrieve the path of the font file.
    #[inline]
    pub fn font_buffer(&self) -> &Rc<[u8]> {
        &self.font_buffer
    }

    /// Gets the index of the face within the font file.
    #[inline]
    pub fn face_index(&self) -> i32 {
        self.face_index
    }

    #[inline]
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
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
                image: None
            }
        )
    }
}

impl Default for Theme {
    fn default() -> Theme {
        let mut theme = Theme::empty();

        let image_buf = |png_buf| {
            let image_png = png::Decoder::new(::std::io::Cursor::new(png_buf));
            let (info, mut reader) = image_png.read_info().unwrap();
            // Allocate the output buffer.
            let mut image = vec![0; info.buffer_size()];
            // Read the next frame. Currently this function should only called once.
            // The default options
            reader.next_frame(&mut image).unwrap();
            let image_resized = unsafe {
                Vec::from_raw_parts(
                    image.as_mut_ptr() as *mut _,
                    image.len() / 4,
                    image.capacity() / 4
                )
            };
            ::std::mem::forget(image);
            image_resized
        };
        thread_local!{
            static DEJA_VU_SANS: Rc<[u8]> = Rc::from(&include_bytes!("./default_theme_resources/DejaVuSans.ttf")[..]);
        }
        let font = ThemeFace::Buffer(ThemeFaceBuffer::new(DEJA_VU_SANS.with(|b| b.clone()), 0));

        macro_rules! image_buf {
            ($path:expr) => {{image_buf(&include_bytes!($path)[..])}}
        }
        macro_rules! upload_image {
            ($name:expr, $path:expr, $dims:expr, $border:expr, $text_align:expr) => {{
                theme.insert_widget(
                    $name.to_string(),
                    ThemeWidget {
                        text: Some(ThemeText {
                            face: font.clone(),
                            color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                            highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                            highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                            face_size: 16 * 64,
                            tab_size: 8,
                            justify: $text_align,
                            margins: Margins::new($border, $border, $border, $border),
                            line_wrap: LineWrap::None
                        }),
                        image: Some(Rc::new(Image {
                            pixels: image_buf!($path),
                            dims: DimsBox::new2($dims.0, $dims.1),
                            rescale: RescaleRules::Slice(Margins::new($border, $border, $border, $border)),
                            size_bounds: SizeBounds {
                                min: DimsBox::new2($border * 2, $border * 2),
                                ..SizeBounds::default()
                            }
                        }))
                    }
                );
            }}
        }

        upload_image!("Group", "./default_theme_resources/group.png", (3, 3), 1, Align2::new(Align::Start, Align::Start));
        upload_image!("Button::Normal", "./default_theme_resources/button/base.png", (16, 16), 4, Align2::new(Align::Center, Align::Center));
        upload_image!("Button::Hover", "./default_theme_resources/button/hover.png", (16, 16), 4, Align2::new(Align::Center, Align::Center));
        upload_image!("Button::Pressed", "./default_theme_resources/button/pressed.png", (16, 16), 4, Align2::new(Align::Center, Align::Center));
        upload_image!("ScrollBar", "./default_theme_resources/scroll_bar.png", (3, 3), 1, Align2::new(Align::Center, Align::Center));
        upload_image!("ScrollBackground", "./default_theme_resources/scroll_bg.png", (3, 3), 1, Align2::new(Align::Center, Align::Center));
        theme.insert_widget(
            "Slider::Bar".to_string(),
            ThemeWidget {
                text: None,
                image: Some(Rc::new(Image {
                    pixels: image_buf!("./default_theme_resources/slider_bar.png"),
                    dims: DimsBox::new2(32, 8),
                    rescale: RescaleRules::Slice(Margins::new(4, 4, 4, 4)),
                    size_bounds: SizeBounds {
                        min: DimsBox::new2(32, 8),
                        max: DimsBox::new2(i32::max_value(), 8)
                    }
                }))
            }
        );
        theme.insert_widget(
            "Slider::Head".to_string(),
            ThemeWidget {
                text: None,
                image: Some(Rc::new(Image {
                    pixels: image_buf!("./default_theme_resources/slider_head.png"),
                    dims: DimsBox::new2(8, 16),
                    rescale: RescaleRules::Align(Align2::new(Align::Center, Align::Center)),
                    size_bounds: SizeBounds {
                        min: DimsBox::new2(8, 16),
                        ..SizeBounds::default()
                    }
                }))
            }
        );
        theme.insert_widget(
            "Label".to_string(),
            ThemeWidget {
                text: Some(ThemeText {
                    face: font.clone(),
                    color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                    highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                    highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                    face_size: 16 * 64,
                    tab_size: 8,
                    justify: Align2::new(Align::Center, Align::Start),
                    margins: Margins::default(),
                    line_wrap: LineWrap::Normal
                }),
                image: None
            }
        );
        theme.insert_widget(
            "CheckBox".to_string(),
            ThemeWidget {
                text: Some(ThemeText {
                    face: font.clone(),
                    color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                    highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                    highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                    face_size: 16 * 64,
                    tab_size: 8,
                    justify: Align2::new(Align::Start, Align::Center),
                    margins: Margins::new(18, 0, 0, 0),
                    line_wrap: LineWrap::None
                }),
                image: None
            }
        );
        macro_rules! checkbox {
            ($name:expr, $path:expr) => {
                theme.insert_widget(
                    concat!("CheckBox::", $name).to_string(),
                    ThemeWidget {
                        text: None,
                        image: Some(Rc::new(Image {
                            pixels: image_buf!($path),
                            dims: DimsBox::new2(16, 16),
                            rescale: RescaleRules::Align(Align2::new(Align::Start, Align::Center)),
                            size_bounds: SizeBounds {
                                min: DimsBox::new2(16, 16),
                                ..SizeBounds::default()
                            }
                        }))
                    }
                );
            }
        }
        checkbox!("Empty", "./default_theme_resources/checkbox/empty.png");
        checkbox!("Empty::Hover", "./default_theme_resources/checkbox/empty.hover.png");
        checkbox!("Empty::Pressed", "./default_theme_resources/checkbox/empty.pressed.png");
        checkbox!("Checked", "./default_theme_resources/checkbox/checked.png");
        checkbox!("Checked::Hover", "./default_theme_resources/checkbox/checked.hover.png");
        checkbox!("Checked::Pressed", "./default_theme_resources/checkbox/checked.pressed.png");
        theme.insert_widget(
            "RadioButton".to_string(),
            ThemeWidget {
                text: Some(ThemeText {
                    face: font.clone(),
                    color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                    highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                    highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                    face_size: 16 * 64,
                    tab_size: 8,
                    justify: Align2::new(Align::Start, Align::Center),
                    margins: Margins::new(18, 0, 0, 0),
                    line_wrap: LineWrap::None
                }),
                image: None
            }
        );
        macro_rules! radiobutton {
            ($name:expr, $path:expr) => {
                theme.insert_widget(
                    concat!("RadioButton::", $name).to_string(),
                    ThemeWidget {
                        text: None,
                        image: Some(Rc::new(Image {
                            pixels: image_buf!($path),
                            dims: DimsBox::new2(16, 16),
                            rescale: RescaleRules::Align(Align2::new(Align::Start, Align::Center)),
                            size_bounds: SizeBounds {
                                min: DimsBox::new2(16, 16),
                                ..SizeBounds::default()
                            }
                        }))
                    }
                );
            }
        }
        radiobutton!("Empty", "./default_theme_resources/radiobutton/empty.png");
        radiobutton!("Empty::Hover", "./default_theme_resources/radiobutton/empty.hover.png");
        radiobutton!("Empty::Pressed", "./default_theme_resources/radiobutton/empty.pressed.png");
        radiobutton!("Selected", "./default_theme_resources/radiobutton/selected.png");
        radiobutton!("Selected::Hover", "./default_theme_resources/radiobutton/selected.hover.png");
        radiobutton!("Selected::Pressed", "./default_theme_resources/radiobutton/selected.pressed.png");

        macro_rules! progress_bar {
            ($name:expr, $path:expr) => {
                theme.insert_widget(
                    concat!("ProgressBar::", $name).to_string(),
                    ThemeWidget {
                        text: None,
                        image: Some(Rc::new(Image {
                            pixels: image_buf!($path),
                            dims: DimsBox::new2(16, 16),
                            rescale: RescaleRules::Slice(Margins::new(3, 2, 3, 2)),
                            size_bounds: SizeBounds {
                                min: DimsBox::new2(6, 4),
                                ..SizeBounds::default()
                            }
                        }))
                    }
                );
            }
        }
        progress_bar!("Background", "./default_theme_resources/progressbar.bg.png");
        progress_bar!("Fill", "./default_theme_resources/progressbar.fill.png");

        macro_rules! tab {
            ($name:expr, $path:expr) => {
                theme.insert_widget(
                    concat!("Tab::", $name).to_string(),
                    ThemeWidget {
                        text: Some(ThemeText {
                            face: font.clone(),
                            color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                            highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                            highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                            face_size: 16 * 64,
                            tab_size: 8,
                            justify: Align2::new(Align::Center, Align::Center),
                            margins: Margins::new(4, 4, 4, 4),
                            line_wrap: LineWrap::None
                        }),
                        image: Some(Rc::new(Image {
                            pixels: image_buf!($path),
                            dims: DimsBox::new2(16, 8),
                            rescale: RescaleRules::Slice(Margins::new(4, 4, 4, 0)),
                            size_bounds: SizeBounds {
                                min: DimsBox::new2(8, 4),
                                ..SizeBounds::default()
                            }
                        }))
                    }
                );
            }
        }
        tab!("Normal", "./default_theme_resources/tab/base.png");
        tab!("Hover", "./default_theme_resources/tab/hover.png");
        tab!("Pressed", "./default_theme_resources/tab/pressed.png");
        theme.insert_widget(
            "EditBox".to_string(),
            ThemeWidget {
                text: Some(ThemeText {
                    face: font.clone(),
                    color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                    highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                    highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                    face_size: 16 * 64,
                    tab_size: 8,
                    justify: Align2::new(Align::Start, Align::Start),
                    margins: Margins::new(3, 3, 3, 3),
                    line_wrap: LineWrap::Normal
                }),
                image: Some(Rc::new(Image {
                    pixels: image_buf!("./default_theme_resources/editbox.png"),
                    dims: DimsBox::new2(8, 8),
                    rescale: RescaleRules::Slice(Margins::new(3, 3, 3, 3)),
                    size_bounds: SizeBounds {
                        min: DimsBox::new2(3 * 2, 3 * 2),
                        ..SizeBounds::default()
                    }
                }))
            }
        );
        theme.insert_widget(
            "LineBox".to_string(),
            ThemeWidget {
                text: Some(ThemeText {
                    face: font.clone(),
                    color: Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255)),
                    highlight_bg_color: Rgba::new(Nu8(0), Nu8(120), Nu8(215), Nu8(255)),
                    highlight_text_color: Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255)),
                    face_size: 16 * 64,
                    tab_size: 8,
                    justify: Align2::new(Align::Start, Align::Start),
                    margins: Margins::new(3, 3, 3, 3),
                    line_wrap: LineWrap::None
                }),
                image: Some(Rc::new(Image {
                    pixels: image_buf!("./default_theme_resources/editbox.png"),
                    dims: DimsBox::new2(8, 8),
                    rescale: RescaleRules::Slice(Margins::new(3, 3, 3, 3)),
                    size_bounds: SizeBounds {
                        min: DimsBox::new2(3 * 2, 3 * 2),
                        ..SizeBounds::default()
                    }
                }))
            }
        );

        theme
    }
}

impl Image {
    pub fn min_size(&self) -> DimsBox<Point2<i32>> {
        self.size_bounds.min
        // match self.rescale {
        //     RescaleRules::Align(_) => self.dims.cast().unwrap_or(DimsBox::new2(i32::max_value(), i32::max_value())),
        //     RescaleRules::StretchOnPixelCenter |
        //     RescaleRules::Stretch => DimsBox::new2(0, 0),
        //     RescaleRules::Slice(margins) => DimsBox::new2(margins.width() as i32, margins.height() as i32),
        // }
    }
}
