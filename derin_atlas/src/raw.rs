use cgmath_geometry::{D2, rect::{DimsBox, GeoBox}};
use crate::cgmath::{Vector2};
use itertools::Itertools;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAtlas<P: 'static + Copy> {
    pixels: Box<[P]>
}

impl<P: Copy> RawAtlas<P> {
    pub fn new(atlas_dims: DimsBox<D2, u32>, background_color: P) -> RawAtlas<P> {
        RawAtlas {
            pixels: vec![background_color; (atlas_dims.width() * atlas_dims.height()) as usize].into_boxed_slice(),
        }
    }

    pub fn pixels(&self) -> &[P] {
        &self.pixels
    }

    pub fn blit_slice_iter<'a, I: IntoIterator<Item=&'a [P]>>(
        &mut self,
        atlas_dims: DimsBox<D2, u32>,
        src: I,
        src_dims: DimsBox<D2, u32>,
        dst_offset: Vector2<u32>
    ) {
        blit_slice_iter(
            src, src_dims,
            &mut self.pixels,
            atlas_dims,
            dst_offset,
        );
    }

    pub fn blit_pixel_iter<I: IntoIterator<Item = P>>(
        &mut self,
        atlas_dims: DimsBox<D2, u32>,
        src: I,
        src_dims: DimsBox<D2, u32>,
        dst_offset: Vector2<u32>
    ) {
        blit_pixel_iter(
            src, src_dims,
            &mut self.pixels,
            atlas_dims,
            dst_offset,
        );
    }

    pub fn clear(&mut self, background_color: P) {
        for pixel in &mut *self.pixels {
            *pixel = background_color;
        }
    }
}

fn blit_slice_iter<'a, P: 'a + Copy, I: IntoIterator<Item=&'a [P]>>(
    src: I, src_dims: DimsBox<D2, u32>,
    dst: &mut [P], dst_dims: DimsBox<D2, u32>, dst_offset: Vector2<u32>
) {
    let (mut width, mut height) = (src_dims.width(), 0);
    for (row_num, src_row) in src.into_iter().enumerate() {
        let dst_row_num = row_num + dst_offset.y as usize;
        let dst_slice_offset = dst_row_num * dst_dims.width() as usize;
        let dst_row = &mut dst[dst_slice_offset..dst_slice_offset + dst_dims.width() as usize];

        let dst_copy_to_slice = &mut dst_row[dst_offset.x as usize..dst_offset.x as usize + src_row.len()];
        dst_copy_to_slice.copy_from_slice(src_row);

        height += 1;
        width &= src_row.len() as u32;
    }

    assert_eq!(src_dims, DimsBox::new2(width, height));
}

fn blit_pixel_iter<P, I>(
    src: I, src_dims: DimsBox<D2, u32>,
    dst: &mut [P], dst_dims: DimsBox<D2, u32>, dst_offset: Vector2<u32>
)
    where I: IntoIterator<Item=P>,
{
    let (mut width, mut height) = (src_dims.width(), 0);
    for (row_num, src_row) in src.into_iter().chunks(src_dims.width() as usize).into_iter().enumerate() {
        let dst_row_num = row_num + dst_offset.y as usize;
        let dst_slice_offset = dst_row_num * dst_dims.width() as usize;
        let dst_row = &mut dst[dst_slice_offset..dst_slice_offset + dst_dims.width() as usize];

        let dst_copy_to_slice = &mut dst_row[dst_offset.x as usize..];
        let mut src_row_len = 0;

        for (p, v) in dst_copy_to_slice.iter_mut().zip(src_row.into_iter()) {
            *p = v;
            src_row_len += 1;
        }

        height += 1;
        width &= src_row_len;
    }

    assert_eq!(src_dims, DimsBox::new2(width, height));
}
