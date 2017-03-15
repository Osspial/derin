extern crate winapi;
extern crate user32;
extern crate kernel32;
extern crate comctl32;
#[macro_use]
extern crate lazy_static;
extern crate dct;
#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate dww_macros;

pub mod user_msg;
pub mod msg_queue;

use user_msg::UserMsg;

use dct::geometry::{Px, SizeBounds, Rect, Point, OriginRect, OffsetRect};
use dct::events::MouseButton;

use winapi::*;

use std::{ptr, mem, str};
use std::marker::PhantomData;
use std::path::Path;
use std::io::{Result, Error};

use self::ucs2::{WithString, Ucs2String, Ucs2Str, ucs2_str, ucs2_str_from_ptr, UCS2_CONVERTER};

pub enum Msg<'a, U: UserMsg> {
    Wm(Wm<'a>),
    Bm(Bm<'a>),
    User(U)
}

pub enum Wm<'a> {
    Close,
    Size(OriginRect),
    MouseDown(MouseButton, Point),
    MouseDoubleDown(MouseButton, Point),
    MouseUp(MouseButton, Point),
    SetText(&'a Ucs2Str),
    GetSizeBounds(&'a mut SizeBounds)
}

pub enum Bm<'a> {
    GetIdealSize(&'a mut OriginRect)
}



/// A trait representing a subclass on a window. Note that, if multiple subclasses are applied,
/// only the outermost subclass is used.
pub trait Subclass<W: Window> {
    type UserMsg: UserMsg;

    fn subclass_proc(&self, &ProcWindowRef<W, Self>, Msg<Self::UserMsg>) -> i64;
}

const SUBCLASS_ID: UINT_PTR = 0;

lazy_static!{
    static ref BLANK_WINDOW_CLASS: Ucs2String = unsafe {
        let class_name: Ucs2String = ucs2_str("Blank Window Class").collect();

        let window_class = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(user32::DefWindowProcW),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: kernel32::GetModuleHandleW(ptr::null()),
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: (COLOR_MENU + 1) as *mut _,
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: ptr::null_mut()
        };
        user32::RegisterClassExW(&window_class);

        class_name
    };
    static ref BUTTON_CLASS: Ucs2String = ucs2_str("BUTTON").collect();
    static ref STATIC_CLASS: Ucs2String = ucs2_str("STATIC").collect();
}


pub struct WindowIcon {
    pub big: Option<Icon>,
    pub small: Option<Icon>
}

impl AsRef<WindowIcon> for WindowIcon {
    fn as_ref(&self) -> &WindowIcon {self}
}

pub struct WindowBuilder<'a> {
    pub pos: Option<(i32, i32)>,
    pub size: Option<OriginRect>,
    pub window_text: &'a str,
    pub show_window: bool
}

impl<'a> WindowBuilder<'a> {
    pub fn pos(mut self, pos: Option<(i32, i32)>) -> WindowBuilder<'a> {
        self.pos = pos;
        self
    }

    pub fn size(mut self, size: Option<OriginRect>) -> WindowBuilder<'a> {
        self.size = size;
        self
    }

    pub fn window_text(mut self, window_text: &'a str) -> WindowBuilder<'a> {
        self.window_text = window_text;
        self
    }

    pub fn show_window(mut self, show_window: bool) -> WindowBuilder<'a> {
        self.show_window = show_window;
        self
    }

    pub fn build_blank(self) -> BlankBase {
        let window_handle = self.build(WS_CLIPCHILDREN, 0, &BLANK_WINDOW_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        BlankBase(window_handle)
    }

    pub fn build_push_button(self) -> PushButtonBase {
        let window_handle = self.build(BS_PUSHBUTTON, 0, &BUTTON_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        PushButtonBase(window_handle)
    }

    pub fn build_text_label(self) -> TextLabelBase {
        let window_handle = self.build(0, 0, &STATIC_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        TextLabelBase(window_handle)
    }

    fn build(self, style: DWORD, style_ex: DWORD, class: &Ucs2Str) -> HWND {
        UCS2_CONVERTER.with_string(self.window_text, |window_text| unsafe {
            let pos = self.pos.unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT));
            let size = match self.size {
                Some(s) => {
                    let mut size_rect = RECT {
                        left: 0,
                        top: 0,
                        right: s.width() as LONG,
                        bottom: s.height() as LONG
                    };

                    user32::AdjustWindowRectEx(&mut size_rect, 0, 0, 0);
                    (size_rect.right - size_rect.left, size_rect.bottom - size_rect.top)
                }

                None => (0, 0)
            };

            let window_handle = user32::CreateWindowExW(
                style_ex,
                class.as_ptr(),
                window_text.as_ptr(),
                style,
                pos.0, pos.1,
                size.0, size.1,
                ptr::null_mut(),
                ptr::null_mut(),
                kernel32::GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );

            user32::SetWindowLongW(window_handle, GWL_STYLE, style as LONG);

            if self.show_window {
                user32::ShowWindow(window_handle, SW_SHOW);
            }

            window_handle
        })
    }
}

