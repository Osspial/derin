// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin_atlas;
extern crate image;
extern crate cgmath_geometry;
extern crate rand;

use crate::cgmath_geometry::rect::GeoBox;
use std::slice;
use derin_atlas::SkylineAtlas;
use image::{DynamicImage, ColorType};
use cgmath_geometry::{D2, rect::DimsBox};
use rand::prelude::*;

fn main() {
    let mut atlas = SkylineAtlas::new(DimsBox::new2(512, 512), [0; 4]);
    let mut rng = rand::thread_rng();
    let mut gen_image = || {
        let dims = DimsBox::new2(
            rng.gen_range(4, 128),
            rng.gen_range(4, 128),
        );
        let color = [
            rng.gen_range(8, 256) as u8,
            rng.gen_range(8, 256) as u8,
            rng.gen_range(8, 256) as u8,
            rng.gen_range(8, 256) as u8,
        ];
        (
            dims,
            vec![color; (dims.dims.x * dims.dims.y) as usize]
        )
    };

    let output_atlas = |atlas: &SkylineAtlas<_>, iteration| {
        let pixels = atlas.pixels();
        // let (edges_dims, edges) = atlas.edge_image([32; 4], |i| [i as u8, 255, 255, 255]);
        // let combined = pixels.chunks(512).flat_map(|p| p.iter().cloned().chain(Some([0; 4]))).chain(edges.iter().cloned()).collect::<Vec<_>>();
        image::save_buffer(
            format!("./out/skyline_atlas_{}.bmp", iteration),
            unsafe{slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)},
            512, 512,
            ColorType::RGBA(8)
        ).unwrap();

        // if iteration == 14 {
        //     std::thread::sleep_ms(3000);
        // }

        // image::save_buffer(
        //     format!("./out/perimeter_atlas_edges_{}.bmp", iteration),
        //     unsafe{slice::from_raw_parts(edges.as_ptr() as *const u8, edges.len() * 4)},
        //     edges_dims.dims.x, edges_dims.dims.y,
        //     ColorType::RGBA(8)
        // ).unwrap();
    };

    let mut rectangles = vec![];

    for i in 0.. {
        let image = gen_image();
        println!("\n\nITERATION {:?}", i);
        let rect = atlas.add_image(image.0, &image.1);
        let rect = match rect {
            Some(rect) => rect,
            None => break,
        };
        rectangles.push(rect);
        output_atlas(&atlas, i);
        // atlas.verify();

        use itertools::Itertools;

        for (a, b) in rectangles.iter().cloned().tuple_combinations() {
            assert!(a.intersect_rect(b).overlaps().is_none());
        }
    }

    // for _ in 0..4 {
    //     rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    //     output_atlas(&atlas);
    //     rectangles.push(atlas.add_image(tf.0, &tf.1).unwrap());
    //     output_atlas(&atlas);
    // }

    // rectangles.push(atlas.add_image(doge.0, &doge.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(rust.0, &rust.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(tf.0, &tf.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    // output_atlas(&atlas);

    // rectangles.push(atlas.add_image(doge.0, &doge.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(rust.0, &rust.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(tf.0, &tf.1).unwrap());
    // output_atlas(&atlas);
    // rectangles.push(atlas.add_image(ffx.0, &ffx.1).unwrap());
    // output_atlas(&atlas);

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
