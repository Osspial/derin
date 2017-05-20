use {user32, comctl32, msg};
use window::*;
use window::refs::*;
use winapi::*;

use gdi::text::Font;

use std::mem;
use std::borrow::Borrow;
use std::cell::UnsafeCell;

pub struct IconWrapper<W: WindowBase, I: Borrow<WindowIcon> = WindowIcon> {
    pub(super) window: W,
    pub(super) icon: I
}

pub struct OverlapWrapper<W: WindowBase>( pub(super) W );

pub struct SubclassWrapper<W: WindowBase, S: Subclass<W>> {
    window: W,
    data: Box<UnsafeCell<S>>
}

pub struct UnsafeSubclassWrapper<W: WindowBase, S: Subclass<W>> {
    window: W,
    data: UnsafeCell<S>
}

// IconWrapper impls
impl_window_traits!{
    unsafe impl<W: WindowOwned, I: Borrow<WindowIcon>>
        WindowBase(this) => this.window;
        WindowMut,
        WindowOwned,
        WindowFont,
        OverlappedWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        TextLabelWindow,
        ProgressBarWindow,
        TrackbarWindow
    for IconWrapper<W, I>
}
unsafe impl<W: WindowOwned, I: Borrow<WindowIcon>> IconWindow for IconWrapper<W, I> {
    type I = I;
    #[inline]
    fn set_icon(&mut self, icon: I) {
        unsafe{ self.set_window_icon(icon.borrow()) };
        self.icon = icon;
    }
}


// OverlapWrapper impls
unsafe impl<W: WindowOwned> OverlappedWindow for OverlapWrapper<W> {}
impl_window_traits!{
    unsafe impl<W: WindowOwned>
        WindowBase(this) => this.0;
        WindowMut,
        WindowOwned,
        WindowFont,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        TextLabelWindow,
        ProgressBarWindow,
        TrackbarWindow,
        IconWindow
    for OverlapWrapper<W>
}


// SubclassWrapper impls
impl<W: WindowOwned, S: Subclass<W>> SubclassWrapper<W, S> {
    pub fn new(window: W, data: S) -> SubclassWrapper<W, S> {
        let wrapper = SubclassWrapper {
            window: window,
            data: Box::new(UnsafeCell::new(data))
        };

        unsafe{ comctl32::SetWindowSubclass(
            wrapper.window.hwnd(),
            Some(subclass_proc::<W, S>),
            SUBCLASS_ID,
            wrapper.data.get() as DWORD_PTR
        ) };
        wrapper
    }

    /// Send a user message, yielding the value returned by `S::subclass_proc`.
    pub fn send_user_msg(&mut self, msg: S::UserMsg) -> i64 {
        let discriminant = msg.discriminant();
        let encoded_bytes = msg::user::encode(msg);

        unsafe {
            let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
            user32::SendMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam)
        }
    }

    /// Post a user message to the message queue associatd with the window.
    pub fn post_user_msg(&self, msg: S::UserMsg)
            where S::UserMsg: 'static
    {
        let discriminant = msg.discriminant();
        let encoded_bytes = msg::user::encode(msg);

        unsafe {
            let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
            user32::PostMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam);
        }
    }

    pub fn data(&self) -> &S {
        unsafe{ &*self.data.get() }
    }

    pub fn data_mut(&mut self) -> &mut S {
        unsafe{ &mut *self.data.get() }
    }
}
impl_window_traits!{
    unsafe impl<W: WindowOwned, S: Subclass<W>>
        WindowBase(this) => this.window;
        WindowMut,
        WindowOwned,
        WindowFont,
        OverlappedWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        TextLabelWindow,
        ProgressBarWindow,
        TrackbarWindow,
        IconWindow
    for SubclassWrapper<W, S>
}


// UnsafeSubclassWrapper impls
impl<W: WindowOwned, S: Subclass<W>> UnsafeSubclassWrapper<W, S> {
    pub unsafe fn new(window: W, data: S) -> UnsafeSubclassWrapper<W, S> {
        UnsafeSubclassWrapper {
            window: window,
            data: UnsafeCell::new(data)
        }
    }

    /// Send a user message, yielding the value returned by `S::subclass_proc`.
    ///
    /// Unsafe because it cannot guarantee that the subclass pointer is pointing to the correct
    /// location.
    pub unsafe fn send_user_msg(&mut self, msg: S::UserMsg) -> i64 {
        self.unsafe_subclass_ref().send_user_msg(msg)
    }

    /// Post a user message to the message queue associatd with the window.
    pub fn post_user_msg(&self, msg: S::UserMsg)
            where S::UserMsg: 'static
    {
        unsafe{ UnsafeSubclassRef::from_raw(self.hwnd()).post_user_msg(msg) }
    }

    pub fn unsafe_subclass_ref(&mut self) -> UnsafeSubclassRef<S::UserMsg> {
        unsafe{ UnsafeSubclassRef::from_raw(self.hwnd()) }
    }

    pub fn update_subclass_ptr(&self) {
        unsafe {
            comctl32::SetWindowSubclass(
                self.window.hwnd(),
                Some(subclass_proc::<W, S>),
                SUBCLASS_ID,
                self.data.get() as DWORD_PTR
            );
        }
    }

    pub fn data(&self) -> &S {
        unsafe{ &*self.data.get() }
    }

    pub fn data_mut(&mut self) -> &mut S {
        unsafe{ &mut *self.data.get() }
    }

    pub fn unwrap_data(self) -> S {
        unsafe{ self.data.into_inner() }
    }
}
impl_window_traits!{
    unsafe impl<W: WindowOwned, S: Subclass<W>>
        WindowBase(this) => this.window;
        WindowMut,
        WindowOwned,
        WindowFont,
        OverlappedWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        TextLabelWindow,
        ProgressBarWindow,
        TrackbarWindow,
        IconWindow
    for UnsafeSubclassWrapper<W, S>
}