impl<'a> Default for WindowBuilder<'a> {
    #[inline]
    fn default() -> WindowBuilder<'a> {
        WindowBuilder {
            pos: None,
            size: None,
            window_text: "",
            show_window: true
        }
    }
}


pub struct BlankBase( HWND );
impl Window for BlankBase {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {self.0}
}
impl ParentWindow for BlankBase {}
impl Drop for BlankBase {
    fn drop(&mut self) {
        unsafe{ user32::DestroyWindow(self.0) };
    }
}

pub struct PushButtonBase( HWND );
impl Window for PushButtonBase {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {self.0}
}
impl ButtonWindow for PushButtonBase {}
impl Drop for PushButtonBase {
    fn drop(&mut self) {
        unsafe{ user32::DestroyWindow(self.0) };
    }
}

pub struct TextLabelBase( HWND );
impl Window for TextLabelBase {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {self.0}
}
impl TextLabelWindow for TextLabelBase {}
impl Drop for TextLabelBase {
    fn drop(&mut self) {
        unsafe{ user32::DestroyWindow(self.0) };
    }
}


pub struct IconWrapper<I: AsRef<WindowIcon>, W: Window> {
    window: W,
    icon: I
}

pub struct OverlapWrapper<W: Window>( W );

pub struct SubclassWrapper<W: Window, S: Subclass<W>> {
    window: W,
    pub data: Box<S>
}

pub struct UnsafeSubclassWrapper<W: Window, S: Subclass<W>> {
    window: W,
    pub data: S
}

pub struct ProcWindowRef<W: Window, S: Subclass<W> + ?Sized> {
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
    __marker: PhantomData<(W, S)>
}

#[derive(Clone, Copy)]
pub struct ParentRef( HWND );
#[derive(Clone, Copy)]
pub struct WindowRef( HWND );
#[derive(Clone, Copy)]
pub struct UnsafeSubclassRef<U: UserMsg>( HWND, PhantomData<U> );



pub trait Window: Sized {
    unsafe fn hwnd(&self) -> HWND;

    fn as_icon<I: AsRef<WindowIcon>>(self, icon: I) -> IconWrapper<I, Self> {
        let mut icon_window = IconWrapper {
            window: self,
            icon: unsafe{ mem::uninitialized() }
        };
        icon_window.set_icon(icon);
        icon_window
    }

    fn as_overlapped(self, overlapped: bool) -> OverlapWrapper<Self> {
        let window = OverlapWrapper(self);
        window.overlapped(overlapped);
        window
    }


    fn window_ref(&self) -> WindowRef {
        WindowRef( unsafe{self.hwnd()} )
    }

    fn set_text(&self, title: &str) {
        UCS2_CONVERTER.with_string(title, |title_ucs2|
            unsafe{ user32::SetWindowTextW(self.hwnd(), title_ucs2.as_ptr()) }
        );
    }

    fn set_rect(&self, rect: OffsetRect) {
        let adjusted_rect = self.adjust_window_rect(rect);
        unsafe{user32::SetWindowPos(
            self.hwnd(),
            ptr::null_mut(),
            adjusted_rect.topleft.x as c_int,
            adjusted_rect.topleft.y as c_int,
            adjusted_rect.width() as c_int,
            adjusted_rect.height() as c_int,
            SWP_NOOWNERZORDER | SWP_NOZORDER
        )};
    }

