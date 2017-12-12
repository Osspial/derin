use theme::ThemeFace;
use glyphydog::{Face, FTLib, Error};

use std::path::PathBuf;

struct FaceCached {
    path: PathBuf,
    face_index: i32,
    face: Face<()>
}

pub struct FontCache {
    lib: FTLib,
    faces: Vec<FaceCached>,
    max_faces: usize
}

impl FontCache {
    pub fn new() -> FontCache {
        FontCache {
            lib: FTLib::new(),
            faces: Vec::new(),
            max_faces: 16
        }
    }

    pub fn face(&mut self, theme_face: ThemeFace) -> Result<&mut Face<()>, Error> {
        let mut cached_face_index = None;

        for (i, face) in self.faces.iter().enumerate() {
            if &*face.path == theme_face.font_path() && face.face_index == theme_face.face_index() {
                cached_face_index = Some(i);
            }
        }

        match cached_face_index {
            Some(i) => {
                if i > 1 {
                    // Move the newest face to the front of the face list.
                    self.faces[..i].rotate(i - 1);
                }
                Ok(&mut self.faces[0].face)
            },
            None => {
                self.faces.insert(
                    0,
                    FaceCached {
                        path: theme_face.font_path().to_owned(),
                        face_index: theme_face.face_index(),
                        face: Face::new_path(theme_face.font_path(), theme_face.face_index(), &self.lib)?
                    }
                );
                if self.faces.len() > self.max_faces {
                    self.faces.pop();
                }
                Ok(&mut self.faces[0].face)
            }
        }
    }
}
