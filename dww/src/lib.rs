#![feature(never_type)]

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
#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

pub mod msg;
pub mod gdi;
pub mod ucs2;
pub mod window;
mod vkey;

use winapi::*;

use std::mem;

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
    use msg::user::{self, UserMsg};
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

        assert_eq!(msg, unsafe{ user::decode(discriminant, user::encode(msg)) });
    }
}
