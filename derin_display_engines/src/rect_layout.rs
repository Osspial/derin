// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod image_slice;
pub mod text;
pub mod theme;

use derin_common_types::layout::{Margins, Align2, Align, SizeBounds};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};
use crate::cgmath::{Point2, Vector2, EuclideanSpace};
use crate::EditStringDecorations;
use theme::{Color, ImageId, TextRenderStyle};
use text::{FaceManager, StringLayoutData, TextToRects};
use image_slice::ImageSlicer;
use euclid::default::Size2D;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub rect: BoundBox<D2, i32>,
    pub fill: RectFill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RectFill {
    Color(Color),
    Image(ImageRectFill),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageRectFill {
    pub image_id: ImageId,
    pub subrect: BoundBox<D2, i32>,
}

impl RectFill {
    pub fn image_id(&self) -> Option<ImageId> {
        match *self {
            RectFill::Color(_) => None,
            RectFill::Image(irf) => Some(irf.image_id),
        }
    }
}

struct WidgetRectLayout<'a, F: FaceManager> {
    background: Option<ImageLaidOut>,
    text: Option<RectOffsetClip<TextToRects<'a, F>>>,
}

enum ImageLaidOut {
    Slice(ImageSlicer),
    Image(Rect),
}

pub trait ImageManager: 'static {
    type ImagePath: ?Sized;
    type ResolveImageError;
    fn resolve_image(&mut self, image_path: &Self::ImagePath) -> Result<ImageId, Self::ResolveImageError>;
    fn image_layout(&mut self, image_id: ImageId) -> Option<ImageLayout>;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageLayout {
    #[serde(default)]
    pub rescale: RescaleRules,
    #[serde(default)]
    pub dims: Size2D<u32>,
    #[serde(default)]
    pub size_bounds: SizeBounds,
    #[serde(default)]
    pub margins: Margins<i32>,
}

/// The algorithm used to rescale an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RescaleRules {
    /// Rescale the image by uniformly stretching it out, from its edges.
    Stretch,
    /// Perform nine-slicing on the provided image, stretching out the center of the image while
    /// keeping the borders of the image a constant size.
    Slice(Margins<i32>),
    Align(Align2),
}

impl Default for RescaleRules {
    fn default() -> RescaleRules {
        RescaleRules::Stretch
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ImageLayoutData {
    pub image_id: ImageId,
    pub rect: BoundBox<D2, i32>,
    pub rescale: RescaleRules,
    pub dims: DimsBox<D2, i32>,
}

#[derive(Debug, Clone)]
pub struct TextLayoutData<'a> {
    pub string_layout: &'a StringLayoutData,
    pub decorations: EditStringDecorations,
    pub render_style: TextRenderStyle,
    pub offset: Vector2<i32>,
    pub clip_rect: BoundBox<D2, i32>,
}

struct RectOffsetClip<R: Iterator<Item=Rect>> {
    offset: Vector2<i32>,
    clip_rect: BoundBox<D2, i32>,
    rects: R
}

pub fn layout_widget_rects<'a>(
        image: Option<ImageLayoutData>,
        text: Option<TextLayoutData<'a>>,
        face_manager: &'a mut impl FaceManager,
) -> impl 'a + Iterator<Item=Rect> {
    let background = try {
        let ImageLayoutData {
            image_id,
            rect,
            rescale,
            dims,
        } = image?;

        let image_laid_out = match rescale {
            RescaleRules::Stretch => ImageLaidOut::Image(Rect {
                rect,
                fill: RectFill::Image(ImageRectFill {
                    image_id,
                    subrect: dims.into(),
                })
            }),
            RescaleRules::Slice(margins) => ImageLaidOut::Slice(
                ImageSlicer::new(
                    image_id,
                    rect,
                    dims,
                    margins
                )
            ),
            RescaleRules::Align(alignment) => {
                let (min_x, max_x) = match alignment.x {
                    Align::Stretch => (rect.min.x, rect.max.x),
                    Align::Start => (rect.min.x, rect.min.x + dims.dims.x),
                    Align::End => (rect.max.x - dims.dims.x, rect.max.x),
                    Align::Center => {
                        let center = rect.center().x;
                        let length = dims.dims.x;
                        (
                            center - (length / 2),
                            center + (length / 2) + (length % 2)
                            // We add (length % 2) to make sure images with odd side lengths
                            // keep their exact length.
                        )
                    }
                };
                let (min_y, max_y) = match alignment.y {
                    Align::Stretch => (rect.min.y, rect.max.y),
                    Align::Start => (rect.min.y, rect.min.y + dims.dims.y),
                    Align::End => (rect.max.y - dims.dims.y, rect.max.y),
                    Align::Center => {
                        let center = rect.center().y;
                        let length = dims.dims.y;
                        (
                            center - (length / 2),
                            center + (length / 2) + (length % 2)
                            // We add (length % 2) to make sure images with odd side lengths
                            // keep their exact length.
                        )
                    }
                };
                ImageLaidOut::Image(Rect {
                    rect: BoundBox::new2(min_x, min_y, max_x, max_y),
                    fill: RectFill::Image(ImageRectFill {
                        image_id,
                        subrect: dims.into(),
                    })
                })
            }
        };
        image_laid_out
    };

    let text = try {
        let TextLayoutData {
            string_layout,
            decorations: EditStringDecorations {
                cursor_pos,
                highlight_range,
            },
            render_style,
            offset,
            clip_rect,
        } = text?;

        RectOffsetClip {
            offset,
            clip_rect,
            rects: TextToRects::new(
                string_layout,
                highlight_range,
                cursor_pos,
                render_style,
                face_manager
            ),
        }
    };

    WidgetRectLayout {
        background,
        text,
    }
}

