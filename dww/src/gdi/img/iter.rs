use dct::geometry::Px;

use std::mem;

#[derive(Clone)]
pub struct ImageLineIter<'a> {
    slice: &'a [u8],
    width_bytes: usize,
    width_bytes_padded: usize
}

pub struct ImageLineIterMut<'a> {
    slice: &'a mut [u8],
    width_bytes: usize,
    width_bytes_padded: usize
}

impl<'a> ImageLineIter<'a> {
    #[inline]
    pub(super) fn new(slice: &'a [u8], width_bytes: Px, width_bytes_padded: usize) -> ImageLineIter<'a> {
        assert!(width_bytes_padded <= slice.len());
        assert!(width_bytes as usize <= width_bytes_padded);
        if width_bytes_padded != slice.len() {
            assert_ne!(width_bytes_padded, 0);
        }

        ImageLineIter {
            slice, width_bytes_padded,
            width_bytes: width_bytes as usize
        }
    }
}

impl<'a> ImageLineIterMut<'a> {
    #[inline]
    pub(super) fn new(slice: &'a mut [u8], width_bytes: Px, width_bytes_padded: usize) -> ImageLineIterMut<'a> {
        assert!(width_bytes_padded <= slice.len());
        assert!(width_bytes as usize <= width_bytes_padded);
        if width_bytes_padded != slice.len() {
            assert_ne!(width_bytes_padded, 0);
        }

        ImageLineIterMut {
            slice, width_bytes_padded,
            width_bytes: width_bytes as usize
        }
    }
}

impl<'a> Iterator for ImageLineIter<'a> {
    type Item = &'a [u8];

    #[inline]
    fn next(&mut self) -> Option<&'a [u8]> {
        if self.width_bytes_padded <= self.slice.len() {
            let (slice_ret, slice_iter) = self.slice.split_at(self.width_bytes_padded);
            self.slice = slice_iter;

            Some(&slice_ret[..self.width_bytes])
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.slice.len() / self.width_bytes_padded;
        (size, Some(size))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }

    #[inline]
    fn last(mut self) -> Option<&'a [u8]> {
        self.next_back()
    }
}
impl<'a> DoubleEndedIterator for ImageLineIter<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a [u8]> {
        if self.width_bytes_padded <= self.slice.len() {
            let split_index = self.slice.len() - self.width_bytes_padded;
            let (slice_iter, slice_ret) = self.slice.split_at(split_index);
            self.slice = slice_iter;

            Some(&slice_ret[..self.width_bytes])
        } else {
            None
        }
    }
}
impl<'a> ExactSizeIterator for ImageLineIter<'a> {}

impl<'a> Iterator for ImageLineIterMut<'a> {
    type Item = &'a mut [u8];

    #[inline]
    fn next(&mut self) -> Option<&'a mut [u8]> {
        if self.width_bytes_padded <= self.slice.len() {
            let tmp = mem::replace(&mut self.slice, &mut []);
            let (slice_ret, slice_iter) = tmp.split_at_mut(self.width_bytes_padded);
            self.slice = slice_iter;

            Some(&mut slice_ret[..self.width_bytes])
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.slice.len() / self.width_bytes_padded;
        (size, Some(size))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }

    #[inline]
    fn last(mut self) -> Option<&'a mut [u8]> {
        self.next_back()
    }
}
impl<'a> DoubleEndedIterator for ImageLineIterMut<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a mut [u8]> {
        if self.width_bytes_padded <= self.slice.len() {
            let tmp = mem::replace(&mut self.slice, &mut []);
            let split_index = tmp.len() - self.width_bytes_padded;
            let (slice_iter, slice_ret) = tmp.split_at_mut(split_index);
            self.slice = slice_iter;

            Some(&mut slice_ret[..self.width_bytes])
        } else {
            None
        }
    }
}
impl<'a> ExactSizeIterator for ImageLineIterMut<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::TestResult;

    macro_rules! check_iter_input_discard {
        ($image_data:expr, $width_bytes:expr, $width_bytes_padded:expr) => {
            if $width_bytes_padded < $width_bytes as usize ||
               $image_data.len() < $width_bytes_padded ||
               $width_bytes_padded == 0 || $width_bytes == 0 ||
               $image_data.len() % $width_bytes_padded != 0
            {
                return TestResult::discard();
            }
        }
    }

    quickcheck!{
        fn image_line_iter(image_data: Vec<u8>, width_bytes: Px, width_bytes_padded: usize) -> TestResult {
            check_iter_input_discard!(image_data, width_bytes, width_bytes_padded);

            let image_data_chunks = image_data.chunks(width_bytes_padded).map(|chunk| &chunk[..width_bytes as usize]);

            if ImageLineIter::new(&image_data, width_bytes, width_bytes_padded).eq(image_data_chunks) {
                TestResult::passed()
            } else {
                TestResult::failed()
            }
        }

        fn image_line_iter_back(image_data: Vec<u8>, width_bytes: Px, width_bytes_padded: usize) -> TestResult {
            check_iter_input_discard!(image_data, width_bytes, width_bytes_padded);

            let image_data_chunks = image_data.chunks(width_bytes_padded).map(|chunk| &chunk[..width_bytes as usize]).rev();

            if ImageLineIter::new(&image_data, width_bytes, width_bytes_padded).rev().eq(image_data_chunks) {
                TestResult::passed()
            } else {
                TestResult::failed()
            }
        }

        fn image_line_iter_mut(image_data: Vec<u8>, width_bytes: Px, width_bytes_padded: usize) -> TestResult {
            check_iter_input_discard!(image_data, width_bytes, width_bytes_padded);

            let mut image_data = image_data;
            let mut image_data_chunks = image_data.clone();
            let image_data_chunks = image_data_chunks.chunks_mut(width_bytes_padded).map(|chunk| &mut chunk[..width_bytes as usize]);

            if ImageLineIterMut::new(&mut image_data, width_bytes, width_bytes_padded).eq(image_data_chunks) {
                TestResult::passed()
            } else {
                TestResult::failed()
            }
        }

        fn image_line_iter_mut_back(image_data: Vec<u8>, width_bytes: Px, width_bytes_padded: usize) -> TestResult {
            check_iter_input_discard!(image_data, width_bytes, width_bytes_padded);

            let mut image_data = image_data;
            let mut image_data_chunks = image_data.clone();
            let image_data_chunks = image_data_chunks.chunks_mut(width_bytes_padded).map(|chunk| &mut chunk[..width_bytes as usize]).rev();

            if ImageLineIterMut::new(&mut image_data, width_bytes, width_bytes_padded).rev().eq(image_data_chunks) {
                TestResult::passed()
            } else {
                TestResult::failed()
            }
        }
    }
}
