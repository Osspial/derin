use {user32, comctl32, msg};
use window::*;
use window::refs::*;
use winapi::*;

use std::mem;
use std::cell::UnsafeCell;

pub struct IconWrapper<W: BaseWindow, S: Icon, L: Icon = S>{
    pub(super) window: W,
    pub(super) icon_sm: Option<S>,
    pub(super) icon_lg: Option<L>
}

pub struct OverlapWrapper<W: BaseWindow>( pub(super) W );

pub struct SubclassWrapper<W: BaseWindow, S: Subclass<W>> {
    window: W,
    data: Box<UnsafeCell<S>>
}

pub struct UnsafeSubclassWrapper<W: BaseWindow, S: Subclass<W>> {
    window: W,
    data: UnsafeCell<S>
}

// IconWrapper impls
impl_window_traits!{
    unsafe impl<W: OwnedWindow, S: Icon, L: Icon>
        BaseWindow,
        MutWindow,
        OwnedWindow,
        FontWindow,
        OverlappedWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        StaticTextWindow,
        StaticBitmapWindow,
        ProgressBarWindow,
        TrackbarWindow
    for IconWrapper<W, S, L>
}
unsafe impl<W: OwnedWindow, S: Icon, L: Icon> IconWindow for IconWrapper<W, S, L> {
    type IconSm = S;
    type IconLg = L;

    fn set_icon_sm(&mut self, icon: Option<Self::IconSm>) -> Option<Self::IconSm> {
        if let Some(ic) = icon.as_ref() {unsafe {
            user32::SendMessageW(self.hwnd(), WM_SETICON, ICON_SMALL as WPARAM, ic.hicon() as LPARAM);
        }}
        let ret = self.icon_sm.take();
        self.icon_sm = icon;
        ret
    }

    fn set_icon_lg(&mut self, icon: Option<Self::IconLg>) -> Option<Self::IconLg> {
        if let Some(ic) = icon.as_ref() {unsafe {
            user32::SendMessageW(self.hwnd(), WM_SETICON, ICON_BIG as WPARAM, ic.hicon() as LPARAM);
        }}
        let ret = self.icon_lg.take();
        self.icon_lg = icon;
        ret
    }
}
unsafe impl<W: OwnedWindow, S: Icon, L: Icon> WrapperWindow for IconWrapper<W, S, L> {
    type Inner = W;

    fn inner(&self) -> &W {
        &self.window
    }
    fn inner_mut(&mut self) -> &mut W {
        &mut self.window
    }
}


// OverlapWrapper impls
unsafe impl<W: OwnedWindow> OverlappedWindow for OverlapWrapper<W> {}
impl_window_traits!{
    unsafe impl<W: OwnedWindow>
        BaseWindow,
        MutWindow,
        OwnedWindow,
        FontWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        StaticTextWindow,
        StaticBitmapWindow,
        ProgressBarWindow,
        TrackbarWindow,
        IconWindow
    for OverlapWrapper<W>
}
unsafe impl<W: OwnedWindow> WrapperWindow for OverlapWrapper<W> {
    type Inner = W;

    fn inner(&self) -> &W {
        &self.0
    }
    fn inner_mut(&mut self) -> &mut W {
        &mut self.0
    }
}


// SubclassWrapper impls
impl<W: OwnedWindow, S: Subclass<W>> SubclassWrapper<W, S> {
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
    unsafe impl<W: OwnedWindow, S: Subclass<W>>
        BaseWindow,
        MutWindow,
        OwnedWindow,
        FontWindow,
        OverlappedWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        StaticTextWindow,
        StaticBitmapWindow,
        ProgressBarWindow,
        TrackbarWindow,
        IconWindow
    for SubclassWrapper<W, S>
}
unsafe impl<W: OwnedWindow, S: Subclass<W>> WrapperWindow for SubclassWrapper<W, S> {
    type Inner = W;

    fn inner(&self) -> &W {
        &self.window
    }
    fn inner_mut(&mut self) -> &mut W {
        &mut self.window
    }
}


// UnsafeSubclassWrapper impls
impl<W: OwnedWindow, S: Subclass<W>> UnsafeSubclassWrapper<W, S> {
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
    unsafe impl<W: OwnedWindow, S: Subclass<W>>
        BaseWindow,
        MutWindow,
        OwnedWindow,
        FontWindow,
        OverlappedWindow,
        OrphanableWindow,
        ParentWindow,
        ButtonWindow,
        StaticTextWindow,
        StaticBitmapWindow,
        ProgressBarWindow,
        TrackbarWindow,
        IconWindow
    for UnsafeSubclassWrapper<W, S>
}
unsafe impl<W: OwnedWindow, S: Subclass<W>> WrapperWindow for UnsafeSubclassWrapper<W, S> {
    type Inner = W;

    fn inner(&self) -> &W {
        &self.window
    }
    fn inner_mut(&mut self) -> &mut W {
        &mut self.window
    }
}