    fn bound_to_size_bounds(&self) {
        unsafe {
            let mut rect = mem::zeroed();
            user32::GetWindowRect(self.hwnd(), &mut rect);
            user32::SetWindowPos(
                self.hwnd(),
                ptr::null_mut(),
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOOWNERZORDER | SWP_NOZORDER
            );
        }
    }

    fn adjust_window_rect<R: Rect>(&self, rect: R) -> R {
        use std::cmp;
        let mut winapi_rect = RECT {
            left: rect.topleft().x as LONG,
            top: rect.topleft().y as LONG,
            right: rect.lowright().x as LONG,
            bottom: rect.lowright().y as LONG
        };

        unsafe {user32::AdjustWindowRectEx(
            &mut winapi_rect,
            self.get_style(),
            0,
            self.get_style_ex()
        )};

        let x_offset = -cmp::min(winapi_rect.left, Px::min_value() as LONG);
        let y_offset = -cmp::min(winapi_rect.top, Px::min_value() as LONG);

        winapi_rect.left += x_offset;
        winapi_rect.right += x_offset;
        winapi_rect.top += y_offset;
        winapi_rect.bottom += y_offset;

        // Clamp the values to within the `Px` range bounds
        winapi_rect.right = cmp::min(Px::max_value() as LONG, winapi_rect.right);
        winapi_rect.bottom = cmp::min(Px::max_value() as LONG, winapi_rect.bottom);

        R::from(OffsetRect::new(winapi_rect.left as Px, winapi_rect.top as Px,
                                winapi_rect.right as Px, winapi_rect.bottom as Px))
    }

    fn get_style(&self) -> DWORD {
        unsafe{ user32::GetWindowLongW(self.hwnd(), -16) as DWORD }
    }

    fn get_style_ex(&self) -> DWORD {
        unsafe{ user32::GetWindowLongW(self.hwnd(), -20) as DWORD }
    }

    unsafe fn set_style(&self, style: DWORD) {
        user32::SetWindowLongW(self.hwnd(), GWL_STYLE, style as LONG);
    }

    unsafe fn set_style_ex(&self, style_ex: DWORD) {
        user32::SetWindowLongW(self.hwnd(), GWL_EXSTYLE, style_ex as LONG);
    }

    fn stash_long(&self, long: LONG) {
        unsafe{ user32::SetWindowLongW(self.hwnd(), GWL_USERDATA, long) };
    }

    fn retrieve_long(&self) -> LONG {
        unsafe{ user32::GetWindowLongW(self.hwnd(), GWL_USERDATA) }
    }

    fn enable(&self) {
        unsafe{ user32::EnableWindow(self.hwnd(), TRUE) };
    }

    fn disable(&self) {
        unsafe{ user32::EnableWindow(self.hwnd(), FALSE) };
    }

    fn show(&self, show_window: bool) {
        let show_int = match show_window {
            false => SW_HIDE,
            true  => SW_SHOWNA
        };
        unsafe{ user32::ShowWindow(self.hwnd(), show_int )};
    }

    fn orphan(&self) {
        unsafe {
            user32::SetParent(self.hwnd(), ptr::null_mut());
            self.set_style(self.get_style() & !WS_CHILD);
        }
    }

    fn get_parent_window(&self) -> Option<ParentRef> {
        let parent_hwnd = unsafe{ user32::GetParent(self.hwnd()) };
        if parent_hwnd == ptr::null_mut() {
            None
        } else {
            Some(ParentRef(parent_hwnd))
        }
    }

    fn size_bounds(&self) -> SizeBounds {
        let mut mmi: MINMAXINFO = MINMAXINFO {
            ptMaxTrackSize: POINT {x: LONG::max_value(), y: LONG::max_value()},
            ..unsafe{ mem::zeroed() }
        };
        unsafe{ user32::SendMessageW(self.hwnd(), WM_GETMINMAXINFO, 0, &mut mmi as *mut MINMAXINFO as LPARAM) };

        SizeBounds {
            min: OriginRect::new(mmi.ptMinTrackSize.x as Px, mmi.ptMinTrackSize.y as Px),
            max: OriginRect::new(mmi.ptMaxTrackSize.x as Px, mmi.ptMaxTrackSize.y as Px)
        }
    }
}

