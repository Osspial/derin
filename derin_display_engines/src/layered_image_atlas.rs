use cgmath_geometry::{D2, rect::{DimsBox, GeoBox}};
use derin_atlas::SkylineAtlas;
use crate::{
    HasLifetimeIterator,
    gullery_display_engine::{LayeredImageAtlas, Image, ImageAtlasRect},
    rect_layout::{
        theme::{Color, ImageId},
    },
};
use std::collections::HashMap;

pub struct SkylineLayeredImageAtlas {
    layers: Vec<SkylineAtlas<Color>>,
    images: HashMap<ImageId, (DimsBox<D2, u16>, ImageCoords)>,
    updated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ImageCoords {
    SingleLayer(ImageAtlasRect),
    MultiLayer(Box<[ImageAtlasRect]>),
}

const BACKGROUND_COLOR: Color = Color::new(255, 0, 255, 255);

impl SkylineLayeredImageAtlas {
    pub fn new(layer_dims: DimsBox<D2, u32>) -> SkylineLayeredImageAtlas {
        SkylineLayeredImageAtlas {
            layers: vec![SkylineAtlas::new(BACKGROUND_COLOR, layer_dims)],
            images: HashMap::new(),
            updated: true,
        }
    }
}

impl LayeredImageAtlas for SkylineLayeredImageAtlas {
    fn add_image(&mut self, image: ImageId, dims: DimsBox<D2, u16>, pixels: impl IntoIterator<Item=Color>) {
        assert!(dims.width() <= self.layer_dims().width());
        assert!(dims.height() <= self.layer_dims().height());
        let dims_u32 = dims.cast::<u32>().unwrap();

        let image_rect = {
            let add_image_deferred = self.layers
                .iter_mut().enumerate()
                .filter_map(|(i, l)| l.add_image_deferred(dims_u32).map(|aid| (i, aid))).next();

            let atlas_rect = match add_image_deferred {
                Some((layer, add_image_deferred)) => {
                    let layer_subrect = add_image_deferred.add_image_pixels(pixels).cast::<u16>().unwrap();
                    ImageAtlasRect {
                        layer: layer as u16,
                        layer_subrect: layer_subrect.into(),
                    }
                },
                None => {
                    let mut new_layer = SkylineAtlas::new(BACKGROUND_COLOR, self.layer_dims().cast::<u32>().unwrap());
                    let layer_subrect = new_layer.add_image_pixels(dims_u32, pixels).ok().expect("unreachable").cast::<u16>().unwrap();
                    self.layers.push(new_layer);
                    ImageAtlasRect {
                        layer: self.layers.len() as u16 - 1,
                        layer_subrect: layer_subrect.into(),
                    }
                }
            };
            ImageCoords::SingleLayer(atlas_rect)
        };
        self.updated = true;
        self.images.insert(image, (dims, image_rect));
    }
    fn image_coords(&self, image: ImageId) -> Option<Image<'_>> {
        self.images.get(&image).map(|(dims, coords)| Image {
            dims: *dims,
            atlas_rects: coords.image_atlas_rect_slice(),
        })
    }
    fn layers(&self) -> AtlasLayersIter<'_> {
        self.layers.iter().map(|l| l.pixels())
    }
    fn num_layers(&self) -> u16 {
        self.layers.len() as u16
    }
    fn layer_dims(&self) -> DimsBox<D2, u16> {
        self.layers[0].dims().cast::<u16>().unwrap()
    }
    fn clean(&mut self) {
        // TODO: Remove infrequently-used images from atlas
        self.updated = false;
    }
    fn updated_since_clean(&self) -> bool {
        self.updated
    }
}

impl ImageCoords {
    fn image_atlas_rect_slice(&self) -> &[ImageAtlasRect] {
        match self {
            ImageCoords::SingleLayer(rect) => std::slice::from_ref(rect),
            ImageCoords::MultiLayer(rects) => &*rects,
        }
    }
}

pub type AtlasLayersIter<'a> = impl Iterator<Item=&'a [Color]>;

