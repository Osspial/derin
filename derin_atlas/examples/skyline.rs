// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin_atlas;
extern crate image;
extern crate cgmath_geometry;

use std::slice;
use derin_atlas::SkylineAtlas;
use image::{DynamicImage, ColorType};
use cgmath_geometry::{D2, rect::DimsBox};

fn main() {
    let mut atlas = SkylineAtlas::new([0; 4], DimsBox::new2(512, 512));
    let doge = extract_buffer(image::open("test_images/doge.png").unwrap());
    let ffx = extract_buffer(image::open("test_images/ffx.png").unwrap());
    let rust = extract_buffer(image::open("test_images/rust.png").unwrap());
    let tf = extract_buffer(image::open("test_images/tf.png").unwrap());

    let mut rectangles = vec![];

    for _ in 0..4 {
        rectangles.push(atlas.add_image(ffx.0, ffx.0.into(), &ffx.1).unwrap());
        rectangles.push(atlas.add_image(tf.0, tf.0.into(), &tf.1).unwrap());
    }

    rectangles.push(atlas.add_image(doge.0, doge.0.into(), &doge.1).unwrap());
    rectangles.push(atlas.add_image(ffx.0, ffx.0.into(), &ffx.1).unwrap());
    rectangles.push(atlas.add_image(rust.0, rust.0.into(), &rust.1).unwrap());
    rectangles.push(atlas.add_image(tf.0, tf.0.into(), &tf.1).unwrap());
    rectangles.push(atlas.add_image(ffx.0, ffx.0.into(), &ffx.1).unwrap());

    rectangles.push(atlas.add_image(doge.0, doge.0.into(), &doge.1).unwrap());
    rectangles.push(atlas.add_image(ffx.0, ffx.0.into(), &ffx.1).unwrap());
    rectangles.push(atlas.add_image(rust.0, rust.0.into(), &rust.1).unwrap());
    rectangles.push(atlas.add_image(tf.0, tf.0.into(), &tf.1).unwrap());
    rectangles.push(atlas.add_image(ffx.0, ffx.0.into(), &ffx.1).unwrap());

    println!();
    atlas.compact(&mut rectangles);

    let pixels = atlas.pixels();
    image::save_buffer("./skyline_atlas.bmp", unsafe{slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)}, 512, 512, ColorType::RGBA(8)).unwrap();
}

fn extract_buffer(img: DynamicImage) -> (DimsBox<D2, u32>, Vec<[u8; 4]>) {
    match img {
        DynamicImage::ImageRgba8(img) => {
            let rect = DimsBox::new2(img.width(), img.height());
            println!("{:?}", rect);
            (rect, img.pixels().map(|p| p.data).collect())
        },
        _ => unimplemented!()
    }
}