pub trait OverlappedWindow: Window {
    /// Set all of the overlapped window properties (i.e. all the other functions in this struct)
    /// to either true or false.
    fn overlapped(&self, overlapped: bool) {
        let new_style = match overlapped {
            true => self.get_style() | WS_OVERLAPPEDWINDOW,
            false => self.get_style() & !WS_OVERLAPPEDWINDOW
        };
        unsafe{ self.set_style(new_style) };
    }

    /// Set whether or not the window has a title bar
    fn title_bar(&self, title_bar: bool) {
        let new_style = match title_bar {
            true => self.get_style() | WS_CAPTION,
            false => self.get_style() & !WS_CAPTION
        };
        unsafe{ self.set_style(new_style) };
    }

    /// Set whether or not the window has a menu bar
    fn menu_bar(&self, menu_bar: bool) {
        let new_style = match menu_bar {
            true => self.get_style() | WS_SYSMENU,
            false => self.get_style() & !WS_SYSMENU
        };
        unsafe{ self.set_style(new_style) };
    }

    /// Set whether or not the window can be resized by dragging the edges
    fn size_border(&self, size_border: bool) {
        let new_style = match size_border {
            true => self.get_style() | WS_SIZEBOX,
            false => self.get_style() & !WS_SIZEBOX
        };
        unsafe{ self.set_style(new_style) };
    }

    /// Set whether or not the window has a minimize button
    fn min_button(&self, min_button: bool) {
        let new_style = match min_button {
            true => self.get_style() | WS_MINIMIZEBOX,
            false => self.get_style() & !WS_MINIMIZEBOX
        };
        unsafe{ self.set_style(new_style) };
    }

    /// Set whether or not the window has a maximize button
    fn max_button(&self, max_button: bool) {
        let new_style = match max_button {
            true => self.get_style() | WS_MAXIMIZEBOX,
            false => self.get_style() & !WS_MAXIMIZEBOX
        };
        unsafe{ self.set_style(new_style) };
    }
}

pub trait IconWindow: Window {
    type I: AsRef<WindowIcon>;

    fn icon_mut(&mut self) -> &mut Self::I;

    fn set_icon(&mut self, icon: Self::I) {
        unsafe {
            let icon_ref = icon.as_ref();
            let big_icon = icon_ref.big.as_ref().map(|icon| icon.0).unwrap_or(ptr::null_mut());
            let small_icon = icon_ref.small.as_ref().map(|icon| icon.0).unwrap_or(ptr::null_mut());

            user32::SendMessageW(self.hwnd(), WM_SETICON, ICON_BIG as WPARAM, big_icon as LPARAM);
            user32::SendMessageW(self.hwnd(), WM_SETICON, ICON_SMALL as WPARAM, small_icon as LPARAM);
        }
        *self.icon_mut() = icon;
    }
}

pub trait ParentWindow: Window {
    fn parent_ref(&self) -> ParentRef {
        ParentRef(unsafe{ self.hwnd() })
    }

    fn add_child_window<W: Window>(&self, child: W) {
        unsafe {
            user32::SetParent(child.hwnd(), self.hwnd());
            child.set_style(child.get_style() | WS_CHILD | WS_CLIPSIBLINGS);
        }
    }
}

pub trait ButtonWindow: Window {
    fn get_ideal_size(&self) -> OriginRect {
        let mut size = SIZE{ cx: 0, cy: 0 };
        unsafe{ user32::SendMessageW(self.hwnd(), BCM_GETIDEALSIZE, 0, &mut size as *mut SIZE as LPARAM) };
        OriginRect::new(size.cx as Px, size.cy as Px)
    }
}

pub trait TextLabelWindow: Window {
    fn min_unclipped_rect(&self) -> OriginRect {
        let text_len = unsafe{ user32::GetWindowTextLengthW(self.hwnd()) };
        UCS2_CONVERTER.with_ucs2_buffer(text_len as usize, |text_buf| unsafe {
            user32::GetWindowTextW(self.hwnd(), text_buf.as_mut_ptr(), text_len);
            self.min_unclipped_rect_raw(text_buf)
        })
    }

    unsafe fn min_unclipped_rect_raw(&self, text: &Ucs2Str) -> OriginRect {
        let mut label_rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0
        };

        let hdc = user32::GetDC(self.hwnd());
        user32::DrawTextW(
            hdc,
            text.as_ptr(),
            -1,
            &mut label_rect,
            DT_CALCRECT
        );
        user32::ReleaseDC(self.hwnd(), hdc);

