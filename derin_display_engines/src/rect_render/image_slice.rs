use super::{
    Rect, RectFill,
    theme::ImageId,
};
use cgmath_geometry::{
    D2,
    cgmath::{Point2, Vector2},
    rect::{BoundBox, DimsBox, GeoBox},
};
use derin_common_types::layout::Margins;

pub struct ImageSlicer {
    image_id: ImageId,
    image_dims: DimsBox<D2, i32>,
    rect: BoundBox<D2, i32>,
    margins: Margins<i32>,
    index: u8,
}

impl ImageSlicer {
    pub fn new(image_id: ImageId, rect: BoundBox<D2, i32>, image_dims: DimsBox<D2, i32>, margins: Margins<i32>) -> ImageSlicer {
        ImageSlicer {
            image_id,
            image_dims,
            rect,
            margins,
            index: 0,
        }
    }
}

impl Iterator for ImageSlicer {
    type Item = Rect;

    fn next(&mut self) -> Option<Rect> {
        let rect = self.rect;
        let image_dims = self.image_dims;
        let margins = self.margins;
        let image_id = self.image_id;

        // 00--01---02--03
        // |   |    |   |
        // |   |    |   |
        // 10--11---12--13
        // |   |    |   |
        // |   |    |   |
        // |   |    |   |
        // 20--21---22--23
        // |   |    |   |
        // |   |    |   |
        // 30--31---32--33

        // Output rect coordinates
        let x_0 = rect.min.x;
        let x_1 = rect.min.x + margins.left;
        let x_2 = rect.max.x - margins.right;
        let x_3 = rect.max.x;

        let y_0 = rect.min.y;
        let y_1 = rect.min.y + margins.top;
        let y_2 = rect.max.y - margins.bottom;
        let y_3 = rect.max.y;

        let raster_00 = Point2::new(x_0, y_0);
        let raster_01 = Point2::new(x_1, y_0);
        let raster_02 = Point2::new(x_2, y_0);
        // let raster_03 = Point2::new(x_3, y_0);

        let raster_10 = Point2::new(x_0, y_1);
        let raster_11 = Point2::new(x_1, y_1);
        let raster_12 = Point2::new(x_2, y_1);
        let raster_13 = Point2::new(x_3, y_1);

        let raster_20 = Point2::new(x_0, y_2);
        let raster_21 = Point2::new(x_1, y_2);
        let raster_22 = Point2::new(x_2, y_2);
        let raster_23 = Point2::new(x_3, y_2);

        // let raster_30 = Point2::new(x_0, y_3);
        let raster_31 = Point2::new(x_1, y_3);
        let raster_32 = Point2::new(x_2, y_3);
        let raster_33 = Point2::new(x_3, y_3);


        // Sample rect coordinates
        let x_0 = 0;
        let x_1 = margins.left;
        let x_2 = image_dims.width() - margins.right;
        let x_3 = image_dims.width();

        let y_0 = 0;
        let y_1 = margins.top;
        let y_2 = image_dims.height() - margins.bottom;
        let y_3 = image_dims.height();

        let sample_00 = Point2::new(x_0, y_0);
        let sample_01 = Point2::new(x_1, y_0);
        let sample_02 = Point2::new(x_2, y_0);
        // let sample_03 = Point2::new(x_3, y_0);

        let sample_10 = Point2::new(x_0, y_1);
        let sample_11 = Point2::new(x_1, y_1);
        let sample_12 = Point2::new(x_2, y_1);
        let sample_13 = Point2::new(x_3, y_1);

        let sample_20 = Point2::new(x_0, y_2);
        let sample_21 = Point2::new(x_1, y_2);
        let sample_22 = Point2::new(x_2, y_2);
        let sample_23 = Point2::new(x_3, y_2);

        // let sample_30 = Point2::new(x_0, y_3);
        let sample_31 = Point2::new(x_1, y_3);
        let sample_32 = Point2::new(x_2, y_3);
        let sample_33 = Point2::new(x_3, y_3);


        // Offset the rect corners for the inner slices so that we only sample from each pixel once.
        // We use `*_exists` so that the center rect doesn't get offset if a side doesn't have
        // margins.
        let left_exists = (margins.left != 0) as i32;
        let top_exists = (margins.top != 0) as i32;
        let right_exists = (margins.right != 0) as i32;
        let bottom_exists = (margins.bottom != 0) as i32;

        let ul = Vector2::new(-right_exists, -bottom_exists);
        let uz = Vector2::new( 0           , -bottom_exists);
        // let ur = Vector2::new( left_exists , -bottom_exists);
        let zr = Vector2::new( left_exists ,  0            );
        let dr = Vector2::new( left_exists ,  top_exists   );
        let dz = Vector2::new( 0           ,  top_exists   );
        // let dl = Vector2::new(-right_exists,  top_exists   );
        let zl = Vector2::new(-right_exists,  0            );

        // let ul = Vector2::new(-1, -1);
        // let uz = Vector2::new( 0, -1);
        // // let ur = Vector2::new( 1, -1);
        // let zr = Vector2::new( 1,  0);
        // let dr = Vector2::new( 1,  1);
        // let dz = Vector2::new( 0,  1);
        // // let dl = Vector2::new(-1, 1);
        // let zl = Vector2::new(-1,  0);

        let ret = match self.index {
            // ■□□
            // □□□
            // □□□
            0 => Some(Rect {
                rect: BoundBox::new(raster_00, raster_11),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_00, sample_11),
                }
            }),
            // □■□
            // □□□
            // □□□
            1 => Some(Rect {
                rect: BoundBox::new(raster_01 + zr, raster_12 + zl),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_01 + zr, sample_12 + zl),
                }
            }),
            // □□■
            // □□□
            // □□□
            2 => Some(Rect {
                rect: BoundBox::new(raster_02, raster_13),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_02, sample_13),
                }
            }),
            // □□□
            // □□■
            // □□□
            3 => Some(Rect {
                rect: BoundBox::new(raster_12 + dz, raster_23 + uz),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_12 + dz, sample_23 + uz),
                }
            }),
            // □□□
            // □□□
            // □□■
            4 => Some(Rect {
                rect: BoundBox::new(raster_22, raster_33),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_22, sample_33),
                }
            }),
            // □□□
            // □□□
            // □■□
            5 => Some(Rect {
                rect: BoundBox::new(raster_21 + zr, raster_32 + zl),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_21 + zr, sample_32 + zl),
                },
            }),
            // □□□
            // □□□
            // ■□□
            6 => Some(Rect {
                rect: BoundBox::new(raster_20, raster_31),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_20, sample_31),
                },
            }),
            // □□□
            // ■□□
            // □□□
            7 => Some(Rect {
                rect: BoundBox::new(raster_10 + dz, raster_21 + uz),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_10 + dz, sample_21 + uz),
                },
            }),
            // □□□
            // □■□
            // □□□
            8 => Some(Rect {
                rect: BoundBox::new(raster_11 + dr, raster_22 + ul),
                fill: RectFill::Image {
                    image_id,
                    subrect: BoundBox::new(sample_11 + dr, sample_22 + ul),
                }
            }),
            9 => None,
            _ => unreachable!()
        };

        self.index += 1;

        // Ignore any zero-width or zero-height rects.
        if ret.map(|r| r.rect.width() != 0 && r.rect.height() != 0).unwrap_or(true) {
            ret
        } else {
            self.next()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::rects_from_string;

    #[test]
    fn simple_nine_slice() {
        let raster = "
            0---+1-----+2--r
            |   ||     ||  |
            |   ||     ||  |
            |   ||     ||  |
            +---0+-----1+--2
            7---+8-----+3--+
            |   ||     ||  |
            |   ||     ||  |
            |   ||     ||  |
            +---7+-----8+--3
            6---+5-----+4--+
            |   ||     ||  |
            r---6+-----5+--4
        ";
        let raster = rects_from_string(raster, false);

        let sample = "
            0---+1-+2--s
            |   || ||  |
            |   || ||  |
            |   || ||  |
            +---0+-1+--2
            7---+8-+3--+
            |   || ||  |
            |   || ||  |
            +---7+-8+--3
            6---+5-+4--+
            |   || ||  |
            s---6+-5+--4
        ";
        let sample = rects_from_string(sample, true);

        let image_id = ImageId::new();
        let image_dims = sample[&'s'].dims();
        let raster_rect = raster[&'r'];
        let margins = Margins::new(4, 4, 3, 2);
        let slicer = ImageSlicer::new(
                image_id,
                raster_rect,
                image_dims,
                margins
            );

        assert_eq!(
            slicer.collect::<Vec<_>>(),
            vec![
                // ■□□
                // □□□
                // □□□
                Rect {
                    rect: raster[&'0'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'0'],
                    }
                },
                // □■□
                // □□□
                // □□□
                Rect {
                    rect: raster[&'1'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'1'],
                    }
                },
                // □□■
                // □□□
                // □□□
                Rect {
                    rect: raster[&'2'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'2'],
                    }
                },
                // □□□
                // □□■
                // □□□
                Rect {
                    rect: raster[&'3'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'3'],
                    }
                },
                // □□□
                // □□□
                // □□■
                Rect {
                    rect: raster[&'4'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'4'],
                    }
                },
                // □□□
                // □□□
                // □■□
                Rect {
                    rect: raster[&'5'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'5'],
                    }
                },
                // □□□
                // □□□
                // ■□□
                Rect {
                    rect: raster[&'6'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'6'],
                    }
                },
                // □□□
                // ■□□
                // □□□
                Rect {
                    rect: raster[&'7'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'7'],
                    }
                },
                // □□□
                // □■□
                // □□□
                Rect {
                    rect: raster[&'8'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'8'],
                    }
                },
            ],
        );
    }

    #[test]
    fn no_slice() {
        let raster = "
            0---r
            |   |
            |   |
            |   |
            r---0
        ";
        let raster = rects_from_string(raster, false);

        let sample = "
            0-s
            | |
            s-0
        ";
        let sample = rects_from_string(sample, true);

        let image_id = ImageId::new();
        let image_dims = sample[&'s'].dims();
        let raster_rect = raster[&'r'];
        let margins = Margins::new(0, 0, 0, 0);
        let slicer = ImageSlicer::new(
                image_id,
                raster_rect,
                image_dims,
                margins
            );

        assert_eq!(
            slicer.collect::<Vec<_>>(),
            vec![
                // □□□
                // □■□
                // □□□
                Rect {
                    rect: raster[&'0'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'0'],
                    }
                },
            ]
        );
    }

    #[test]
    fn partial_slice_left() {
        let raster = "
            +-+
            | |
            +-0-+1-r
              | || |
              | || |
              | || |
              | || |
              r-0+-1
        ";
        let raster = rects_from_string(raster, true);

        let sample = "
            0-+1s
            | |||
            | |||
            | |||
            | |||
            s-0+1
        ";
        let sample = rects_from_string(sample, true);

        let image_id = ImageId::new();
        let image_dims = sample[&'s'].dims();
        let raster_rect = raster[&'r'];
        let margins = Margins::new(2, 0, 0, 0);
        let slicer = ImageSlicer::new(
                image_id,
                raster_rect,
                image_dims,
                margins
            );

        assert_eq!(
            slicer.collect::<Vec<_>>(),
            vec![
                // □□□
                // ■□□
                // □□□
                Rect {
                    rect: raster[&'0'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'0'],
                    }
                },
                // □□□
                // □■□
                // □□□
                Rect {
                    rect: raster[&'1'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'1'],
                    }
                },
            ]
        );
    }

    #[test]
    fn partial_slice_top() {
        let raster = "
            +-+
            | |
            +-0----r
              |    |
              +----0
              1----+
              |    |
              r----1
        ";
        let raster = rects_from_string(raster, false);

        let sample = "
            0----s
            |    |
            +----0
            1----+
            s----1
        ";
        let sample = rects_from_string(sample, true);

        let image_id = ImageId::new();
        let image_dims = sample[&'s'].dims();
        let raster_rect = raster[&'r'];
        let margins = Margins::new(0, 2, 0, 0);
        let slicer = ImageSlicer::new(
                image_id,
                raster_rect,
                image_dims,
                margins
            );

        assert_eq!(
            slicer.collect::<Vec<_>>(),
            vec![
                // □□□
                // ■□□
                // □□□
                Rect {
                    rect: raster[&'0'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'0'],
                    }
                },
                // □□□
                // □■□
                // □□□
                Rect {
                    rect: raster[&'1'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'1'],
                    }
                },
            ]
        );
    }

    #[test]
    fn partial_slice_right() {
        let raster = "
            +-+
            | |
            +-1-+0-r
              | || |
              | || |
              | || |
              | || |
              r-1+-0
        ";
        let raster = rects_from_string(raster, false);

        let sample = "
            1+0-s
            ||| |
            ||| |
            ||| |
            ||| |
            s1+-0
        ";
        let sample = rects_from_string(sample, true);

        let image_id = ImageId::new();
        let image_dims = sample[&'s'].dims();
        let raster_rect = raster[&'r'];
        let margins = Margins::new(0, 0, 2, 0);
        let slicer = ImageSlicer::new(
                image_id,
                raster_rect,
                image_dims,
                margins
            );

        assert_eq!(
            slicer.collect::<Vec<_>>(),
            vec![
                // □□□
                // ■□□
                // □□□
                Rect {
                    rect: raster[&'0'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'0'],
                    }
                },
                // □□□
                // □■□
                // □□□
                Rect {
                    rect: raster[&'1'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'1'],
                    }
                },
            ]
        );
    }

    #[test]
    fn partial_slice_bottom() {
        let raster = "
            +-+
            | |
            +-1----r
              |    |
              +----1
              0----+
              |    |
              r----0
        ";
        let raster = rects_from_string(raster, false);

        let sample = "
            1----s
            +----1
            0----+
            |    |
            s----0
        ";
        let sample = rects_from_string(sample, true);

        let image_id = ImageId::new();
        let image_dims = sample[&'s'].dims();
        let raster_rect = raster[&'r'];
        let margins = Margins::new(0, 0, 0, 2);
        let slicer = ImageSlicer::new(
                image_id,
                raster_rect,
                image_dims,
                margins
            );

        assert_eq!(
            slicer.collect::<Vec<_>>(),
            vec![
                // □□□
                // ■□□
                // □□□
                Rect {
                    rect: raster[&'0'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'0'],
                    }
                },
                // □□□
                // □■□
                // □□□
                Rect {
                    rect: raster[&'1'],
                    fill: RectFill::Image {
                        image_id,
                        subrect: sample[&'1'],
                    }
                },
            ]
        );
    }
}
