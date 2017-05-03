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

pub mod user_msg;
pub mod msg_queue;
pub mod hdc;
pub mod notify;
mod vkey;

use user_msg::UserMsg;

use dct::buttons::{MouseButton, Key};
use dct::geometry::{Px, Rect, Point, OriginRect, OffsetRect};
use dct::hints::SizeBounds;

use winapi::*;

use std::{ptr, mem, str};
use std::marker::{PhantomData, Send, Sync};
use std::path::Path;
use std::io::{Result, Error};
use std::cell::UnsafeCell;
use std::borrow::Borrow;

use self::hdc::{DeviceContext, RetrievedContext, TextFormat, PaintInit};
use self::notify::{Notification, NotifyType, ThumbReason};
use self::ucs2::{WithString, Ucs2String, Ucs2Str, ucs2_str, ucs2_str_from_ptr, UCS2_CONVERTER};

#[derive(Debug)]
pub enum Msg<'a, U: UserMsg> {
    Wm(Wm<'a>),
    User(U)
}

#[derive(Debug)]
pub enum Wm<'a> {
    Close,
    Size(OriginRect),
    MouseDown(MouseButton, Point),
    MouseDoubleDown(MouseButton, Point),
    MouseUp(MouseButton, Point),
    KeyDown(Key, RepeatCount),
    KeyUp(Key, RepeatCount),
    SetText(&'a Ucs2Str),
    Paint(PaintInit<'a>),
    EraseBackground,
    Notify(Notification),
    GetSizeBounds(&'a mut SizeBounds)
}

pub type RepeatCount = u16;



/// A trait representing a subclass on a window. Note that, if multiple subclasses are applied,
/// only the outermost subclass is used.
pub trait Subclass<W: WindowBase> {
    type UserMsg: UserMsg;

    fn subclass_proc(&mut ProcWindowRef<W, Self>, Msg<Self::UserMsg>) -> i64;
}

const SUBCLASS_ID: UINT_PTR = 0;

lazy_static!{
    static ref BLANK_WINDOW_CLASS: Ucs2String = unsafe {
        let class_name: Ucs2String = ucs2_str("Blank WindowBase Class").collect();

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
    static ref PROGRESS_CLASS: Ucs2String = ucs2_str("msctls_progress32").collect();
    static ref TRACKBAR_CLASS: Ucs2String = ucs2_str("msctls_trackbar32").collect();
}


pub struct WindowIcon {
    pub big: Option<Icon>,
    pub small: Option<Icon>
}

#[derive(Clone, Copy, Debug)]
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
        let window_handle = self.build(WS_CLIPCHILDREN, 0, None, &BLANK_WINDOW_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        BlankBase(window_handle)
    }

    pub fn build_push_button<P: ParentWindow>(self, parent: &P) -> PushButtonBase {
        self.build_push_button_with_font(parent, DefaultFont)
    }

    pub fn build_push_button_with_font<P: ParentWindow, F: Borrow<Font>>(self, parent: &P, font: F) -> PushButtonBase<F> {
        let window_handle = self.build(BS_PUSHBUTTON, 0, unsafe{ Some(parent.hwnd()) }, &BUTTON_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        let mut window = PushButtonBase(window_handle, unsafe{ mem::uninitialized() });
        window.set_font(font);
        window
    }

    pub fn build_group_box<P: ParentWindow>(self, parent: &P) -> GroupBoxBase {
        self.build_group_box_with_font(parent, DefaultFont)
    }

    pub fn build_group_box_with_font<P: ParentWindow, F: Borrow<Font>>(self, parent: &P, font: F) -> GroupBoxBase<F> {
        let window_handle = self.build(BS_GROUPBOX, 0, unsafe{ Some(parent.hwnd()) }, &BUTTON_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        let mut window = GroupBoxBase(window_handle, unsafe{ mem::uninitialized() });
        window.set_font(font);
        window
    }

    pub fn build_text_label<P: ParentWindow>(self, parent: &P) -> TextLabelBase {
        self.build_text_label_with_font(parent, DefaultFont)
    }

    pub fn build_text_label_with_font<P: ParentWindow, F: Borrow<Font>>(self, parent: &P, font: F) -> TextLabelBase<F> {
        let window_handle = self.build(0, 0, unsafe{ Some(parent.hwnd()) }, &STATIC_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        let mut window = TextLabelBase(window_handle, unsafe{ mem::uninitialized() });
        window.set_font(font);
        window
    }

    pub fn build_progress_bar<P: ParentWindow>(self, parent: &P) -> ProgressBarBase {
        let window_handle = self.build(PBS_SMOOTHREVERSE, 0, unsafe{ Some(parent.hwnd()) }, &PROGRESS_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        ProgressBarBase(window_handle)
    }

    pub fn build_trackbar<P: ParentWindow>(self, parent: &P) -> TrackbarBase {
        let window_handle = self.build(TBS_NOTIFYBEFOREMOVE, 0, unsafe{ Some(parent.hwnd()) }, &TRACKBAR_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        TrackbarBase(window_handle)
    }

    fn build(self, style: DWORD, style_ex: DWORD, parent: Option<HWND>, class: &Ucs2Str) -> HWND {
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
            let style = style | parent.map(|_| WS_CHILD | WS_CLIPSIBLINGS).unwrap_or(0);

            let window_handle = user32::CreateWindowExW(
                style_ex,
                class.as_ptr(),
                window_text.as_ptr(),
                style,
                pos.0, pos.1,
                size.0, size.1,
                parent.unwrap_or(ptr::null_mut()),
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


macro_rules! base_wrapper {
    () => ();
    (pub struct $name:ident$(<$font_generic:ident>)*; $($rest:tt)*) => {
        pub struct $name$(<$font_generic: Borrow<Font> = DefaultFont>)*( HWND $(, $font_generic)* );
        unsafe impl$(<$font_generic: Borrow<Font>>)* WindowBase for $name$(<$font_generic>)* {
            #[inline]
            unsafe fn hwnd(&self) -> HWND {self.0}
        }
        unsafe impl$(<$font_generic: Borrow<Font>>)* WindowMut for $name$(<$font_generic>)* {}
        unsafe impl$(<$font_generic: Borrow<Font>>)* WindowOwned for $name$(<$font_generic>)* {}
        $(
            unsafe impl<$font_generic: Borrow<Font>> WindowFont<$font_generic> for $name<$font_generic> {
                unsafe fn font_mut(&mut self) -> &mut $font_generic {
                    &mut self.1
                }
            }
        )*
        impl$(<$font_generic: Borrow<Font>>)* Drop for $name$(<$font_generic>)* {
            fn drop(&mut self) {
                unsafe{ user32::DestroyWindow(self.0) };
            }
        }
        base_wrapper!{$($rest)*}
    }
}

base_wrapper! {
    pub struct BlankBase;
    pub struct PushButtonBase<F>;
    pub struct GroupBoxBase<F>;
    pub struct TextLabelBase<F>;
    pub struct ProgressBarBase;
    pub struct TrackbarBase;
}

unsafe impl ParentWindow for BlankBase {}
unsafe impl OrphanableWindow for BlankBase {}

unsafe impl<F: Borrow<Font>> ButtonWindow for PushButtonBase<F> {}
unsafe impl<F: Borrow<Font>> ParentWindow for GroupBoxBase<F> {}
unsafe impl<F: Borrow<Font>> TextLabelWindow for TextLabelBase<F> {}
unsafe impl ProgressBarWindow for ProgressBarBase {}
unsafe impl TrackbarWindow for TrackbarBase {}

pub struct IconWrapper<W: WindowBase, I: Borrow<WindowIcon>> {
    window: W,
    icon: I
}

pub struct OverlapWrapper<W: WindowBase>( W );

pub struct SubclassWrapper<W: WindowBase, S: Subclass<W>> {
    window: W,
    data: Box<UnsafeCell<S>>
}

pub struct UnsafeSubclassWrapper<W: WindowBase, S: Subclass<W>> {
    window: W,
    data: UnsafeCell<S>
}

pub struct ProcWindowRef<'a, W: WindowBase, S: 'a + Subclass<W> + ?Sized> {
    hwnd: HWND,
    msg: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_data: &'a mut S,
    __marker: PhantomData<(W, S)>
}

#[derive(Debug, Clone, Copy)]
pub enum TickPosition {
    BottomRight,
    TopLeft,
    Both,
    None
}

impl Default for TickPosition {
    fn default() -> TickPosition {
        TickPosition::BottomRight
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentRef( HWND );
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRef( HWND );
#[derive(Debug, PartialEq, Eq)]
pub struct WindowRefMut<'a>( HWND, PhantomData<&'a mut ()> );
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsafeSubclassRef<'a, U: UserMsg>( HWND, PhantomData<(U, PhantomData<&'a mut ()>)> );

#[derive(Clone)]
pub struct WindowIterTopDown {
    next_window: HWND
}

#[derive(Clone)]
pub struct WindowIterBottomUp {
    next_window: HWND
}

pub unsafe trait WindowBase: Sized {
    unsafe fn hwnd(&self) -> HWND;

    fn window_ref(&self) -> WindowRef {
        WindowRef(unsafe{ self.hwnd() })
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

    #[inline]
    fn get_parent(&self) -> Option<ParentRef> {
        let parent = unsafe{ user32::GetParent(self.hwnd()) };
        if ptr::null_mut() != parent {
            Some(ParentRef(parent))
        } else {
            None
        }
    }

    #[inline]
    fn move_before<W: WindowBase>(&self, window: &W) -> Result<()> {
        unsafe {
            // Windows only provides functions for moving windows after other windows, so we need
            // to get the window before the provided window and then move this window after that
            // window.
            let mut window_after = user32::GetWindow(window.hwnd(), GW_HWNDPREV);
            if ptr::null_mut() == window_after {
                window_after = HWND_BOTTOM;
            }

            let result = user32::SetWindowPos(
                self.hwnd(),
                window_after,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            );
            if result != 0 {
                Ok(())
            } else {
                Err(Error::last_os_error())
            }
        }
    }

    #[inline]
    fn move_after<W: WindowBase>(&self, window: &W) -> Result<()> {
        unsafe {
            let result = user32::SetWindowPos(
                self.hwnd(),
                window.hwnd(),
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            );
            if result != 0 {
                Ok(())
            } else {
                Err(Error::last_os_error())
            }
        }
    }

    #[inline]
    fn move_to_bottom(&self) {
        unsafe {
            user32::SetWindowPos(
                self.hwnd(),
                HWND_BOTTOM,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            );
        }
    }

    #[inline]
    fn move_to_top(&self) {
        unsafe {
            user32::SetWindowPos(
                self.hwnd(),
                HWND_TOP,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            );
        }
    }

    #[inline]
    fn windows_below(&self) -> WindowIterTopDown {
        WindowIterTopDown {
            // The window iterators stores the window that will be returned next by the iterator,
            // not the window that is below (or above) the window that will be returned next. So,
            // we need to get the window below this now instead of waiting for the iterator.
            next_window: unsafe{ user32::GetWindow(self.hwnd(), GW_HWNDNEXT) }
        }
    }

    #[inline]
    fn windows_above(&self) -> WindowIterBottomUp {
        WindowIterBottomUp {
            next_window: unsafe{ user32::GetWindow(self.hwnd(), GW_HWNDPREV) }
        }
    }

    fn get_outer_size(&self) -> OriginRect {
        unsafe {
            let mut rect: RECT = mem::zeroed();
            user32::GetWindowRect(self.hwnd(), &mut rect);
            OriginRect::new((rect.right - rect.left) as Px, (rect.bottom - rect.top) as Px)
        }
    }

    fn get_inner_size(&self) -> OriginRect {
        unsafe {
            let mut rect: RECT = mem::zeroed();
            user32::GetClientRect(self.hwnd(), &mut rect);
            OriginRect::new((rect.right - rect.left) as Px, (rect.bottom - rect.top) as Px)
        }
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

    fn get_dc(&self) -> Option<RetrievedContext> {
        unsafe{ RetrievedContext::retrieve_dc(self.hwnd()) }
    }
}

pub unsafe trait WindowMut: WindowBase {
    fn window_ref_mut(&mut self) -> WindowRefMut {
        WindowRefMut(unsafe{ self.hwnd() }, PhantomData)
    }

    fn set_text(&mut self, title: &str) {
        UCS2_CONVERTER.with_string(title, |title_ucs2|
            unsafe{ user32::SetWindowTextW(self.hwnd(), title_ucs2.as_ptr()) }
        );
    }

    fn set_text_fn<F>(&mut self, title_fn: F)
            where F: FnOnce(&Self) -> &str
    {
        UCS2_CONVERTER.with_string(title_fn(self), |title_ucs2|
            unsafe{ user32::SetWindowTextW(self.hwnd(), title_ucs2.as_ptr()) }
        );
    }

    fn set_text_noprefix(&mut self, title: &str) {
        UCS2_CONVERTER.with_string_noprefix(title, |title_ucs2|
            unsafe{ user32::SetWindowTextW(self.hwnd(), title_ucs2.as_ptr()) }
        );
    }

    fn set_text_noprefix_fn<F>(&mut self, title_fn: F)
            where F: FnOnce(&Self) -> &str
    {
        UCS2_CONVERTER.with_string_noprefix(title_fn(self), |title_ucs2|
            unsafe{ user32::SetWindowTextW(self.hwnd(), title_ucs2.as_ptr()) }
        );
    }

    fn set_rect(&mut self, rect: OffsetRect) {
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

    fn bound_to_size_bounds(&mut self) {
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

    fn enable(&mut self) {
        unsafe{ user32::EnableWindow(self.hwnd(), TRUE) };
    }

    fn disable(&mut self) {
        unsafe{ user32::EnableWindow(self.hwnd(), FALSE) };
    }

    fn show(&mut self, show_window: bool) {
        let show_int = match show_window {
            false => SW_HIDE,
            true  => SW_SHOWNA
        };
        unsafe{ user32::ShowWindow(self.hwnd(), show_int )};
    }

    fn size_bounds(&mut self) -> SizeBounds {
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

pub unsafe trait WindowOwned: WindowMut {
    fn as_icon<I: Borrow<WindowIcon>>(self, icon: I) -> IconWrapper<Self, I> {
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
}

pub unsafe trait WindowFont<F: Borrow<Font>>: WindowBase {
    unsafe fn font_mut(&mut self) -> &mut F;
    fn set_font(&mut self, font: F) {
        unsafe{
            user32::SendMessageW(self.hwnd(), WM_SETFONT, font.borrow().0 as WPARAM, TRUE as LPARAM);
            *self.font_mut() = font;
        }
    }
}

pub unsafe trait OverlappedWindow: WindowBase {
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

pub unsafe trait IconWindow: WindowOwned {
    type I: Borrow<WindowIcon>;

    fn set_icon(&mut self, icon: Self::I);
    unsafe fn set_window_icon(&self, icon: &WindowIcon) {
        let big_icon = icon.big.as_ref().map(|icon| icon.0).unwrap_or(ptr::null_mut());
        let small_icon = icon.small.as_ref().map(|icon| icon.0).unwrap_or(ptr::null_mut());

        user32::SendMessageW(self.hwnd(), WM_SETICON, ICON_BIG as WPARAM, big_icon as LPARAM);
        user32::SendMessageW(self.hwnd(), WM_SETICON, ICON_SMALL as WPARAM, small_icon as LPARAM);
    }
}

pub unsafe trait ParentWindow: WindowBase {
    fn parent_ref(&self) -> ParentRef {
        ParentRef(unsafe{ self.hwnd() })
    }

    fn add_child_window<W: WindowBase>(&self, child: &W) {
        unsafe {
            let child_style = child.get_style() | WS_CHILD;
            child.set_style(child_style);
            user32::SetParent(child.hwnd(), self.hwnd());
        }
    }
}

pub unsafe trait OrphanableWindow: WindowBase {
    fn orphan(&self) {
        unsafe {
            let child_style = self.get_style() & !WS_CHILD;
            self.set_style(child_style);
            user32::SetParent(self.hwnd(), ptr::null_mut());
        }
    }
}

pub unsafe trait ButtonWindow: WindowMut {
    fn get_ideal_size(&self) -> OriginRect {
        let mut size = SIZE{ cx: 0, cy: 0 };
        unsafe{ user32::SendMessageW(self.hwnd(), BCM_GETIDEALSIZE, 0, &mut size as *mut SIZE as LPARAM) };
        OriginRect::new(size.cx as Px, size.cy as Px)
    }
}

pub unsafe trait TextLabelWindow: WindowBase {
    fn min_unclipped_rect(&self) -> OriginRect {
        let text_len = unsafe{ user32::GetWindowTextLengthW(self.hwnd()) };
        UCS2_CONVERTER.with_ucs2_buffer(text_len as usize, |text_buf| unsafe {
            user32::GetWindowTextW(self.hwnd(), text_buf.as_mut_ptr(), text_len);
            self.min_unclipped_rect_ucs2(text_buf)
        })
    }

    unsafe fn min_unclipped_rect_ucs2(&self, text: &Ucs2Str) -> OriginRect {
        self.get_dc().expect("Could not get DC").calc_text_rect_ucs2(text, TextFormat::default())
    }
}

pub unsafe trait ProgressBarWindow: WindowBase {
    fn set_range(&mut self, min: WORD, max: WORD) {
        let lparam = min as LPARAM | ((max as LPARAM) << 16);
        unsafe{ user32::SendMessageW(self.hwnd(), PBM_SETRANGE, 0, lparam) };
    }

    fn get_range(&self) -> (WORD, WORD) {
        let mut range = PBRANGE{ iLow: 0, iHigh: 0 };
        unsafe{ user32::SendMessageW(self.hwnd(), PBM_GETRANGE, 0, &mut range as *mut _ as LPARAM) };
        (range.iLow as WORD, range.iHigh as WORD)
    }

    fn set_progress(&mut self, progress: WORD) {
        unsafe{ user32::SendMessageW(self.hwnd(), PBM_SETPOS, progress as WPARAM, 0) };
    }

    fn delta_progress(&mut self, delta: i16) {
        unsafe{ user32::SendMessageW(self.hwnd(), PBM_DELTAPOS, delta as WPARAM, 0) };
    }

    fn set_step(&mut self, step: u16) {
        unsafe{ user32::SendMessageW(self.hwnd(), PBM_SETSTEP, step as WPARAM, 0) };
    }

    fn step(&mut self) {
        unsafe{ user32::SendMessageW(self.hwnd(), PBM_STEPIT, 0, 0) };
    }

    fn get_progress(&self) -> WORD {
        unsafe{ user32::SendMessageW(self.hwnd(), PBM_GETPOS, 0, 0) as WORD }
    }

    fn set_marquee(&mut self, marquee: bool) {
        unsafe{
            user32::SendMessageW(self.hwnd(), PBM_SETMARQUEE, marquee as WPARAM, 0);
            let new_style = if marquee {
                self.get_style() | PBS_MARQUEE
            } else {
                self.get_style() & !PBS_MARQUEE
            };
            self.set_style(new_style);
        }
    }

    fn set_vertical(&mut self, vertical: bool) {
        let progress = self.get_progress();
        let new_style = if vertical {
            self.get_style() | PBS_VERTICAL
        } else {
            self.get_style() & !PBS_VERTICAL
        };
        unsafe{ self.set_style(new_style) };
        self.set_progress(progress);
    }

    fn is_marquee(&self) -> bool {
        self.get_style() & PBS_MARQUEE != 0
    }

    fn is_vertical(&self) -> bool {
        self.get_style() & PBS_VERTICAL != 0
    }
}

pub unsafe trait TrackbarWindow: WindowBase {
    fn set_pos(&mut self, pos: u32) {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_SETPOS, TRUE as WPARAM, pos as LPARAM) };
    }

    fn get_pos(&self) -> u32 {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_GETPOS, 0, 0) as u32 }
    }

    fn set_range(&mut self, min: u32, max: u32) {
        unsafe {
            user32::SendMessageW(self.hwnd(), TBM_SETRANGEMIN, FALSE as WPARAM, min as LPARAM);
            user32::SendMessageW(self.hwnd(), TBM_SETRANGEMAX, TRUE as WPARAM, max as LPARAM);
        }
    }

    fn set_range_min(&mut self, min: u32) {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_SETRANGEMIN, TRUE as WPARAM, min as LPARAM) };
    }

    fn set_range_max(&mut self, max: u32) {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_SETRANGEMAX, TRUE as WPARAM, max as LPARAM) };
    }

    fn get_range(&self) -> (u32, u32) {
        unsafe {
            let min = user32::SendMessageW(self.hwnd(), TBM_GETRANGEMIN, 0, 0) as u32;
            let max = user32::SendMessageW(self.hwnd(), TBM_GETRANGEMAX, 0, 0) as u32;
            (min, max)
        }
    }

    /// Automatically add tick marks to the trackbar, clearing all other ticks.
    fn auto_ticks(&mut self, frequency: u32) {
        unsafe {
            let style = self.get_style() | TBS_AUTOTICKS;
            self.set_style(style);
            user32::SendMessageW(self.hwnd(), TBM_SETTICFREQ, frequency as WPARAM, 0);
        }
    }

    /// Add a tick at the specified position on the trackbar.
    fn add_tick(&mut self, pos: u32) {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_SETTIC, 0, pos as LPARAM) };
    }

    /// Clear all ticks from the trackbar.
    fn clear_ticks(&mut self) {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_CLEARTICS, TRUE as WPARAM, 0) };
    }

    fn set_tick_position(&self, tick_position: TickPosition) {
        let new_style = match tick_position {
            TickPosition::BottomRight =>
                (self.get_style() | TBS_BOTTOM | TBS_RIGHT) & !(TBS_TOP | TBS_LEFT | TBS_BOTH | TBS_NOTICKS),
            TickPosition::TopLeft =>
                (self.get_style() | TBS_TOP | TBS_LEFT) & !(TBS_BOTTOM | TBS_RIGHT | TBS_BOTH | TBS_NOTICKS),
            TickPosition::Both =>
                (self.get_style() | TBS_BOTH) & !(TBS_BOTTOM | TBS_RIGHT | TBS_TOP | TBS_LEFT | TBS_NOTICKS),
            TickPosition::None =>
                (self.get_style() | TBS_NOTICKS) & !(TBS_BOTTOM | TBS_RIGHT | TBS_TOP | TBS_LEFT | TBS_BOTH)
        };
        unsafe{ self.set_style(new_style) };
    }

    fn set_vertical(&self, vertical: bool) {
        let new_style = if vertical {
            self.get_style() | TBS_VERT
        } else {
            self.get_style() & !TBS_VERT
        };
        unsafe{ self.set_style(new_style) };
    }

    fn show_slider(&self, show_slider: bool) {
        let new_style = if show_slider {
            self.get_style() & !TBS_NOTHUMB
        } else {
            self.get_style() | TBS_NOTHUMB
        };
        unsafe{ self.set_style(new_style) };
    }

    fn show_sel_range(&self, sel_range: bool) {
        let new_style = if sel_range {
            self.get_style() | TBS_ENABLESELRANGE
        } else {
            self.get_style() & !TBS_ENABLESELRANGE
        };
        unsafe{ self.set_style(new_style) };
    }

    fn set_sel_range(&mut self, start: u32, end: u32) {
        unsafe {
            user32::SendMessageW(self.hwnd(), TBM_SETSELSTART, FALSE as WPARAM, start as LPARAM);
            user32::SendMessageW(self.hwnd(), TBM_SETSELEND, TRUE as WPARAM, end as LPARAM);
        }
    }

    fn set_sel_start(&mut self, start: u32) {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_SETSELSTART, TRUE as WPARAM, start as LPARAM) };
    }

    fn set_sel_end(&mut self, end: u32) {
        unsafe{ user32::SendMessageW(self.hwnd(), TBM_SETSELEND, TRUE as WPARAM, end as LPARAM) };
    }
}

macro_rules! impl_window_traits {
    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            ;
        for $window:ty
    ) => ();

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            WindowBase($self_ident:ident) => $window_expr:expr;
        for $window:ty
    ) => (
        unsafe impl<$($lt,)* W: $($window_bound +)* $(, $gen: $gen_bound)*> WindowBase for $window {
            #[inline]
            unsafe fn hwnd(&self) -> HWND {
                let $self_ident = self;
                $window_expr.hwnd()
            }
        }
    );

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            WindowBase($self_ident:ident) => $window_expr:expr;
            IconWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: IconWindow $(+ $window_bound)* $(, $gen: $gen_bound)*> IconWindow for $window {
            type I = <W as IconWindow>::I;
            #[inline]
            fn set_icon(&mut self, icon: <W as IconWindow>::I) {
                let $self_ident = self;
                unsafe{ $window_expr.set_window_icon(icon.borrow()) };
                $window_expr.set_icon(icon);
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                WindowBase($self_ident) => $window_expr;
                $($trait_rest),*
            for $window
        }
    };

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            WindowBase($self_ident:ident) => $window_expr:expr;
            WindowFont
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* F: Borrow<Font>, W: WindowFont<F> $(+ $window_bound)* $(, $gen: $gen_bound)*> WindowFont<F> for $window {
            unsafe fn font_mut(&mut self) -> &mut F {
                let $self_ident = self;
                $window_expr.font_mut()
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                WindowBase($self_ident) => $window_expr;
                $($trait_rest),*
            for $window
        }
    };

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            $(WindowBase($self_ident:ident) => $window_expr:expr)*;
            $trait_name:ident
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: $trait_name $(+ $window_bound)* $(, $gen: $gen_bound)*> $trait_name for $window {}
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $(WindowBase($self_ident) => $window_expr)*;
                $($trait_rest),*
            for $window
        }
    };
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
        unsafe{ UnsafeSubclassRef(self.hwnd(), PhantomData).post_user_msg(msg) }
    }

    pub fn unsafe_subclass_ref(&mut self) -> UnsafeSubclassRef<S::UserMsg> {
        unsafe{ UnsafeSubclassRef(self.hwnd(), PhantomData) }
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


// ProcWindowRef impls
impl<'a, W: WindowBase, S: Subclass<W>> ProcWindowRef<'a, W, S> {
    unsafe fn new(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM, subclass_data: &'a mut S) -> ProcWindowRef<'a, W, S> {
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
        let encoded_bytes = user_msg::encode(msg);

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
            Msg::Wm(Wm::GetSizeBounds(ref mut size_bounds)) => {
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
        let encoded_bytes = user_msg::encode(msg);

        let (wparam, lparam): (WPARAM, LPARAM) = mem::transmute(encoded_bytes);
        user32::SendMessageW(self.hwnd(), discriminant as UINT + WM_APP, wparam, lparam)
    }

    pub fn post_user_msg(&self, msg: U)
            where U: 'static
    {
        unsafe {
            let discriminant = msg.discriminant();
            let encoded_bytes = user_msg::encode(msg);

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

impl Iterator for WindowIterTopDown {
    type Item = WindowRef;

    fn next(&mut self) -> Option<WindowRef> {
        if ptr::null_mut() != self.next_window {
            let ret = Some(WindowRef(self.next_window));
            self.next_window = unsafe{ user32::GetWindow(self.next_window, GW_HWNDNEXT) };
            ret
        } else {
            None
        }
    }
}

impl Iterator for WindowIterBottomUp {
    type Item = WindowRef;

    fn next(&mut self) -> Option<WindowRef> {
        if ptr::null_mut() != self.next_window {
            let ret = Some(WindowRef(self.next_window));
            self.next_window = unsafe{ user32::GetWindow(self.next_window, GW_HWNDPREV) };
            ret
        } else {
            None
        }
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

unsafe extern "system" fn subclass_proc<W: WindowBase, S: Subclass<W>>
                                       (hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM,
                                        _: UINT_PTR, subclass_data: DWORD_PTR) -> LRESULT
{
    /// Partially applied function to run S::subclass_proc with a message. This is a macro because
    /// using a closure resulted in lifetime errors.
    macro_rules! run_subclass_proc {
        ($message:expr) => {{S::subclass_proc(&mut ProcWindowRef::new(hwnd, msg, wparam, lparam, &mut *(subclass_data as *mut S)), $message) as LRESULT}}
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
            WM_KEYDOWN => {
                if let Some(key) = vkey::key_from_code(wparam) {
                    let repeat_count = (wparam & 0xFFFF) as u16;
                    run_subclass_proc!(Msg::Wm(Wm::KeyDown(key, repeat_count)))
                } else {
                    1
                }
            }
            WM_KEYUP => {
                if let Some(key) = vkey::key_from_code(wparam) {
                    let repeat_count = (wparam & 0xFFFF) as u16;
                    run_subclass_proc!(Msg::Wm(Wm::KeyUp(key, repeat_count)))
                } else {
                    1
                }
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
            WM_PAINT => {
                run_subclass_proc!(Msg::Wm(Wm::Paint(PaintInit::new(hwnd))))
            }
            WM_ERASEBKGND => {
                run_subclass_proc!(Msg::Wm(Wm::EraseBackground))
            }
            WM_NOTIFY => {
                let notify_info = &*(lparam as *const NMHDR);
                let notify_type = match notify_info.code {
                    NM_CHAR => {
                        use std::char;
                        let char_info = &*(lparam as *const NMCHAR);
                        char::from_u32(char_info.ch).map(|ch| NotifyType::Char(ch))
                    },
                    NM_FONTCHANGED => Some(NotifyType::FontChanged),
                    NM_HOVER => Some(NotifyType::Hover),
                    NM_KEYDOWN => {
                        let key_info = &*(lparam as *const NMKEY);
                        let repeat_count = (key_info.uFlags & 0xFFFF) as u16;
                        vkey::key_from_code(key_info.nVKey as u64).map(|key| NotifyType::KeyDown(key, repeat_count))
                    },
                    NM_KILLFOCUS => Some(NotifyType::KillFocus),
                    NM_LDOWN => Some(NotifyType::LDown),
                    NM_OUTOFMEMORY => Some(NotifyType::OutOfMemory),
                    NM_RELEASEDCAPTURE => Some(NotifyType::ReleasedCapture),
                    NM_RETURN => Some(NotifyType::Return),
                    NM_SETFOCUS => Some(NotifyType::SetFocus),
                    NM_THEMECHANGED => Some(NotifyType::ThemeChanged),
                    NM_TOOLTIPSCREATED => {
                        let tooltip_info = &*(lparam as *const NMTOOLTIPSCREATED);
                        Some(NotifyType::TooltipCreated(WindowRef(tooltip_info.hwndToolTips)))
                    },
                    TRBN_THUMBPOSCHANGING => {
                        let thumb_pos_info = &*(lparam as *const NMTRBTHUMBPOSCHANGING);
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
                            _                => return comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
                        };
                        Some(NotifyType::TrackbarThumbPosChanging(thumb_pos_info.dwPos, reason))
                    },
                    _ => None
                };

                if let Some(nty) = notify_type {
                    let notification = Notification {
                        source: WindowRef(hwnd),
                        notify_type: nty
                    };
                    run_subclass_proc!(Msg::Wm(Wm::Notify(notification)))
                } else {
                    comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
                }
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

pub mod ucs2 {
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
