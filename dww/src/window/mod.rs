/// Window trait helper macro. This is here instead of in a submodule because two submodules need
/// to use it.
macro_rules! impl_window_traits {
    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
        for $window:ty
    ) => ();

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            BaseWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => (
        unsafe impl<$($lt,)* W: $($window_bound +)* $(, $gen: $gen_bound)*> BaseWindow for $window {
            #[inline]
            fn hwnd(&self) -> HWND {
                self.inner().hwnd()
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    );

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            IconWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: IconWindow $(+ $window_bound)* $(, $gen: $gen_bound)*> IconWindow for $window {
            type IconSm = W::IconSm;
            type IconLg = W::IconLg;

            fn set_icon_sm(&mut self, icon: Option<Self::IconSm>) -> Option<Self::IconSm> {
                self.inner_mut().set_icon_sm(icon)
            }
            fn set_icon_lg(&mut self, icon: Option<Self::IconLg>) -> Option<Self::IconLg> {
                self.inner_mut().set_icon_lg(icon)
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    };

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            FontWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: FontWindow $(+ $window_bound)* $(, $gen: $gen_bound)*> FontWindow for $window {
            type Font = W::Font;
            fn set_font(&mut self, font: W::Font) {
                self.inner_mut().set_font(font)
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    };

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            $trait_name:ident
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: $trait_name $(+ $window_bound)* $(, $gen: $gen_bound)*> $trait_name for $window {}
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    };
}

pub mod wrappers;
pub mod refs;

use self::wrappers::*;
use self::refs::*;

use winapi::*;
use {comctl32, user32, kernel32, vkey};
use gdi::{DeviceContext, RetrievedContext};
use gdi::img::Icon;
use gdi::text::{Font, DefaultFont, TextFormat};
use ucs2::{ucs2_str, ucs2_str_from_ptr, Ucs2Str, Ucs2String, WithString, UCS2_CONVERTER};
use msg::user::UserMsg;

use dct::geometry::*;
use dct::hints::SizeBounds;
use dct::buttons::MouseButton;

use std::{ptr, mem};
use std::borrow::Borrow;
use std::io::{Result, Error};

fn init_common_controls() {
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

#[derive(Clone, Copy, Debug)]
pub struct WindowBuilder<'a> {
    pub pos: Option<Point>,
    pub size: Option<OriginRect>,
    pub text: &'a str,
    pub show_window: bool
}

impl<'a> WindowBuilder<'a> {
    pub fn rect(mut self, rect: OffsetRect) -> WindowBuilder<'a> {
        self.pos = Some(rect.topleft);
        self.size = Some(rect.into());

        self
    }

    pub fn pos(mut self, pos: Point) -> WindowBuilder<'a> {
        self.pos = Some(pos);
        self
    }

    pub fn size(mut self, size: OriginRect) -> WindowBuilder<'a> {
        self.size = Some(size);
        self
    }

    pub fn text(mut self, text: &'a str) -> WindowBuilder<'a> {
        self.text = text;
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
        let window_handle = self.build(BS_PUSHBUTTON, 0, Some(parent.hwnd()), &BUTTON_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        let mut window = PushButtonBase(window_handle, unsafe{ mem::uninitialized() });
        window.set_font(font);
        window
    }

    pub fn build_group_box<P: ParentWindow>(self, parent: &P) -> GroupBoxBase {
        self.build_group_box_with_font(parent, DefaultFont)
    }

    pub fn build_group_box_with_font<P: ParentWindow, F: Borrow<Font>>(self, parent: &P, font: F) -> GroupBoxBase<F> {
        let window_handle = self.build(BS_GROUPBOX, 0, Some(parent.hwnd()), &BUTTON_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        let mut window = GroupBoxBase(window_handle, unsafe{ mem::uninitialized() });
        window.set_font(font);
        window
    }

    pub fn build_text_label<P: ParentWindow>(self, parent: &P) -> TextLabelBase {
        self.build_text_label_with_font(parent, DefaultFont)
    }

    pub fn build_text_label_with_font<P: ParentWindow, F: Borrow<Font>>(self, parent: &P, font: F) -> TextLabelBase<F> {
        let window_handle = self.build(0, 0, Some(parent.hwnd()), &STATIC_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        let mut window = TextLabelBase(window_handle, unsafe{ mem::uninitialized() });
        window.set_font(font);
        window
    }

    pub fn build_progress_bar<P: ParentWindow>(self, parent: &P) -> ProgressBarBase {
        init_common_controls();
        let window_handle = self.build(PBS_SMOOTHREVERSE, 0, Some(parent.hwnd()), &PROGRESS_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        ProgressBarBase(window_handle)
    }

    pub fn build_trackbar<P: ParentWindow>(self, parent: &P) -> TrackbarBase {
        let window_handle = self.build(TBS_NOTIFYBEFOREMOVE, 0, Some(parent.hwnd()), &TRACKBAR_CLASS);
        assert_ne!(window_handle, ptr::null_mut());
        TrackbarBase(window_handle)
    }

    fn build(self, style: DWORD, style_ex: DWORD, parent: Option<HWND>, class: &Ucs2Str) -> HWND {
        UCS2_CONVERTER.with_string(self.text, |text| unsafe {
            let pos = self.pos.unwrap_or(Point::new(CW_USEDEFAULT, CW_USEDEFAULT));
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
                text.as_ptr(),
                style,
                pos.x, pos.y,
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
            text: "",
            show_window: true
        }
    }
}

macro_rules! base_window {
    () => ();
    (pub struct $name:ident$(<$font_generic:ident>)*; $($rest:tt)*) => {
        pub struct $name$(<$font_generic: Borrow<Font> = DefaultFont>)*( HWND $(, $font_generic)* );
        unsafe impl$(<$font_generic: Borrow<Font>>)* BaseWindow for $name$(<$font_generic>)* {
            #[inline]
            fn hwnd(&self) -> HWND {self.0}
        }
        unsafe impl$(<$font_generic: Borrow<Font>>)* MutWindow for $name$(<$font_generic>)* {}
        unsafe impl$(<$font_generic: Borrow<Font>>)* OwnedWindow for $name$(<$font_generic>)* {}
        $(
            unsafe impl<$font_generic: Borrow<Font>> FontWindow for $name<$font_generic> {
                type Font = $font_generic;
                fn set_font(&mut self, font: F) {
                    unsafe{
                        user32::SendMessageW(self.hwnd(), WM_SETFONT, font.borrow().hfont() as WPARAM, TRUE as LPARAM);
                        self.1 = font;
                    }
                }
            }
        )*
        impl$(<$font_generic: Borrow<Font>>)* Drop for $name$(<$font_generic>)* {
            fn drop(&mut self) {
                unsafe{ user32::DestroyWindow(self.0) };
            }
        }
        base_window!{$($rest)*}
    }
}

base_window! {
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
unsafe impl<F: Borrow<Font>> TextLabelWindow for TextLabelBase<F> {}
unsafe impl ProgressBarWindow for ProgressBarBase {}
unsafe impl TrackbarWindow for TrackbarBase {}


const SUBCLASS_ID: UINT_PTR = 0;

lazy_static!{
    static ref BLANK_WINDOW_CLASS: Ucs2String = unsafe {
        let class_name: Ucs2String = ucs2_str("Blank BaseWindow Class").collect();

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

/// A trait representing a subclass on a window. Note that, if multiple subclasses are applied,
/// only the outermost subclass is used.
pub trait Subclass<W: BaseWindow> {
    type UserMsg: UserMsg;

    fn subclass_proc(window: ProcWindowRef<W, Self>) -> i64;
}

impl<W: BaseWindow> Subclass<W> for () {
    type UserMsg = !;
    fn subclass_proc(mut window: ProcWindowRef<W, ()>) -> i64 {
        window.default_window_proc()
    }
}

impl<W, F> Subclass<W> for F
        where W: BaseWindow,
              F: for<'a> FnMut(ProcWindowRef<W, ()>) -> i64
{
    type UserMsg = !;
    fn subclass_proc(window: ProcWindowRef<W, F>) -> i64 {
        let (func, data) = window.split_subclass_data();
        func(data)
    }
}

pub unsafe trait BaseWindow: Sized {
    fn hwnd(&self) -> HWND;

    fn window_ref(&self) -> WindowRef {
        unsafe{ WindowRef::from_raw(self.hwnd()) }
    }

    fn adjust_window_rect<R: Rect>(&self, rect: R) -> R {
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

        // Catch overflows
        if rect.topleft().x < winapi_rect.left {
            winapi_rect.left = rect.topleft().x;
        }
        if rect.topleft().y < winapi_rect.top {
            winapi_rect.top = rect.topleft().y;
        }
        if rect.lowright().x > winapi_rect.right {
            winapi_rect.right = rect.lowright().x;
        }
        if rect.lowright().y > winapi_rect.bottom {
            winapi_rect.bottom = rect.lowright().y;
        }

        R::from(OffsetRect::new(winapi_rect.left as Px, winapi_rect.top as Px,
                                winapi_rect.right as Px, winapi_rect.bottom as Px))
    }

    #[inline]
    fn get_parent(&self) -> Option<ParentRef> {
        let parent = unsafe{ user32::GetParent(self.hwnd()) };
        if ptr::null_mut() != parent {
            Some(unsafe{ ParentRef::from_raw(parent) })
        } else {
            None
        }
    }

    #[inline]
    fn move_before<W: BaseWindow>(&self, window: &W) -> Result<()> {
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
    fn move_after<W: BaseWindow>(&self, window: &W) -> Result<()> {
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
    fn make_topmost(&self, topmost: bool) {
        unsafe {
            let insert_after = match topmost {
                true => HWND_TOPMOST,
                false => HWND_NOTOPMOST
            };
            user32::SetWindowPos(
                self.hwnd(),
                insert_after,
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

    fn invalidate_rect(&self, erase: bool, rect: Option<OffsetRect>) {
        let winapi_rect: RECT;
        let rect_ptr = if let Some(rect) = rect {
            winapi_rect = RECT {
                left: rect.topleft.x as LONG,
                top: rect.topleft.y as LONG,
                right: rect.lowright.x as LONG,
                bottom: rect.lowright.y as LONG
            };
            &winapi_rect
        } else {
            ptr::null()
        };

        unsafe{ user32::InvalidateRect(self.hwnd(), rect_ptr, erase as BOOL) };
    }

    fn validate_rect(&self, rect: Option<OffsetRect>) {
        let winapi_rect: RECT;
        let rect_ptr = if let Some(rect) = rect {
            winapi_rect = RECT {
                left: rect.topleft.x as LONG,
                top: rect.topleft.y as LONG,
                right: rect.lowright.x as LONG,
                bottom: rect.lowright.y as LONG
            };
            &winapi_rect
        } else {
            ptr::null()
        };

        unsafe{ user32::ValidateRect(self.hwnd(), rect_ptr) };
    }

    fn get_dc(&self) -> Option<RetrievedContext> {
        unsafe{ RetrievedContext::retrieve_dc(self.hwnd()) }
    }
}

pub unsafe trait MutWindow: BaseWindow {
    fn window_ref_mut(&mut self) -> WindowRefMut {
        unsafe{ WindowRefMut::from_raw(self.hwnd()) }
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

pub unsafe trait WrapperWindow: OwnedWindow {
    type Inner: OwnedWindow;

    fn inner(&self) -> &Self::Inner;
    fn inner_mut(&mut self) -> &mut Self::Inner;
}

pub unsafe trait OwnedWindow: MutWindow {
    fn with_icons<I>(self, icon_sm: Option<I>, icon_lg: Option<I>) -> IconWrapper<Self, I>
            where I: Icon
    {
        self.with_icons_dif_types(icon_sm, icon_lg)
    }

    fn with_icons_dif_types<S, L>(self, icon_sm: Option<S>, icon_lg: Option<L>) -> IconWrapper<Self, S, L>
            where S: Icon, L: Icon
    {
        let mut icon_window = IconWrapper {
            window: self,
            icon_sm: unsafe{ mem::uninitialized() },
            icon_lg: unsafe{ mem::uninitialized() }
        };
        icon_window.set_icon_sm(icon_sm);
        icon_window.set_icon_lg(icon_lg);
        icon_window
    }

    fn as_overlapped(self, overlapped: bool) -> OverlapWrapper<Self> {
        let window = OverlapWrapper(self);
        window.overlapped(overlapped);
        window
    }
}

pub unsafe trait FontWindow: OwnedWindow {
    type Font: Borrow<Font>;
    fn set_font(&mut self, font: Self::Font);
}

pub unsafe trait OverlappedWindow: BaseWindow {
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

pub unsafe trait IconWindow: OwnedWindow {
    type IconSm: Icon;
    type IconLg: Icon;

    fn set_icon_sm(&mut self, icon: Option<Self::IconSm>) -> Option<Self::IconSm>;
    fn set_icon_lg(&mut self, icon: Option<Self::IconLg>) -> Option<Self::IconLg>;
}

pub unsafe trait ParentWindow: BaseWindow {
    fn parent_ref(&self) -> ParentRef {
        unsafe{ ParentRef::from_raw(self.hwnd()) }
    }

    fn clip_children(&self, clip_children: bool) {
        let new_style = match clip_children {
            true => self.get_style() | WS_CLIPCHILDREN,
            false => self.get_style() & !WS_CLIPCHILDREN
        };
        unsafe{ self.set_style(new_style) };
    }

    fn add_child_window<W: BaseWindow>(&self, child: &W) {
        unsafe {
            let child_style = child.get_style() | WS_CHILD;
            child.set_style(child_style);
            user32::SetParent(child.hwnd(), self.hwnd());
        }
    }
}

pub unsafe trait OrphanableWindow: BaseWindow {
    fn orphan(&self) {
        unsafe {
            let child_style = self.get_style() & !WS_CHILD;
            self.set_style(child_style);
            user32::SetParent(self.hwnd(), ptr::null_mut());
        }
    }
}

pub unsafe trait ButtonWindow: MutWindow {
    fn get_ideal_size(&self) -> OriginRect {
        let mut size = SIZE{ cx: 0, cy: 0 };
        unsafe{ user32::SendMessageW(self.hwnd(), BCM_GETIDEALSIZE, 0, &mut size as *mut SIZE as LPARAM) };
        OriginRect::new(size.cx as Px, size.cy as Px)
    }
}

pub unsafe trait TextLabelWindow: BaseWindow {
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

pub unsafe trait ProgressBarWindow: BaseWindow {
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

pub unsafe trait TrackbarWindow: BaseWindow {
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

#[derive(Clone)]
pub struct WindowIterTopDown {
    next_window: HWND
}

#[derive(Clone)]
pub struct WindowIterBottomUp {
    next_window: HWND
}


impl Iterator for WindowIterTopDown {
    type Item = WindowRef;

    fn next(&mut self) -> Option<WindowRef> {
        if ptr::null_mut() != self.next_window {
            let ret = unsafe{ Some(WindowRef::from_raw(self.next_window)) };
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
            let ret = unsafe{ Some(WindowRef::from_raw(self.next_window)) };
            self.next_window = unsafe{ user32::GetWindow(self.next_window, GW_HWNDPREV) };
            ret
        } else {
            None
        }
    }
}

unsafe extern "system" fn subclass_proc<W: BaseWindow, S: Subclass<W>>
                                       (hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM,
                                        _: UINT_PTR, subclass_data: DWORD_PTR) -> LRESULT
{
    S::subclass_proc(ProcWindowRef::new(hwnd, msg, wparam, lparam, &mut *(subclass_data as *mut S))) as LRESULT
}

#[inline(always)]
fn loword(lparam: LPARAM) -> WORD {
    lparam as WORD
}

#[inline(always)]
fn hiword(lparam: LPARAM) -> WORD {
    (lparam >> 16) as WORD
}
