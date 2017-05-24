pub mod iter;

use user32;
use gdi32;
use winapi::*;

use ucs2::{WithString, UCS2_CONVERTER};
use dct::geometry::{Px, Rect, OriginRect};
use dct::color::Color24;

use self::iter::*;

use std::{ptr, mem, cmp, slice};
use std::path::Path;
use std::io::{Result, Error};

#[derive(Debug)]
pub struct DDBitmap( HBITMAP );
#[derive(Debug)]
pub struct DIBitmap( HBITMAP );
#[derive(Debug)]
pub struct DIBSection {
    handle: HBITMAP,
    bits: *mut [u8]
}
#[derive(Debug)]
pub struct IconOwned( HICON );

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitmapInfo {
    pub width: Px,
    pub height: Px,
    pub width_bytes: usize,
    pub bits_per_pixel: u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorFormat<'a> {
    /// A monochrome color format, containing a (black_color, white_color) pair.
    Monochrome(Color24, Color24),
    /// 4-bit color paletted format. Palette slice can contain up to 16 colors.
    Paletted4(&'a [Color24]),
    /// 8-bit color paletted format. Palette slice can contain up to 256 colors.
    Paletted8(&'a [Color24]),
    FullColor16,
    FullColor24,
    FullColor32
}


pub trait Bitmap {
    fn hbitmap(&self) -> HBITMAP;

    fn bitmap_info(&self) -> BitmapInfo {
        unsafe {
            let mut bitmap_struct: BITMAP = mem::zeroed();
            gdi32::GetObjectW(
                self.hbitmap() as HGDIOBJ,
                mem::size_of::<BITMAP>() as c_int,
                &mut bitmap_struct as *mut _ as *mut c_void
            );

            BitmapInfo {
                width: bitmap_struct.bmWidth,
                height: bitmap_struct.bmHeight,
                width_bytes: bitmap_struct.bmWidthBytes as usize,
                bits_per_pixel: bitmap_struct.bmBitsPixel as u8
            }
        }
    }

    fn bits(&self) -> &[u8] {
        unsafe {
            let mut bitmap_struct: BITMAP = mem::zeroed();
            gdi32::GetObjectW(
                self.hbitmap() as HGDIOBJ,
                mem::size_of::<BITMAP>() as c_int,
                &mut bitmap_struct as *mut _ as *mut c_void
            );

            slice::from_raw_parts(bitmap_struct.bmBits as *const u8, (bitmap_struct.bmHeight * bitmap_struct.bmWidthBytes) as usize)
        }
    }

    fn bitmap_data(&self) -> (BitmapInfo, &[u8]) {
        unsafe {
            let mut bitmap_struct: BITMAP = mem::zeroed();
            gdi32::GetObjectW(
                self.hbitmap() as HGDIOBJ,
                mem::size_of::<BITMAP>() as c_int,
                &mut bitmap_struct as *mut _ as *mut c_void
            );

            (
                BitmapInfo {
                    width: bitmap_struct.bmWidth,
                    height: bitmap_struct.bmHeight,
                    width_bytes: bitmap_struct.bmWidthBytes as usize,
                    bits_per_pixel: bitmap_struct.bmBitsPixel as u8
                },
                slice::from_raw_parts(bitmap_struct.bmBits as *const u8, (bitmap_struct.bmHeight * bitmap_struct.bmWidthBytes) as usize)
            )
        }
    }

    fn image_lines(&self) -> ImageLineIter {
        let (bmi, bits) = self.bitmap_data();
        ImageLineIter::new(bits, bmi.width * bmi.bits_per_pixel as Px / 8, bmi.width_bytes)
    }
}

pub trait Icon {
    fn hicon(&self) -> HICON;
}


impl DDBitmap {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<DDBitmap> {
        UCS2_CONVERTER.with_string(path.as_ref(), |path| {
            let bitmap = unsafe{ user32::LoadImageW(
                ptr::null_mut(), path.as_ptr(), IMAGE_BITMAP, 0, 0, LR_LOADFROMFILE
            )};

            if bitmap != ptr::null_mut() {
                Ok(DDBitmap(bitmap as HBITMAP))
            } else {
                Err(Error::last_os_error())
            }
        })
    }
}

impl DIBitmap {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<DIBitmap> {
        UCS2_CONVERTER.with_string(path.as_ref(), |path| {
            let bitmap = unsafe{ user32::LoadImageW(
                ptr::null_mut(), path.as_ptr(), IMAGE_BITMAP, 0, 0,
                LR_LOADFROMFILE | LR_CREATEDIBSECTION
            )};

            if bitmap != ptr::null_mut() {
                Ok(DIBitmap(bitmap as HBITMAP))
            } else {
                Err(Error::last_os_error())
            }
        })
    }
}

impl DIBSection {
    pub fn new(width: Px, height: Px, format: ColorFormat, x_ppm: Px, y_ppm: Px) -> DIBSection {
        unsafe {
            let (width, height, x_ppm, y_ppm) =
                (cmp::max(0, width), cmp::max(0, height), cmp::max(0, x_ppm), cmp::max(0, y_ppm));

            let bmi_header = BITMAPINFOHEADER  {
                biSize: ::std::mem::size_of::<BITMAPINFO>() as u32,
                biWidth: width,
                biHeight: height,
                biPlanes: 1,
                biBitCount: format.bits_per_pixel() as u16,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: x_ppm,
                biYPelsPerMeter: y_ppm,
                biClrUsed: 0,
                biClrImportant: 0
            };
            let mut pbits = ptr::null_mut();

            let hbitmap = match format {
                ColorFormat::Monochrome(b, w) => {
                    #[repr(C)]
                    struct BitmapInfoMonochrome {
                        header: BITMAPINFOHEADER,
                        colors: [RGBQUAD; 2]
                    }

                    let bmp_info = BitmapInfoMonochrome {
                        header: BITMAPINFOHEADER {
                            biClrUsed: 2,
                            ..bmi_header
                        },
                        colors: [
                            RGBQUAD {
                                rgbRed: b.red,
                                rgbGreen: b.green,
                                rgbBlue: b.blue,
                                rgbReserved: 0
                            },
                            RGBQUAD {
                                rgbRed: w.red,
                                rgbGreen: w.green,
                                rgbBlue: w.blue,
                                rgbReserved: 0
                            }
                        ]
                    };

                    gdi32::CreateDIBSection(
                        ptr::null_mut(),
                        &bmp_info as *const _ as *const BITMAPINFO,
                        DIB_RGB_COLORS,
                        &mut pbits,
                        ptr::null_mut(),
                        0
                    )
                }
                ColorFormat::Paletted4(palette) => {
                    #[repr(C)]
                    struct BitmapInfoPaletted4 {
                        header: BITMAPINFOHEADER,
                        colors: [RGBQUAD; 16]
                    }

                    let mut bmp_info = BitmapInfoPaletted4 {
                        header: BITMAPINFOHEADER {
                            biClrUsed: palette.len() as DWORD,
                            ..bmi_header
                        },
                        colors: [
                            RGBQUAD {
                                rgbRed: 0,
                                rgbGreen: 0,
                                rgbBlue: 0,
                                rgbReserved: 0
                            }; 16
                        ]
                    };

                    for (index, color) in palette.iter().enumerate() {
                        bmp_info.colors[index] = RGBQUAD {
                            rgbRed: color.red,
                            rgbGreen: color.green,
                            rgbBlue: color.blue,
                            rgbReserved: 0
                        };
                    }

                    gdi32::CreateDIBSection(
                        ptr::null_mut(),
                        &bmp_info as *const _ as *const BITMAPINFO,
                        DIB_RGB_COLORS,
                        &mut pbits,
                        ptr::null_mut(),
                        0
                    )
                },
                ColorFormat::Paletted8(palette) => {
                    #[repr(C)]
                    struct BitmapInfoPaletted8 {
                        header: BITMAPINFOHEADER,
                        colors: [RGBQUAD; 256]
                    }

                    let mut bmp_info = BitmapInfoPaletted8 {
                        header: BITMAPINFOHEADER {
                            biClrUsed: palette.len() as DWORD,
                            ..bmi_header
                        },
                        colors: [
                            RGBQUAD {
                                rgbRed: 0,
                                rgbGreen: 0,
                                rgbBlue: 0,
                                rgbReserved: 0
                            }; 256
                        ]
                    };

                    for (index, color) in palette.iter().enumerate() {
                        bmp_info.colors[index] = RGBQUAD {
                            rgbRed: color.red,
                            rgbGreen: color.green,
                            rgbBlue: color.blue,
                            rgbReserved: 0
                        };
                    }

                    gdi32::CreateDIBSection(
                        ptr::null_mut(),
                        &bmp_info as *const _ as *const BITMAPINFO,
                        DIB_RGB_COLORS,
                        &mut pbits,
                        ptr::null_mut(),
                        0
                    )
                }
                _ => {
                    let bmp_info = BITMAPINFO {
                        bmiHeader: bmi_header,
                        bmiColors: []
                    };

                    gdi32::CreateDIBSection(
                        ptr::null_mut(),
                        &bmp_info,
                        DIB_RGB_COLORS,
                        &mut pbits,
                        ptr::null_mut(),
                        0
                    )
                }
            };

            // This should only fail if there's an invalid parameter, which shouldn't happen if the
            // infrastructure code has been written properly.
            debug_assert_ne!(hbitmap, ptr::null_mut());

            let mut bitmap_struct: BITMAP = mem::zeroed();
            gdi32::GetObjectW(
                hbitmap as HGDIOBJ,
                mem::size_of::<BITMAP>() as c_int,
                &mut bitmap_struct as *mut _ as *mut c_void
            );

            let buffer_length = (bitmap_struct.bmWidthBytes * height) as usize;

            DIBSection {
                handle: hbitmap,
                bits: slice::from_raw_parts_mut(pbits as *mut u8, buffer_length)
            }
        }
    }

    #[inline]
    pub fn bits_mut(&mut self) -> &mut [u8] {
        unsafe{ &mut *self.bits }
    }

    #[inline]
    pub fn image_lines_mut(&mut self) -> ImageLineIterMut {
        let bmi = self.bitmap_info();
        ImageLineIterMut::new(self.bits_mut(), bmi.width * bmi.bits_per_pixel as Px / 8, bmi.width_bytes)
    }
}

impl IconOwned {
    pub fn open<P: AsRef<Path>>(path: P, size: OriginRect) -> Result<IconOwned> {
        UCS2_CONVERTER.with_string(path.as_ref(), |path| {
            let icon = unsafe{ user32::LoadImageW(
                ptr::null_mut(), path.as_ptr(), IMAGE_ICON, size.width() as c_int,
                size.height() as c_int, LR_LOADFROMFILE
            )};

            if icon != ptr::null_mut() {
                Ok(IconOwned(icon as HICON))
            } else {
                Err(Error::last_os_error())
            }
        })
    }

    pub fn from_masks(width: Px, height: Px, and_mask: &[u8], xor_mask: &[u8]) -> Result<IconOwned> {
        assert_eq!(width * height / 8, and_mask.len() as Px);
        assert_eq!(width * height / 8, xor_mask.len() as Px);

        let icon = unsafe{ user32::CreateIcon(
            ptr::null_mut(),
            width,
            height,
            1, 1,
            and_mask.as_ptr(),
            xor_mask.as_ptr()
        ) };

        if icon != ptr::null_mut() {
            Ok(IconOwned(icon))
        } else {
            Err(Error::last_os_error())
        }
    }

    pub fn from_mask_bmp<M>(mask: &M) -> Result<IconOwned>
            where M: Bitmap
    {
        let mut icon_info = ICONINFO {
            fIcon: TRUE,
            xHotspot: 0, yHotspot: 0,
            hbmMask: mask.hbitmap(),
            hbmColor: ptr::null_mut()
        };
        let icon = unsafe{ user32::CreateIconIndirect(&mut icon_info) };
        if icon != ptr::null_mut() {
            Ok(IconOwned(icon))
        } else {
            Err(Error::last_os_error())
        }
    }

    pub fn new_color<M, C>(mask: &M, color: &C) -> Result<IconOwned>
            where M: Bitmap, C: Bitmap
    {
        let mut icon_info = ICONINFO {
            fIcon: TRUE,
            xHotspot: 0, yHotspot: 0,
            hbmMask: mask.hbitmap(),
            hbmColor: color.hbitmap()
        };
        let icon = unsafe{ user32::CreateIconIndirect(&mut icon_info) };
        if icon != ptr::null_mut() {
            Ok(IconOwned(icon))
        } else {
            Err(Error::last_os_error())
        }
    }
}

impl Clone for IconOwned {
    fn clone(&self) -> IconOwned {
        IconOwned(unsafe{ user32::CopyIcon(self.0) })
    }
}

impl<'a> ColorFormat<'a> {
    pub fn bits_per_pixel(self) -> u8 {
        match self {
            ColorFormat::Monochrome(_, _) => 1,
            ColorFormat::Paletted4(_)     => 4,
            ColorFormat::Paletted8(_)     => 8,
            ColorFormat::FullColor16      => 16,
            ColorFormat::FullColor24      => 24,
            ColorFormat::FullColor32      => 32
        }
    }
}


impl Bitmap for DDBitmap {
    #[inline]
    fn hbitmap(&self) -> HBITMAP {
        self.0
    }
}
impl Bitmap for DIBitmap {
    #[inline]
    fn hbitmap(&self) -> HBITMAP {
        self.0
    }
}
impl Bitmap for DIBSection {
    #[inline]
    fn hbitmap(&self) -> HBITMAP {
        self.handle
    }
}

impl Icon for IconOwned {
    fn hicon(&self) -> HICON {
        self.0
    }
}

impl Drop for DDBitmap {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.0 as HGDIOBJ) };
    }
}
impl Drop for DIBitmap {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.0 as HGDIOBJ) };
    }
}
impl Drop for DIBSection {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.handle as HGDIOBJ) };
    }
}
impl Drop for IconOwned {
    fn drop(&mut self) {
        unsafe{ user32::DestroyIcon(self.0) };
    }
}
