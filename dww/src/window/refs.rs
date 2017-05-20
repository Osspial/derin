use winapi::*;
use window::*;
use {user32, comctl32};
use msg::{self, Msg};
use msg::user::UserMsg;
use std::ops::{Deref, DerefMut};

use dct::geometry::*;

use std::marker::PhantomData;
use std::mem;

// Honestly, ParentRef and WindowRef *should* have lifetimes. However, them outliving the window
// shouldn't pose any safety concerns, and giving them lifetimes opens up a whole can of worms in
// derin that I don't really want to deal with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentRef( HWND );
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRef( HWND );
#[derive(Debug, PartialEq, Eq)]
pub struct WindowRefMut<'a>( HWND, PhantomData<&'a mut ()> );
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsafeSubclassRef<'a, U: UserMsg>( HWND, PhantomData<(U, PhantomData<&'a mut ()>)> );

pub struct ProcWindowRef<'a, W: WindowBase, S: 'a + Subclass<W> + ?Sized>( ProcWindowRefNoMsg<'a, W, S> );
pub struct ProcWindowRefNoMsg<'a, W: WindowBase, S: 'a + Subclass<W> + ?Sized> {
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
    fn hwnd(&self) -> HWND {
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
    fn hwnd(&self) -> HWND {
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
    fn hwnd(&self) -> HWND {
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
    fn hwnd(&self) -> HWND {
        self.0
    }
}
unsafe impl<'a, U: UserMsg> WindowMut for UnsafeSubclassRef<'a, U> {}

// ProcWindowRef impls
impl<'a, W: WindowBase, S: Subclass<W>> ProcWindowRef<'a, W, S> {
    pub(super) unsafe fn new(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM, subclass_data: &'a mut S) -> ProcWindowRef<'a, W, S> {
        ProcWindowRef(ProcWindowRefNoMsg {
            hwnd: hwnd,
            msg: msg,
            wparam: wparam,
            lparam: lparam,
            subclass_data: subclass_data,
            __marker: PhantomData
        })
    }

    pub fn split_subclass_data(self) -> (&'a mut S, ProcWindowRef<'static, W, ()>) {
        static mut EMPTY: () = ();
        (
            self.0.subclass_data,
            ProcWindowRef(ProcWindowRefNoMsg {
                hwnd: self.0.hwnd,
                msg: if WM_APP <= self.0.msg && self.0.msg <= 0xBFFF {
                    WM_NULL
                } else {self.0.msg},
                wparam: self.0.wparam,
                lparam: self.0.lparam,
                subclass_data: unsafe{ &mut EMPTY },
                __marker: PhantomData
            })
        )
    }

    pub fn msg(&mut self) -> Option<(&mut ProcWindowRefNoMsg<'a, W, S>, Msg<S::UserMsg>)> {
        let msg_option = unsafe {
            if WM_APP <= self.msg && self.msg <= 0xBFFF {
                let discriminant = (self.msg - WM_APP) as u16;
                let bytes: [u8; 16] = mem::transmute((self.wparam, self.lparam));
                Some(Msg::User(msg::user::decode(discriminant, bytes)))
            } else {
                match self.msg {
                    WM_CLOSE => Some(Msg::Close),
                    WM_SIZE  => Some(Msg::Size(OriginRect::new(loword(self.lparam) as Px, hiword(self.lparam) as Px))),
                    WM_LBUTTONDOWN  |
                    WM_MBUTTONDOWN  |
                    WM_RBUTTONDOWN  |
                    WM_XBUTTONDOWN => {
                        let button = match self.msg {
                            WM_LBUTTONDOWN => MouseButton::Left,
                            WM_MBUTTONDOWN => MouseButton::Middle,
                            WM_RBUTTONDOWN => MouseButton::Right,
                            WM_XBUTTONDOWN => MouseButton::Other(hiword(self.wparam as LPARAM) as u8),
                            _ => unreachable!()
                        };

                        Some(Msg::MouseDown(button, Point::new(loword(self.lparam) as Px, hiword(self.lparam) as Px)))
                    }
                    WM_LBUTTONDBLCLK  |
                    WM_MBUTTONDBLCLK  |
                    WM_RBUTTONDBLCLK => {
                        let button = match self.msg {
                            WM_LBUTTONDBLCLK => MouseButton::Left,
                            WM_MBUTTONDBLCLK => MouseButton::Middle,
                            WM_RBUTTONDBLCLK => MouseButton::Right,
                            WM_XBUTTONDBLCLK => MouseButton::Other(hiword(self.wparam as LPARAM) as u8),
                            _ => unreachable!()
                        };

                        Some(Msg::MouseDoubleDown(button, Point::new(loword(self.lparam) as Px, hiword(self.lparam) as Px)))
                    }
                    WM_LBUTTONUP  |
                    WM_MBUTTONUP  |
                    WM_RBUTTONUP => {
                        let button = match self.msg {
                            WM_LBUTTONUP => MouseButton::Left,
                            WM_MBUTTONUP => MouseButton::Middle,
                            WM_RBUTTONUP => MouseButton::Right,
                            WM_XBUTTONUP => MouseButton::Other(hiword(self.wparam as LPARAM) as u8),
                            _ => unreachable!()
                        };

                        Some(Msg::MouseUp(button, Point::new(loword(self.lparam) as Px, hiword(self.lparam) as Px)))
                    }
                    WM_KEYDOWN => {
                        if let Some(key) = vkey::key_from_code(self.wparam) {
                            let repeated_press = (self.lparam & (1 << 30)) != 0;
                            Some(Msg::KeyDown(key, repeated_press))
                        } else {
                            None
                        }
                    }
                    WM_KEYUP => {
                        if let Some(key) = vkey::key_from_code(self.wparam) {
                            let repeated_press = (self.lparam & (1 << 30)) != 0;
                            Some(Msg::KeyUp(key, repeated_press))
                        } else {
                            None
                        }
                    }
                    WM_SETTEXT => {
                        Some(Msg::SetText(ucs2_str_from_ptr(self.lparam as *const WCHAR)))
                    }
                    WM_GETMINMAXINFO => {
                        // oh my god i'm such a terrible person.
                        //
                        // This is probably the worst pointer hack I've ever had to write, and hopefully the worst one I ever
                        // will have to write. See, we need a mut ref to a SizeBounds struct in order to properly send GetSizeBounds.
                        // However, we can't create a SizeBounds here because that would invalidate lifetime guarantees - so, we
                        // take advantage of the fact that the ptMinTrackSize and ptMaxTrackSize fields of the MINMAXINFO struct are
                        // adjacent to each other and, as such, have the same memory layout as a SizeBounds struct. And then we point
                        // to that.
                        let mut size_bounds = &mut*((&mut(&mut*(self.lparam as *mut MINMAXINFO)).ptMinTrackSize) as *mut POINT as *mut SizeBounds);

                        Some(Msg::GetSizeBounds(size_bounds))
                    }
                    WM_PAINT => {
                        use gdi::PaintInit;
                        Some(Msg::Paint(PaintInit::new(self.hwnd)))
                    }
                    WM_ERASEBKGND => {
                        Some(Msg::EraseBackground)
                    }
                    WM_NOTIFY => {
                        use msg::notify::*;

                        let notify_info = &*(self.lparam as *const NMHDR);
                        let notify_type = match notify_info.code {
                            NM_CHAR => {
                                use std::char;
                                let char_info = &*(self.lparam as *const NMCHAR);
                                char::from_u32(char_info.ch).map(|ch| NotifyType::Char(ch))
                            },
                            NM_FONTCHANGED => Some(NotifyType::FontChanged),
                            NM_HOVER => Some(NotifyType::Hover),
                            NM_KEYDOWN => {
                                let key_info = &*(self.lparam as *const NMKEY);
                                let repeated_press = (key_info.uFlags & (1 << 30)) != 0;
                                vkey::key_from_code(key_info.nVKey as u64).map(|key| NotifyType::KeyDown(key, repeated_press))
                            },
                            NM_KILLFOCUS => Some(NotifyType::KillFocus),
                            NM_LDOWN => Some(NotifyType::LDown),
                            NM_OUTOFMEMORY => Some(NotifyType::OutOfMemory),
                            NM_RELEASEDCAPTURE => Some(NotifyType::ReleasedCapture),
                            NM_RETURN => Some(NotifyType::Return),
                            NM_SETFOCUS => Some(NotifyType::SetFocus),
                            NM_THEMECHANGED => Some(NotifyType::ThemeChanged),
                            NM_TOOLTIPSCREATED => {
                                let tooltip_info = &*(self.lparam as *const NMTOOLTIPSCREATED);
                                Some(NotifyType::TooltipCreated(WindowRef::from_raw(tooltip_info.hwndToolTips)))
                            },
                            TRBN_THUMBPOSCHANGING => {
                                let thumb_pos_info = &*(self.lparam as *const NMTRBTHUMBPOSCHANGING);
                                let reason = match thumb_pos_info.nReason as u64 {
                                    TB_LINEDOWN      => ThumbReason::LineDown,
                                    TB_LINEUP        => ThumbReason::LineUp,
                                    TB_PAGEDOWN      => ThumbReason::PageDown,
                                    TB_PAGEUP        => ThumbReason::PageUp,
                                    TB_ENDTRACK      => ThumbReason::EndTrack,
                                    TB_THUMBPOSITION => ThumbReason::ThumbPosition,
                                    TB_THUMBTRACK    => ThumbReason::ThumbTrack,
                                    TB_BOTTOM        => ThumbReason::Bottom,
                                    TB_TOP           => ThumbReason::Top,
                                    _                => return None
                                };
                                Some(NotifyType::TrackbarThumbPosChanging(thumb_pos_info.dwPos, reason))
                            },
                            _ => None
                        };

                        if let Some(nty) = notify_type {
                            let notification = Notification {
                                source: WindowRef::from_raw(self.hwnd),
                                notify_type: nty
                            };
                            Some(Msg::Notify(notification))
                        } else {
                            None
                        }
                    }

                    _ => None
                }
            }
        };
        if let Some(msg) = msg_option {
            Some((&mut self.0, msg))
        } else {
            None
        }
    }

    pub fn default_window_proc(&mut self) -> i64 {
        unsafe{ comctl32::DefSubclassProc(self.hwnd, self.msg, self.wparam, self.lparam) as i64 }
    }
}
unsafe impl<'a, W: WindowBase, S: Subclass<W>> WindowBase for ProcWindowRef<'a, W, S> {
    #[inline]
    fn hwnd(&self) -> HWND {
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
impl<'a, W: WindowBase, S: Subclass<W>> Deref for ProcWindowRef<'a, W, S> {
    type Target = ProcWindowRefNoMsg<'a, W, S>;

    fn deref(&self) -> &ProcWindowRefNoMsg<'a, W, S> {
        &self.0
    }
}
impl<'a, W: WindowBase, S: Subclass<W>> DerefMut for ProcWindowRef<'a, W, S> {
    fn deref_mut(&mut self) -> &mut ProcWindowRefNoMsg<'a, W, S> {
        &mut self.0
    }
}

// ProcWindowRefNoMsg impls
impl<'a, W: WindowBase, S: Subclass<W>> ProcWindowRefNoMsg<'a, W, S> {
    pub fn subclass_data(&mut self) -> &mut S {
        self.subclass_data
    }

    pub fn send_user_msg(&mut self, msg: S::UserMsg) -> i64 {
        let discriminant = msg.discriminant();
        let encoded_bytes = msg::user::encode(msg);

        unsafe {
            let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
            user32::SendMessageW(self.hwnd, discriminant as UINT + WM_APP, wparam, lparam)
        }
    }
}
unsafe impl<'a, W: WindowBase, S: Subclass<W>> WindowBase for ProcWindowRefNoMsg<'a, W, S> {
    #[inline]
    fn hwnd(&self) -> HWND {
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
    for ProcWindowRefNoMsg<'a, W, S>
}
