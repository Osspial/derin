// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin_atlas;
extern crate image;
extern crate cgmath_geometry;
extern crate rand;

use crate::cgmath_geometry::rect::GeoBox;
use std::slice;
use derin_atlas::{SkylineAtlas, PerimeterAtlas};
use image::{DynamicImage, ColorType};
use cgmath_geometry::{D2, rect::DimsBox};
use rand::prelude::*;

fn main() {
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

    let output_atlas = |sky: &SkylineAtlas<_>, perimeter: &PerimeterAtlas<_>, iteration| {
        let sky = sky.pixels();
        let per = perimeter.pixels();
        let mut pixels = sky.to_vec();
        pixels.extend_from_slice(per);
        // let (edges_dims, edges) = atlas.edge_image([32; 4], |i| [i as u8, 255, 255, 255]);
        // let combined = pixels.chunks(512).flat_map(|p| p.iter().cloned().chain(Some([0; 4]))).chain(edges.iter().cloned()).collect::<Vec<_>>();
        image::save_buffer(
            format!("./out/compete_{}.bmp", iteration),
            unsafe{slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4)},
            512, 1024,
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

    let mut images = vec![];
    let mut used_pixels = 0;
    loop {
        let (dims, pixels) = gen_image();
        used_pixels += dims.width() * dims.height();
        if used_pixels >= 512*512 {
            break;
        }
        images.push((dims, pixels));
    }

    let mut sky = SkylineAtlas::new(DimsBox::new2(512, 512), [0; 4]);
    let mut per = PerimeterAtlas::new(DimsBox::new2(512, 512), [0; 4]);
    let mut sky_count = 0;
    let mut per_count = 0;
    for (i, image) in images.iter().enumerate() {
        let sky_rect = sky.add_image(image.0, &image.1);
        let per_rect = per.add_image(image.0, &image.1);

        sky_count += sky_rect.is_some() as u32;
        per_count += per_rect.is_some() as u32;
        output_atlas(&sky, &per, i);
    }

    println!("unsorted: {} {}", sky_count, per_count);

    images.sort_unstable_by_key(|i| -(i.0.height() as i32 * i.0.width() as i32));

    let mut sky = SkylineAtlas::new(DimsBox::new2(512, 512), [0; 4]);
    let mut per = PerimeterAtlas::new(DimsBox::new2(512, 512), [0; 4]);
    let mut sky_count = 0;
    let mut per_count = 0;
    for (i, image) in images.iter().enumerate() {
        let sky_rect = sky.add_image(image.0, &image.1);
        let per_rect = per.add_image(image.0, &image.1);

        sky_count += sky_rect.is_some() as u32;
        per_count += per_rect.is_some() as u32;
        output_atlas(&sky, &per, i + images.len() * 3);
    }

    println!("sorted: {} {}", sky_count, per_count);

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
