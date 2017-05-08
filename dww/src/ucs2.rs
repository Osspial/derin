
use std::thread::LocalKey;
use std::ffi::OsStr;
use std::os::windows::ffi::{OsStrExt, EncodeWide};
use std::cell::RefCell;
use std::slice;
use std::iter::{once, Chain, Once};

use winapi::winnt::WCHAR;

pub type Ucs2Str = [WCHAR];
pub type Ucs2String = Vec<WCHAR>;

thread_local!{
    #[doc(hidden)]
    pub static UCS2_CONVERTER: RefCell<Ucs2Converter> = RefCell::new(Ucs2Converter::default());
}

impl WithString for LocalKey<RefCell<Ucs2Converter>> {
    fn with_string<S, F, R>(&'static self, s: S, f: F) -> R
            where S: AsRef<OsStr>,
                  F: FnOnce(&Ucs2Str) -> R
    {
        self.with(|converter| {
            let mut converter = converter.borrow_mut();
            converter.str_buf.extend(ucs2_str(s.as_ref()));
            let ret = f(&converter.str_buf[..]);
            converter.str_buf.clear();
            ret
        })
    }

    fn with_string_noprefix<S, F, R>(&'static self, s: S, f: F) -> R
            where S: AsRef<OsStr>,
                  F: FnOnce(&Ucs2Str) -> R
    {
        self.with(|converter| {
            let mut converter = converter.borrow_mut();

            // A lot of controls will interperet the '&' character as a signal to underline the
            // next character. That's ignored if the '&' character is doubled up, so this does
            // that.
            for c in ucs2_str(s.as_ref()) {
                converter.str_buf.push(c);
                if c == '&' as u16 {
                    converter.str_buf.push(c);
                }
            }

            let ret = f(&converter.str_buf[..]);
            converter.str_buf.clear();
            ret
        })
    }

    fn with_ucs2_buffer<F, R>(&'static self, len: usize, f: F) -> R
            where F: FnOnce(&mut Ucs2Str) -> R
    {
        self.with(|converter| {
            let mut converter = converter.borrow_mut();
            converter.str_buf.resize(len, 0);
            let ret = f(&mut converter.str_buf[..]);
            converter.str_buf.clear();
            ret
        })
    }
}

pub(crate) trait WithString {
    fn with_string<S, F, R>(&'static self, S, F) -> R
            where S: AsRef<OsStr>,
                  F: FnOnce(&Ucs2Str) -> R;
    fn with_string_noprefix<S, F, R>(&'static self, S, F) -> R
            where S: AsRef<OsStr>,
                  F: FnOnce(&Ucs2Str) -> R;
    fn with_ucs2_buffer<F, R>(&'static self, len: usize, F) -> R
            where F: FnOnce(&mut Ucs2Str) -> R;
}

#[derive(Default)]
pub struct Ucs2Converter {
    str_buf: Ucs2String
}

pub fn ucs2_str<S: ?Sized + AsRef<OsStr>>(s: &S) -> Ucs2Iter {
    Ucs2Iter(s.as_ref().encode_wide().chain(once(0)))
}

pub unsafe fn ucs2_str_from_ptr<'a>(p: *const WCHAR) -> &'a Ucs2Str {
    let mut end = p;
    while *end != 0 {
        end = end.offset(1);
    }
    slice::from_raw_parts(p, end as usize - p as usize)
}


pub struct Ucs2Iter<'a>(Chain<EncodeWide<'a>, Once<u16>>);

impl<'a> Iterator for Ucs2Iter<'a> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        self.0.next()
    }
}