impl<'a> HasLifetimeIterator<'a, &'a [Color]> for SkylineLayeredImageAtlas {
    type Iter = AtlasLayersIter<'a>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_image() {
        // +--------------+
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // |              |
        // +--------------+
        let layer_size = DimsBox::new2(16, 16);
        // +------+
        // |      |
        // |      |
        // |      |
        // |      |
        // |      |
        // |      |
        // +------+
        let image_square = DimsBox::new2(8, 8);
        // +--------------+
        // |              |
        // |              |
        // +--------------+
        let image_rect_long = DimsBox::new2(16, 4);
        // +--+
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // |  |
        // +--+
        let image_rect_tall = DimsBox::new2(4, 16);
        let pixels = |dims: DimsBox<D2, u16>, variant: u16|
            (0..dims.dims.x)
                .flat_map(move |x| (0..dims.dims.y).map(move |y| (x, y)))
                .map(move |(x, y)| (x + variant, y + variant))
                .map(|(x, y)| Color::new(x as u8, y as u8, 0, 255));


        let image_ids = (0..255).map(|_| ImageId::new()).collect::<Vec<_>>();
        let mut atlas = SkylineLayeredImageAtlas::new(layer_size);

        let test_images = |atlas: &SkylineLayeredImageAtlas, image_properties: Vec<(ImageId, DimsBox<D2, u16>, Vec<u16>)>| {
            let images = image_properties.iter()
                .map(|(id, _, _)| atlas.image_coords(*id).unwrap())
                .collect::<Vec<_>>();
            for i in 0..images.len() {
                let image = images[i];
                assert_eq!(image_properties[i].1, image.dims);
                assert_eq!(image_properties[i].2, image.atlas_rects.iter().cloned().map(|r| r.layer).collect::<Vec<_>>());
            }
            let all_subrects = images.iter().flat_map(|i| i.atlas_rects).map(|i| i.layer_subrect).collect::<Vec<_>>();
            let rects_permuted =
                all_subrects.iter().cloned().flat_map(|r0| all_subrects.iter().cloned().map(move |r1| (r0, r1)));
            for (r0, r1) in rects_permuted {
                assert!(r0.intersect_rect(r1).is_none());
            }
        };


        atlas.add_image(image_ids[0], image_square, pixels(image_square, 0));
        atlas.add_image(image_ids[1], image_square, pixels(image_square, 1));
        atlas.add_image(image_ids[2], image_square, pixels(image_square, 2));
        atlas.add_image(image_ids[3], image_square, pixels(image_square, 3));
        test_images(&atlas, vec![
            (image_ids[0], image_square, vec![0]),
            (image_ids[1], image_square, vec![0]),
            (image_ids[2], image_square, vec![0]),
            (image_ids[3], image_square, vec![0]),
        ]);

        atlas.add_image(image_ids[4], image_rect_long, pixels(image_rect_long, 0));
        atlas.add_image(image_ids[5], image_rect_long, pixels(image_rect_long, 1));
        atlas.add_image(image_ids[6], image_rect_long, pixels(image_rect_long, 2));
        atlas.add_image(image_ids[7], image_rect_long, pixels(image_rect_long, 3));
        test_images(&atlas, vec![
            (image_ids[4], image_rect_long, vec![1]),
            (image_ids[5], image_rect_long, vec![1]),
            (image_ids[6], image_rect_long, vec![1]),
            (image_ids[7], image_rect_long, vec![1]),
        ]);

        atlas.add_image(image_ids[8], image_rect_tall, pixels(image_rect_tall, 0));
        atlas.add_image(image_ids[9], image_rect_tall, pixels(image_rect_tall, 1));
        atlas.add_image(image_ids[10], image_rect_tall, pixels(image_rect_tall, 2));
        atlas.add_image(image_ids[11], image_rect_tall, pixels(image_rect_tall, 3));
        test_images(&atlas, vec![
            (image_ids[8], image_rect_tall, vec![2]),
            (image_ids[9], image_rect_tall, vec![2]),
            (image_ids[10], image_rect_tall, vec![2]),
            (image_ids[11], image_rect_tall, vec![2]),
        ]);
    }
}
