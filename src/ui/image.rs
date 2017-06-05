use dct::geometry::{OriginRect, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFormat {
    // Mono,
    RGB888,
    RGBA8888,
    BGR888,
    BGRA8888
}

impl ColorFormat {
    pub fn bits_per_pixel(self) -> usize {
        use self::ColorFormat::*;
        match self {
            // Mono     => 1,
            RGB888   => 24,
            RGBA8888 => 32,
            BGR888   => 24,
            BGRA8888 => 32
        }
    }
}

pub struct Image<P: AsRef<[u8]>> {
    pixels: P,
    dims: OriginRect,
    color_format: ColorFormat
}

impl<P: AsRef<[u8]>> Image<P> {
    pub fn new(pixels: P, dims: OriginRect, color_format: ColorFormat) -> Image<P> {
        {
            let pixel_bytes = pixels.as_ref();
            let buffer_bits = pixel_bytes.len() * 8;
            let image_needed_bits = (dims.width() * dims.height()) as usize * color_format.bits_per_pixel();
            if buffer_bits != image_needed_bits {
                panic!("Mismatched buffer size; expected {}, found {}", image_needed_bits, buffer_bits);
            }
        }

        Image{ pixels, dims, color_format }
    }

    pub fn pixel_bytes(&self) -> &[u8] {
        self.pixels.as_ref()
    }

    pub fn pixel_bytes_mut(&mut self) -> &mut [u8]
            where P: AsMut<[u8]>
    {
        self.pixels.as_mut()
    }

    pub fn dims(&self) -> OriginRect {
        self.dims
    }

    pub fn color_format(&self) -> ColorFormat {
        self.color_format
    }

    pub fn into_raw(self) -> P {
        self.pixels
    }
}