pub fn transform_local_to_global_rects(
    local_rects: impl Iterator<Item=Rect>,
    widget_pos: Point2<i32>,
    clip_rect: BoundBox<D2, i32>,
) -> impl Iterator<Item=Rect> {
    RectOffsetClip {
        rects: local_rects,
        offset: widget_pos.to_vec(),
        clip_rect,
    }
}

impl<'a, F: FaceManager> Iterator for WidgetRectLayout<'a, F> {
    type Item = Rect;

    /// Returns the following, in order:
    /// 1. Background image rects
    /// 2. Text rects
    fn next(&mut self) -> Option<Rect> {
        let background_rect = match self.background {
            Some(ImageLaidOut::Slice(ref mut image_slice)) => {
                match image_slice.next() {
                    Some(rect) => Some(rect),
                    None => {
                        self.background = None;
                        None
                    }
                }
            },
            Some(ImageLaidOut::Image(rect)) => {
                self.background = None;
                Some(rect)
            },
            None => None
        };

        if let Some(rect) = background_rect {
            return Some(rect);
        }

        self.text.as_mut().and_then(|t| t.next())
    }
}

impl<R: Iterator<Item=Rect>> Iterator for RectOffsetClip<R> {
    type Item = Rect;

    fn next(&mut self) -> Option<Rect> {
        let Rect {
            mut rect,
            mut fill,
        } = self.rects.next()?;

        rect = rect + self.offset;
        let rect_unclipped = rect;
        rect = rect.intersect_rect(self.clip_rect)?;

        match fill {
            RectFill::Color(_) => (),
            RectFill::Image(ImageRectFill{image_id: _, ref mut subrect}) => {
                subrect.min += rect.min - rect_unclipped.min;
                subrect.max += rect.max - rect_unclipped.max;
            }
        }

        Some(Rect {
            rect,
            fill,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::rects_from_string;

    #[test]
    fn test_offset_clip() {
        let clipped = "
            o-+
            | |
            +-o-----+
              |c---r|
              ||   ||
              |r---c|
              +-----+
        ";
        let clipped = rects_from_string(clipped, true);

        let base_rect = "
            r-----+
            |c---+|
            ||   ||
            |+---c|
            +-----r
        ";
        let base_rect = rects_from_string(base_rect, true);

        let image_id = ImageId::new();
        let unclipped_rect = base_rect[&'r'];
        let clipped_rect = clipped[&'r'];
        let clipped_subrect = base_rect[&'c'];
        let offset = clipped[&'o'].dims().dims;
        let clip_rect = clipped[&'c'];

        let rect = Rect {
            rect: unclipped_rect,
            fill: RectFill::Image(ImageRectFill {
                image_id,
                subrect: unclipped_rect,
            })
        };

        let roc = RectOffsetClip {
            offset,
            clip_rect,
            rects: Some(rect).into_iter(),
        };
        let offset_clipped_rects = roc.collect::<Vec<_>>();

        assert_eq!(
            offset_clipped_rects,
            vec![
                Rect {
                    rect: clipped_rect,
                    fill: RectFill::Image(ImageRectFill {
                        image_id,
                        subrect: clipped_subrect
                    })
                }
            ]
        );
    }
}