        OriginRect::new(label_rect.right as Px, label_rect.bottom as Px)
    }
}



// IconWindow impls
impl<I: AsRef<WindowIcon>, W: Window> Window for IconWrapper<I, W> {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {self.window.hwnd()}
}
impl<I: AsRef<WindowIcon>, W: Window> OverlappedWindow for IconWrapper<I, W> {}
impl<I: AsRef<WindowIcon>, W: Window> ParentWindow for IconWrapper<I, W> where W: ParentWindow {}
impl<I: AsRef<WindowIcon>, W: Window> ButtonWindow for IconWrapper<I, W> where W: ButtonWindow {}
impl<I: AsRef<WindowIcon>, W: Window> TextLabelWindow for IconWrapper<I, W> where W: TextLabelWindow {}
impl<I: AsRef<WindowIcon>, W: Window> IconWindow for IconWrapper<I, W> {
    type I = I;
    #[inline]
    fn icon_mut(&mut self) -> &mut I {&mut self.icon}
}


// OverlapWrapper impls
impl<W: Window> Window for OverlapWrapper<W> {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {self.0.hwnd()}
}
impl<W: Window> OverlappedWindow for OverlapWrapper<W> {}
impl<W: Window> ParentWindow for OverlapWrapper<W> where W: ParentWindow {}
impl<W: Window> ButtonWindow for OverlapWrapper<W> where W: ButtonWindow {}
impl<W: Window> TextLabelWindow for OverlapWrapper<W> where W: TextLabelWindow {}
impl<W: Window> IconWindow for OverlapWrapper<W> where W: IconWindow {
    type I = <W as IconWindow>::I;
    #[inline]
    fn icon_mut(&mut self) -> &mut <W as IconWindow>::I {self.0.icon_mut()}
}


// SubclassWrapper impls
impl<W: Window, S: Subclass<W>> SubclassWrapper<W, S> {
    pub fn new(window: W, data: S) -> SubclassWrapper<W, S> {
        let wrapper = SubclassWrapper {
            window: window,
            data: Box::new(data)
        };

        unsafe{ comctl32::SetWindowSubclass(
            wrapper.window.hwnd(),
            Some(subclass_proc::<W, S>),
            SUBCLASS_ID,
            &*wrapper.data as *const S as DWORD_PTR
        ) };
        wrapper
    }

    /// Send a user message, yielding the value returned by `S::subclass_proc`.
    pub fn send_user_msg(&self, msg: S::UserMsg) -> i64 {
        let discriminant = msg.discriminant();
        let encoded_bytes = user_msg::encode(msg);

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
        let encoded_bytes = user_msg::encode(msg);

        unsafe {
            let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
            user32::PostMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam);
        }
    }
}
impl<W: Window, S: Subclass<W>> Window for SubclassWrapper<W, S> {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {self.window.hwnd()}
}
impl<W: Window, S: Subclass<W>> OverlappedWindow for SubclassWrapper<W, S> where W: OverlappedWindow {}
impl<W: Window, S: Subclass<W>> ParentWindow for SubclassWrapper<W, S> where W: ParentWindow {}
impl<W: Window, S: Subclass<W>> ButtonWindow for SubclassWrapper<W, S> where W: ButtonWindow {}
impl<W: Window, S: Subclass<W>> TextLabelWindow for SubclassWrapper<W, S> where W: TextLabelWindow {}
impl<W: Window, S: Subclass<W>> IconWindow for SubclassWrapper<W, S> where W: IconWindow {
    type I = <W as IconWindow>::I;
    #[inline]
    fn icon_mut(&mut self) -> &mut <W as IconWindow>::I {self.window.icon_mut()}
}


// UnsafeSubclassWrapper impls
impl<W: Window, S: Subclass<W>> UnsafeSubclassWrapper<W, S> {
    pub unsafe fn new(window: W, data: S) -> UnsafeSubclassWrapper<W, S> {
        UnsafeSubclassWrapper {
            window: window,
            data: data
        }
    }

    /// Send a user message, yielding the value returned by `S::subclass_proc`.
    ///
    /// Unsafe because it cannot guaruntee that the subclass pointer is pointing to the correct
    /// location.
    pub unsafe fn send_user_msg(&self, msg: S::UserMsg) -> i64 {
        self.unsafe_subclass_ref().send_user_msg(msg)
    }

