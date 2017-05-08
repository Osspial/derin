use winapi::*;
use window::*;
use {user32, comctl32};
use msg::{self, Msg};
use msg::user::UserMsg;

use dct::geometry::*;

use std::marker::PhantomData;
use std::mem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentRef( HWND );
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRef( HWND );
#[derive(Debug, PartialEq, Eq)]
pub struct WindowRefMut<'a>( HWND, PhantomData<&'a mut ()> );
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsafeSubclassRef<'a, U: UserMsg>( HWND, PhantomData<(U, PhantomData<&'a mut ()>)> );

pub struct ProcWindowRef<'a, W: WindowBase, S: 'a + Subclass<W> + ?Sized> {
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_data: &'a mut S,
    __marker: PhantomData<(W, S)>
}

// ParentRef impls
impl ParentRef {
    pub unsafe fn from_raw(hwnd: HWND) -> ParentRef {
        ParentRef(hwnd)
    }
}
unsafe impl WindowBase for ParentRef {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {
        self.0
    }
}
unsafe impl ParentWindow for ParentRef {}


// WindowRef impls
impl WindowRef {
    pub unsafe fn from_raw(hwnd: HWND) -> WindowRef {
        WindowRef(hwnd)
    }
}
unsafe impl WindowBase for WindowRef {
    unsafe fn hwnd(&self) -> HWND {
        self.0
    }
}

// WindowRefMut impls
impl<'a> WindowRefMut<'a> {
    pub unsafe fn from_raw(hwnd: HWND) -> WindowRefMut<'a> {
        WindowRefMut(hwnd, PhantomData)
    }
}
unsafe impl<'a> WindowBase for WindowRefMut<'a> {
    unsafe fn hwnd(&self) -> HWND {
        self.0
    }
}


// UnsafeSubclassRef impls
impl<'a, U: UserMsg> UnsafeSubclassRef<'a, U> {
    pub unsafe fn from_raw(hwnd: HWND) -> UnsafeSubclassRef<'a, U> {
        UnsafeSubclassRef(hwnd, PhantomData)
    }

    pub unsafe fn send_user_msg(&mut self, msg: U) -> i64 {
        let discriminant = msg.discriminant();
        let encoded_bytes = msg::user::encode(msg);

        let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
        user32::SendMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam)
    }

    pub fn post_user_msg(&self, msg: U)
            where U: 'static
    {
        unsafe {
            let discriminant = msg.discriminant();
            let encoded_bytes = msg::user::encode(msg);

            let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
            user32::PostMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam);
        }
    }
}
unsafe impl<'a, U: UserMsg> WindowBase for UnsafeSubclassRef<'a, U> {
    unsafe fn hwnd(&self) -> HWND {
        self.0
    }
}
unsafe impl<'a, U: UserMsg> WindowMut for UnsafeSubclassRef<'a, U> {}

// ProcWindowRef impls
impl<'a, W: WindowBase, S: Subclass<W>> ProcWindowRef<'a, W, S> {
    pub(super) unsafe fn new(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM, subclass_data: &'a mut S) -> ProcWindowRef<'a, W, S> {
        ProcWindowRef {
            hwnd: hwnd,
            msg: msg,
            wparam: wparam,
            lparam: lparam,
            subclass_data: subclass_data,
            __marker: PhantomData
        }
    }

    pub fn subclass_data(&mut self) -> &mut S {
        self.subclass_data
    }

    pub fn send_user_msg(&mut self, msg: S::UserMsg) -> i64 {
        let discriminant = msg.discriminant();
        let encoded_bytes = msg::user::encode(msg);

        unsafe {
            let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
            user32::SendMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam)
        }
    }

    /// Forward the message to the specified window. Panics if the message is a user message.
    pub fn forward_msg<F, WM>(&mut self, msg: &mut Msg<S::UserMsg>, window: F) -> i64
            where F: FnOnce(&mut Self) -> &mut WM,
                  WM: WindowMut
    {
        if let Msg::User(_) = *msg {
            panic!("Attempted to forward user message; use `forward_user_msg` instead");
        } else {
            let ret = unsafe{ user32::SendMessageW(window(self).hwnd(), self.msg, self.wparam, self.lparam) };
            self.update_msg_enum(msg);
            ret
        }
    }

    pub fn forward_user_msg<F>(&mut self, msg: &mut Msg<S::UserMsg>, window: F) -> i64
            where F: FnOnce(&mut Self) -> UnsafeSubclassRef<S::UserMsg>
    {
        let ret = unsafe{ user32::SendMessageW(window(self).hwnd(), self.msg, self.wparam, self.lparam) };
        self.update_msg_enum(msg);
        ret
    }

    pub fn default_window_proc(&mut self, msg: &mut Msg<S::UserMsg>) -> i64 {
        let ret = unsafe{ comctl32::DefSubclassProc(self.hwnd, self.msg, self.wparam, self.lparam) as i64 };
        self.update_msg_enum(msg);
        ret
    }

    fn update_msg_enum(&self, msg: &mut Msg<S::UserMsg>) {
        match *msg {
            Msg::GetSizeBounds(ref mut size_bounds) => {
                assert_eq!(self.msg, WM_GETMINMAXINFO);
                let mmi = unsafe{ *(self.lparam as *mut MINMAXINFO) };
                size_bounds.min = OriginRect::new(mmi.ptMinTrackSize.x as Px, mmi.ptMinTrackSize.y as Px);
                size_bounds.max = OriginRect::new(mmi.ptMaxTrackSize.x as Px, mmi.ptMaxTrackSize.y as Px);
            },
            _ => ()
        }
    }
}
unsafe impl<'a, W: WindowBase, S: Subclass<W>> WindowBase for ProcWindowRef<'a, W, S> {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl_window_traits!{
    unsafe impl<lifetime 'a, W: WindowBase, S: Subclass<W>>
        ;WindowMut,
        OverlappedWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        TextLabelWindow,
        ProgressBarWindow,
        TrackbarWindow
    for ProcWindowRef<'a, W, S>
}
