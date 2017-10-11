extern crate dat;
extern crate image;
extern crate cgmath_geometry;

use dat::SkylineAtlas;
use image::{DynamicImage, ColorType};
use cgmath_geometry::DimsRect;

fn main() {
    let mut atlas = SkylineAtlas::new(4, DimsRect::new(512, 512));
    let doge = extract_buffer(image::open("test_images/doge.png").unwrap());
    let ffx = extract_buffer(image::open("test_images/ffx.png").unwrap());
    let rust = extract_buffer(image::open("test_images/rust.png").unwrap());
    let tf = extract_buffer(image::open("test_images/tf.png").unwrap());

    for _ in 0..4 {
        atlas.add_image(ffx.0, &ffx.1).unwrap();
        atlas.add_image(tf.0, &tf.1).unwrap();
    }

    for _ in 0..2 {
        atlas.add_image(doge.0, &doge.1).unwrap();
        atlas.add_image(ffx.0, &ffx.1).unwrap();
        atlas.add_image(rust.0, &rust.1).unwrap();
        atlas.add_image(tf.0, &tf.1).unwrap();
        atlas.add_image(ffx.0, &ffx.1).unwrap();
    }

    for _ in 0..5 {
        atlas.add_image(tf.0, &tf.1).unwrap();
    }

    image::save_buffer("./skyline_atlas.bmp", atlas.pixels(), 512, 512, ColorType::RGBA(8)).unwrap();
}

fn extract_buffer(img: DynamicImage) -> (DimsRect<u32>, Vec<u8>) {
    match img {
        DynamicImage::ImageRgba8(img) => {
            let rect = DimsRect::new(img.width(), img.height());
            println!("{:?}", rect);
            (rect, img.into_raw())
        },
        _ => unimplemented!()
    }
}