    /// Post a user message to the message queue associatd with the window.
    ///
    /// Unsafe because it cannot guaruntee that the subclass pointer is pointing to the correct
    /// location.
    pub unsafe fn post_user_msg(&self, msg: S::UserMsg)
            where S::UserMsg: 'static
    {
        self.unsafe_subclass_ref().post_user_msg(msg)
    }

    pub fn unsafe_subclass_ref(&self) -> UnsafeSubclassRef<S::UserMsg> {
        unsafe{ UnsafeSubclassRef(self.hwnd(), PhantomData) }
    }

    pub fn update_subclass_ptr(&self) {
        unsafe {
            comctl32::SetWindowSubclass(
                self.window.hwnd(),
                Some(subclass_proc::<W, S>),
                SUBCLASS_ID,
                &self.data as *const S as DWORD_PTR
            );
        }
    }
}
impl<W: Window, S: Subclass<W>> Window for UnsafeSubclassWrapper<W, S> {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {self.window.hwnd()}
}
impl<W: Window, S: Subclass<W>> OverlappedWindow for UnsafeSubclassWrapper<W, S> where W: OverlappedWindow {}
impl<W: Window, S: Subclass<W>> ParentWindow for UnsafeSubclassWrapper<W, S> where W: ParentWindow {}
impl<W: Window, S: Subclass<W>> ButtonWindow for UnsafeSubclassWrapper<W, S> where W: ButtonWindow {}
impl<W: Window, S: Subclass<W>> TextLabelWindow for UnsafeSubclassWrapper<W, S> where W: TextLabelWindow {}
impl<W: Window, S: Subclass<W>> IconWindow for UnsafeSubclassWrapper<W, S> where W: IconWindow {
    type I = <W as IconWindow>::I;
    #[inline]
    fn icon_mut(&mut self) -> &mut <W as IconWindow>::I {self.window.icon_mut()}
}


// ProcWindowRef impls
impl<W: Window, S: Subclass<W>> ProcWindowRef<W, S> {
    unsafe fn new(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> ProcWindowRef<W, S> {
        ProcWindowRef {
            hwnd: hwnd,
            msg: msg,
            wparam: wparam,
            lparam: lparam,
            __marker: PhantomData
        }
    }

    pub fn send_user_msg(&self, msg: S::UserMsg) -> i64 {
        let discriminant = msg.discriminant();
        let encoded_bytes = user_msg::encode(msg);

        unsafe {
            let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
            user32::SendMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam)
        }
    }

    pub fn default_window_proc(&self, msg: &mut Msg<S::UserMsg>) -> i64 {
        let ret = unsafe{ comctl32::DefSubclassProc(self.hwnd, self.msg, self.wparam, self.lparam) as i64 };
        match *msg {
            Msg::Wm(Wm::GetSizeBounds(ref mut size_bounds)) => {
                assert_eq!(self.msg, WM_GETMINMAXINFO);
                let mmi = unsafe{ *(self.lparam as *mut MINMAXINFO) };
                size_bounds.min = OriginRect::new(mmi.ptMinTrackSize.x as Px, mmi.ptMinTrackSize.y as Px);
                size_bounds.max = OriginRect::new(mmi.ptMaxTrackSize.x as Px, mmi.ptMaxTrackSize.y as Px);
            },
            Msg::Bm(Bm::GetIdealSize(ref mut ideal_size)) => {
                assert_eq!(self.msg, BCM_GETIDEALSIZE);
                let size = unsafe{ *(self.lparam as *mut SIZE) };
                **ideal_size = OriginRect::new(size.cx as Px, size.cy as Px);
            },
            _ => ()
        }
        ret
    }
}
impl<W: Window, S: Subclass<W>> Window for ProcWindowRef<W, S> {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {
        self.hwnd
    }
}
impl<W: Window, S: Subclass<W>> OverlappedWindow for ProcWindowRef<W, S> where W: OverlappedWindow {}
impl<W: Window, S: Subclass<W>> ParentWindow for ProcWindowRef<W, S> where W: ParentWindow {}
impl<W: Window, S: Subclass<W>> ButtonWindow for ProcWindowRef<W, S> where W: ButtonWindow {}
impl<W: Window, S: Subclass<W>> TextLabelWindow for ProcWindowRef<W, S> where W: TextLabelWindow {}


