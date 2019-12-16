// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin_atlas;
extern crate image;
extern crate cgmath_geometry;

use std::slice;
use derin_atlas::PerimeterAtlas;
use image::{DynamicImage, ColorType};
use cgmath_geometry::{D2, rect::DimsBox};

fn main() {
    let mut atlas = PerimeterAtlas::new(DimsBox::new2(512, 512), [0; 4]);
    let doge = extract_buffer(image::open("test_images/doge.png").unwrap());
    let ffx = extract_buffer(image::open("test_images/ffx.png").unwrap());
    let rust = extract_buffer(image::open("test_images/rust.png").unwrap());
    let tf = extract_buffer(image::open("test_images/tf.png").unwrap());

    let mut rectangles = vec![];

    let mut iteration = 0;
    let mut output_atlas = |atlas: &PerimeterAtlas<_>| {
        let pixels = atlas.pixels();
        let (edges_dims, edges) = atlas.edge_image([32; 4], [255; 4]);
        let combined = pixels.chunks(512).flat_map(|p| p.iter().cloned().chain(Some([0; 4]))).chain(edges.iter().cloned()).collect::<Vec<_>>();
        image::save_buffer(
            format!("./out/perimeter_atlas_{}.bmp", iteration),
            unsafe{slice::from_raw_parts(combined.as_ptr() as *const u8, combined.len() * 4)},
            513, 512 + edges_dims.dims.y,
            ColorType::RGBA(8)
        ).unwrap();

        if iteration == 14 {
            std::thread::sleep_ms(3000);
        }

        // image::save_buffer(
        //     format!("./out/perimeter_atlas_edges_{}.bmp", iteration),
        //     unsafe{slice::from_raw_parts(edges.as_ptr() as *const u8, edges.len() * 4)},
        //     edges_dims.dims.x, edges_dims.dims.y,
        //     ColorType::RGBA(8)
        // ).unwrap();
        iteration += 1;
    };

    for _ in 0..4 {
        rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
        output_atlas(&atlas);
        rectangles.push(atlas.add_image(tf.0, &tf.1).unwrap());
        output_atlas(&atlas);
    }

    rectangles.push(atlas.add_image(doge.0, &doge.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(rust.0, &rust.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(tf.0, &tf.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    output_atlas(&atlas);

    rectangles.push(atlas.add_image(doge.0, &doge.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(rust.0, &rust.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(tf.0, &tf.1).unwrap());
    output_atlas(&atlas);
    rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    output_atlas(&atlas);

    // println!();
    // atlas.compact(&mut rectangles);
    // output_atlas(&atlas);
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
