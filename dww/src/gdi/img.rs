use user32;
use winapi::*;

use ucs2::{WithString, UCS2_CONVERTER};
use dct::geometry::{Rect, OriginRect};

use std::ptr;
use std::path::Path;
use std::io::{Result, Error};

pub struct Icon( HICON );

impl Icon {
    pub fn open<P: AsRef<Path>>(path: P, size: OriginRect) -> Result<Icon> {
        UCS2_CONVERTER.with_string(path.as_ref(), |path| {
            let icon = unsafe{ user32::LoadImageW(
                ptr::null_mut(), path.as_ptr(), IMAGE_ICON, size.width() as c_int,
                size.height() as c_int, LR_LOADFROMFILE
            )};

            if icon != ptr::null_mut() {
                Ok(Icon(icon as HICON))
            } else {
                Err(Error::last_os_error())
            }
        })
    }

    #[inline]
    pub fn hicon(&self) -> HICON {
        self.0
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        unsafe{ user32::DestroyIcon(self.0) };
    }
}