// ParentRef impls
impl Window for ParentRef {
    #[inline]
    unsafe fn hwnd(&self) -> HWND {
        self.0
    }
}
impl ParentWindow for ParentRef {}


impl Window for WindowRef {
    unsafe fn hwnd(&self) -> HWND {
        self.0
    }
}



// UnsafeSubclassRef impls
impl<U: UserMsg> UnsafeSubclassRef<U> {
    pub unsafe fn send_user_msg(&self, msg: U) -> i64 {
        let discriminant = msg.discriminant();
        let encoded_bytes = user_msg::encode(msg);

        let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
        user32::SendMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam)
    }

    pub unsafe fn post_user_msg(&self, msg: U)
            where U: 'static
    {
        let discriminant = msg.discriminant();
        let encoded_bytes = user_msg::encode(msg);

        let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
        user32::PostMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam);
    }
}
impl<U: UserMsg> Window for UnsafeSubclassRef<U> {
    unsafe fn hwnd(&self) -> HWND {
        self.0
    }
}




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

/// Enables win32 visual styles in the hackiest of methods. Basically, this steals the application
/// manifest from `shell32.dll`, which contains the visual styles code, and then enables that
/// manifest here.
pub fn enable_visual_styles() {
    // It's true that this static mut could cause a memory race. However, the only consequence of
    // that memory race is that this function runs more than once, which won't have any bad impacts
    // other than perhaps a slight increase in memory usage.
    static mut ENABLED: bool = false;

    unsafe {
        if !ENABLED {
            const ACTCTX_FLAG_ASSEMBLY_DIRECTORY_VALID: DWORD = 0x004;
            const ACTCTX_FLAG_RESOURCE_NAME_VALID: DWORD = 0x008;
            const ACTCTX_FLAG_SET_PROCESS_DEFAULT: DWORD = 0x010;

            let mut dir = [0u16; MAX_PATH];
            kernel32::GetSystemDirectoryW(dir.as_mut_ptr(), MAX_PATH as u32);
            UCS2_CONVERTER.with_string("shell32.dll", |dll_file_name| {
                let styles_ctx = ACTCTXW {
                    cbSize: mem::size_of::<ACTCTXW>() as u32,
                    dwFlags:
                        ACTCTX_FLAG_ASSEMBLY_DIRECTORY_VALID |
                        ACTCTX_FLAG_RESOURCE_NAME_VALID |
                        ACTCTX_FLAG_SET_PROCESS_DEFAULT,
                    lpSource: dll_file_name.as_ptr(),
                    wProcessorArchitecture: 0,
                    wLangId: 0,
                    lpAssemblyDirectory: dir.as_ptr(),
                    lpResourceName: 124 as LPCWSTR,
                    lpApplicationName: ptr::null_mut(),
                    hModule: ptr::null_mut()
                };

                let mut activation_cookie = 0;
                kernel32::ActivateActCtx(
                    kernel32::CreateActCtxW(&styles_ctx),
                    &mut activation_cookie
                );
            });

            ENABLED = true;
        }
    }
}

