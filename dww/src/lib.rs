extern crate winapi;
#[macro_use]
extern crate kernel32 as _kernel32;
#[macro_use]
extern crate user32 as _user32;
#[macro_use]
extern crate comctl32 as _comctl32;
extern crate gdi32;
extern crate uxtheme;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate dct;
#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate dww_macros;

pub mod msg;
pub mod hdc;
pub mod ucs2;
pub mod window;
mod vkey;

use dct::geometry::{Rect, OriginRect};

use winapi::*;

use std::{ptr, mem};
use std::marker::{Send, Sync};
use std::path::Path;
use std::io::{Result, Error};
use std::borrow::Borrow;

use ucs2::{WithString, UCS2_CONVERTER};

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
}

impl Drop for Icon {
    fn drop(&mut self) {
        unsafe{ user32::DestroyIcon(self.0) };
    }
}

pub struct Font( HFONT );

impl Font {
    pub fn def_sys_font() -> Font {
        Font(ptr::null_mut())
    }

    pub fn sys_caption_font() -> Font {
        let non_client_metrics = non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfCaptionFont) })
    }

    pub fn sys_small_caption_font() -> Font {
        let non_client_metrics = non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfSmCaptionFont) })
    }

    pub fn sys_menu_font() -> Font {
        let non_client_metrics = non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfMenuFont) })
    }

    pub fn sys_status_font() -> Font {
        let non_client_metrics = non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfStatusFont) })
    }

    pub fn sys_message_font() -> Font {
        let non_client_metrics = non_client_metrics();
        Font(unsafe{ gdi32::CreateFontIndirectW(&non_client_metrics.lfMessageFont) })
    }
}

unsafe impl Send for Font {}
unsafe impl Sync for Font {}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe{ gdi32::DeleteObject(self.0 as HGDIOBJ) };
    }
}

pub struct DefaultFont;
impl Borrow<Font> for DefaultFont {
    fn borrow(&self) -> &Font {
        static DEFAULT_FONT: usize = 0;
        unsafe{ mem::transmute(&DEFAULT_FONT) }
    }
}



pub fn init() {
    // It's true that this static mut could cause a memory race. However, the only consequence of
    // that memory race is that this function runs more than once, which won't have any bad impacts
    // other than perhaps a slight increase in memory usage.
    static mut INITIALIZED: bool = false;

    unsafe {
        if !INITIALIZED {
            // Load the common controls dll
            {
                let init_ctrls = INITCOMMONCONTROLSEX {
                    dwSize: mem::size_of::<INITCOMMONCONTROLSEX>() as DWORD,
                    dwICC: ICC_PROGRESS_CLASS
                };
                comctl32::InitCommonControlsEx(&init_ctrls);
            }

            INITIALIZED = true;
        }
    }
}

fn non_client_metrics() -> NONCLIENTMETRICSW {
    unsafe {
        let mut non_client_metrics = NONCLIENTMETRICSW {
            cbSize: mem::size_of::<NONCLIENTMETRICSW>() as UINT,
            ..mem::zeroed::<NONCLIENTMETRICSW>()
        };
        user32::SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            mem::size_of::<NONCLIENTMETRICSW>() as UINT,
            &mut non_client_metrics as *mut NONCLIENTMETRICSW as *mut c_void,
            0
        );
        non_client_metrics
    }
}

mod kernel32 {isolation_aware_kernel32!{{
    use _kernel32;

    const SHELL32_DLL: &'static [WCHAR] = &[0x0073, 0x0068, 0x0065, 0x006C, 0x006C, 0x0033, 0x0032, 0x002E, 0x0064, 0x006C, 0x006C, 0x0000];
    const ACTCTX_FLAG_ASSEMBLY_DIRECTORY_VALID: DWORD = 0x004;
    const ACTCTX_FLAG_RESOURCE_NAME_VALID: DWORD = 0x008;
    const ACTCTX_FLAG_SET_PROCESS_DEFAULT: DWORD = 0x010;

    let mut dir = [0u16; MAX_PATH];
    _kernel32::GetSystemDirectoryW(dir.as_mut_ptr(), MAX_PATH as u32);
    let styles_ctx = ACTCTXW {
        cbSize: mem::size_of::<ACTCTXW>() as u32,
        dwFlags:
            ACTCTX_FLAG_ASSEMBLY_DIRECTORY_VALID |
            ACTCTX_FLAG_RESOURCE_NAME_VALID |
            ACTCTX_FLAG_SET_PROCESS_DEFAULT,
        lpSource: SHELL32_DLL.as_ptr(),
        wProcessorArchitecture: 0,
        wLangId: 0,
        lpAssemblyDirectory: dir.as_ptr(),
        lpResourceName: 124 as LPCWSTR,
        lpApplicationName: ptr::null_mut(),
        hModule: ptr::null_mut()
    };

    self::IsolationAwareCreateActCtxW(&styles_ctx)
}}}
mod user32 {isolation_aware_user32!{kernel32}}
mod comctl32 {isolation_aware_comctl32!{kernel32}}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    #[derive(UserMsg, PartialEq, Debug, Eq, Clone, Copy)]
    enum TestMsg<'a> {
        Foo(u32),
        Bar(&'a u32),
        Baz,
        Slice(&'a [u64])
    }

    #[derive(UserMsg, Clone, Copy)]
    enum SingleVarMsg {
        Bar(u32)
    }

    #[test]
    fn encode_foo() {
        test_encoding(TestMsg::Foo(36));
        test_encoding(TestMsg::Bar(&48));
        test_encoding(TestMsg::Baz);
        test_encoding(TestMsg::Slice(&[1, 2, 3, 4]));
    }

    #[derive(UserMsg, PartialEq, Debug, Eq, Clone, Copy)]
    enum BadMsg<'a> {
        Foo(u64, u64, u64),
        Bar(&'a u32, &'a [u32])
    }

    #[test]
    #[should_panic]
    fn bad_msg_foo() {
        test_encoding(BadMsg::Foo(32, 64, 128));
    }

    #[test]
    #[should_panic]
    fn bad_msg_bar() {
        test_encoding(BadMsg::Bar(&1024, &[2048, 10]));
    }

    fn test_encoding<U: UserMsg + Debug + Eq + Copy>(msg: U) {
        let discriminant = msg.discriminant();

        assert_eq!(msg, unsafe{ user_msg::decode(discriminant, user_msg::encode(msg)) });
    }
}
