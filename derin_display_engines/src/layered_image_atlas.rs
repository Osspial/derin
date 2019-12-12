use cgmath_geometry::{D2, rect::DimsBox};
use derin_atlas::SkylineAtlas;
use crate::{
    HasLifetimeIterator,
    gullery_display_engine::{LayeredImageAtlas, Image, ImageAtlasRect},
    rect_layout::{
        theme::{Color, ImageId, WidgetStyle},
    },
};
use std::collections::HashMap;

pub struct SkylineLayeredImageAtlas {
    layers: Vec<SkylineAtlas<Color>>,
    images: HashMap<ImageId, ImageCoords>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ImageCoords {
    SingleLayer(ImageAtlasRect),
    MultiLayer(Box<[ImageAtlasRect]>),
}

impl LayeredImageAtlas for SkylineLayeredImageAtlas {
    fn add_image(&mut self, image: ImageId, dims: DimsBox<D2, u32>, pixels: impl Iterator<Item=Color>) {
        unimplemented!()
    }
    fn image_coords(&self, image: ImageId) -> Option<Image<<Self as HasLifetimeIterator<'_, ImageAtlasRect>>::Iter>> {
        unimplemented!()
    }
    fn layers(&self) -> <Self as HasLifetimeIterator<'_, &'_ [Color]>>::Iter {
        unimplemented!()
    }
    fn num_layers(&self) -> u16 {
        unimplemented!()
    }
    fn layer_dims(&self) -> DimsBox<D2, u32> {
        unimplemented!()
    }
    fn clean(&mut self) {
        unimplemented!()
    }
    fn updated_since_clean(&self) -> bool {
        unimplemented!()
    }
}

impl<'a> HasLifetimeIterator<'a, &'a [Color]> for SkylineLayeredImageAtlas {
    type Iter = std::iter::Once<&'a [Color]>;
}

impl<'a> HasLifetimeIterator<'a, ImageAtlasRect> for SkylineLayeredImageAtlas {
    type Iter = std::iter::Once<ImageAtlasRect>;
}
