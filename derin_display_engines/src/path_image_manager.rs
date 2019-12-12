use std::{
    collections::hash_map::HashMap,
    path::{Path, PathBuf},
    io::{self, Read},
    fs,
    ffi::OsStr,
};
use crate::{
    HasLifetimeIterator,
    rect_layout::{
        ImageManager, ImageLayout,
        theme::{Color, ImageId}
    },
    gullery_display_engine::ImageRasterizer,
};
use cgmath_geometry::{D2, rect::DimsBox};

// All `PathBuf`s in this struct are canonicalized.
pub struct PathImageManager {
    /// Root path. All images must be contained within this folder.
    root_path: PathBuf,
    images: HashMap<ImageId, (ImageLayout, Vec<Color>)>,
    paths: HashMap<PathBuf, ImageId>,
}

impl PathImageManager {
    pub fn new(root_path: PathBuf) -> Result<PathImageManager, io::Error> {
        Ok(PathImageManager {
            root_path: root_path.canonicalize()?,
            images: HashMap::new(),
            paths: HashMap::new(),
        })
    }
}

impl ImageManager for PathImageManager {
    type ImagePath = Path;
    type ResolveImageError = io::Error;
    fn resolve_image(&mut self, image_path: &Path) -> Result<ImageId, io::Error> {
        let image_path = image_path.canonicalize()?;
        if !image_path.starts_with(&self.root_path) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("image path {:?} is not subpath of {:?}", image_path, &self.root_path)));
        }
        if image_path.extension() != Some(OsStr::new("png")) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("image path {:?} is not a .png file", image_path)));
        }
        let image = fs::File::open(&image_path)?;
        let layout_path = {
            let mut p = image_path.clone();
            p.set_extension("png.layout.toml");
            p
        };
        let layout = fs::File::open(&layout_path);

        let pixel_buffer: Vec<Color> = {
            let mut image_decoder = png::Decoder::new(image);
            image_decoder.set_transformations(
                png::Transformations::STRIP_16 | png::Transformations::EXPAND
            );

            let (image_info, mut reader) = image_decoder.read_info()?;
            assert!(png::ColorType::RGB == image_info.color_type || png::ColorType::RGBA == image_info.color_type);
            assert_eq!(png::BitDepth::Eight, image_info.bit_depth);
            let mut image_buffer = vec![0; image_info.buffer_size()];
            reader.next_frame(&mut image_buffer)?;
            match image_info.color_type {
                png::ColorType::RGBA => Color::from_raw_slice(&image_buffer).to_vec(),
                png::ColorType::RGB => image_buffer.chunks(3).map(|s| Color::new(s[0], s[1], s[2], 255)).collect(),
                _ => unreachable!()
            }
        };
        let image_layout: ImageLayout = layout
            .and_then(|mut layout| {
                let mut buffer = String::new();
                layout.read_to_string(&mut buffer)?;
                toml::from_str(&buffer).map_err(|e| e.into())
            })
            .unwrap_or_default();
        let image_id = ImageId::new();
        self.images.insert(image_id, (image_layout, pixel_buffer));
        self.paths.insert(image_path, image_id);
        Ok(image_id)
    }
    fn image_layout(&mut self, image_id: ImageId) -> Option<ImageLayout> {
        self.images.get(&image_id).map(|(layout, _)| *layout)
    }
}
impl ImageRasterizer for PathImageManager {
    fn rasterize(&mut self, image: ImageId) -> Option<(DimsBox<D2, u16>, ImageRasterizerIter<'_>)> {
        self.images
            .get(&image)
            .map(|(image_layout, pixel_buffer)| (
                DimsBox::new2(
                    image_layout.dims.width as _,
                    image_layout.dims.height as _
                ),
                pixel_buffer.iter().cloned()
            ))
    }
}

impl<'a> HasLifetimeIterator<'a, Color> for PathImageManager {
    type Iter = ImageRasterizerIter<'a>;
}

pub type ImageRasterizerIter<'a> = impl 'a + Iterator<Item=Color>;
