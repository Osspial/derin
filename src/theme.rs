use gl_raii::colors::Rgba;
use gl_raii::glsl::Nu8;

use cgmath_geometry::DimsRect;
use dct::hints::{Align2, Margins};

use std::io;
use std::rc::Rc;
use std::path::Path;
use std::collections::HashMap;

use core::render::Theme as CoreTheme;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Image {
    pub pixels: Vec<Rgba<Nu8>>,
    pub dims: DimsRect<u32>,
    pub rescale: RescaleRules
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RescaleRules {
    Stretch,
    Slice(Margins<u16>)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeText {
    pub face: ThemeFace,
    pub color: Rgba<Nu8>,
    pub face_size: u32,
    pub tab_size: u32,
    pub justify: Align2
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemeNode {
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
    map: HashMap<String, ThemeNode>
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
    pub fn new() -> Theme {
        Theme {
            map: HashMap::new()
        }
    }

    pub fn insert_node(&mut self, key: String, theme: ThemeNode) -> Option<ThemeNode> {
        self.map.insert(key, theme)
    }
}

impl CoreTheme for Theme {
    type Key = str;
    type ThemeValue = ThemeNode;

    fn node_theme(&self, path: &str) -> ThemeNode {
        self.map.get(path).cloned().unwrap_or(
            ThemeNode {
                text: None,
                icon: None
            }
        )
    }
}