unsafe extern "system" fn subclass_proc<W: Window, S: Subclass<W>>
                                       (hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM,
                                        _: UINT_PTR, subclass_data: DWORD_PTR) -> LRESULT
{
    let subclass_data = &*(subclass_data as *const S);

    /// Partially applied function to run S::subclass_proc with a message. This is a macro because
    /// using a closure resulted in lifetime errors.
    macro_rules! run_subclass_proc {
        ($message:expr) => {{S::subclass_proc(subclass_data, &ProcWindowRef::new(hwnd, msg, wparam, lparam), $message) as LRESULT}}
    }

    if WM_APP <= msg && msg <= 0xBFFF {
        let discriminant = (msg - WM_APP) as u16;
        let bytes: [u8; 16] = mem::transmute((wparam, lparam));
        run_subclass_proc!(Msg::User(user_msg::decode(discriminant, bytes)))
    } else {
        match msg {
            WM_CLOSE => run_subclass_proc!(Msg::Wm(Wm::Close)),
            WM_SIZE  => run_subclass_proc!(Msg::Wm(Wm::Size(OriginRect::new(loword(lparam) as Px, hiword(lparam) as Px)))),
            WM_LBUTTONDOWN  |
            WM_MBUTTONDOWN  |
            WM_RBUTTONDOWN  |
            WM_XBUTTONDOWN => {
                let button = match msg {
                    WM_LBUTTONDOWN => MouseButton::Left,
                    WM_MBUTTONDOWN => MouseButton::Middle,
                    WM_RBUTTONDOWN => MouseButton::Right,
                    WM_XBUTTONDOWN => MouseButton::Other(hiword(wparam as LPARAM) as u8),
                    _ => unreachable!()
                };

                run_subclass_proc!(Msg::Wm(Wm::MouseDown(button, Point::new(loword(lparam) as Px, hiword(lparam) as Px))))
            }
            WM_LBUTTONDBLCLK  |
            WM_MBUTTONDBLCLK  |
            WM_RBUTTONDBLCLK => {
                let button = match msg {
                    WM_LBUTTONDBLCLK => MouseButton::Left,
                    WM_MBUTTONDBLCLK => MouseButton::Middle,
                    WM_RBUTTONDBLCLK => MouseButton::Right,
                    WM_XBUTTONDBLCLK => MouseButton::Other(hiword(wparam as LPARAM) as u8),
                    _ => unreachable!()
                };

                run_subclass_proc!(Msg::Wm(Wm::MouseDoubleDown(button, Point::new(loword(lparam) as Px, hiword(lparam) as Px))))
            }
            WM_LBUTTONUP  |
            WM_MBUTTONUP  |
            WM_RBUTTONUP => {
                let button = match msg {
                    WM_LBUTTONUP => MouseButton::Left,
                    WM_MBUTTONUP => MouseButton::Middle,
                    WM_RBUTTONUP => MouseButton::Right,
                    WM_XBUTTONUP => MouseButton::Other(hiword(wparam as LPARAM) as u8),
                    _ => unreachable!()
                };

                run_subclass_proc!(Msg::Wm(Wm::MouseUp(button, Point::new(loword(lparam) as Px, hiword(lparam) as Px))))
            }
            WM_SETTEXT => {
                run_subclass_proc!(Msg::Wm(Wm::SetText(ucs2_str_from_ptr(lparam as *const WCHAR))))
            }
            WM_GETMINMAXINFO => {
                let mut mmi = &mut*(lparam as *mut MINMAXINFO);
                let mut size_bounds = SizeBounds::default();

                let ret = run_subclass_proc!(Msg::Wm(Wm::GetSizeBounds(&mut size_bounds)));

                let window = WindowRef(hwnd);
                size_bounds.min = window.adjust_window_rect(size_bounds.min);
                size_bounds.max = window.adjust_window_rect(size_bounds.max);

                mmi.ptMinTrackSize.x = size_bounds.min.width as LONG;
                mmi.ptMinTrackSize.y = size_bounds.min.height as LONG;
                mmi.ptMaxTrackSize.x = size_bounds.max.width as LONG;
                mmi.ptMaxTrackSize.y = size_bounds.max.height as LONG;

                ret
            }

            BCM_GETIDEALSIZE => {
                let size_winapi = &mut *(lparam as *mut SIZE);
                let mut size = OriginRect::new(size_winapi.cx as Px, size_winapi.cy as Px);
                let ret = run_subclass_proc!(Msg::Bm(Bm::GetIdealSize(&mut size)));
                size_winapi.cx = size.width as LONG;
                size_winapi.cy = size.height as LONG;
                ret
            }
            _ => comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
        }
    }
}

#[inline(always)]
fn loword(lparam: LPARAM) -> WORD {
    lparam as WORD
}

#[inline(always)]
fn hiword(lparam: LPARAM) -> WORD {
    (lparam >> 16) as WORD
}

mod ucs2 {
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

    pub trait WithString {
        fn with_string<S, F, R>(&'static self, S, F) -> R
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
}

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

    #[derive(UserMsg)]
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

    fn test_encoding<U: UserMsg + Debug + Eq + Copy>(msg: M) {
        let discriminant = msg.discriminant();

        assert_eq!(msg, unsafe{ user_msg::decode(discriminant, user_msg::encode(msg)) });
    }
}
