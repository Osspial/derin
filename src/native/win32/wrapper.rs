use winapi;
use user32;
use kernel32;
use dwmapi;

use winapi::{UINT, WPARAM, LPARAM};
use winapi::windef::{HWND, HDC};
use winapi::winuser::WNDCLASSEXW;

use std::ptr;
use std::mem;
use std::ops::Drop;
use std::ffi::OsStr;
use std::iter::{FromIterator, once};
use std::os::raw::c_int;
use std::os::windows::ffi::OsStrExt;

use smallvec::SmallVec;

use native::{WindowConfig, NativeResult, NativeError};


type SmallUcs2String = SmallVec<[u16; 128]>;
type Ucs2String = Vec<u16>;
#[derive(Clone)]
pub struct WindowWrapper( pub HWND, pub HDC );

unsafe impl Send for WindowWrapper {}
unsafe impl Sync for WindowWrapper {}

impl WindowWrapper {
    #[inline]
    pub fn new<'a>(config: &WindowConfig, owner: HwndType) -> NativeResult<WindowWrapper> {
        unsafe {
            let (style, style_ex) = {
                use native::InitialState::*;

                let mut style = winapi::WS_SYSMENU;
                let mut style_ex = 0;

                if let HwndType::Child(_) = owner {
                    style |= winapi::WS_CHILD;
                }

                if !config.borderless && !config.tool_window {
                    style |= winapi::WS_CAPTION;

                    if config.resizable {
                        style |= winapi::WS_SIZEBOX;

                        if config.maximizable {
                            style |= winapi::WS_MAXIMIZEBOX;
                        }
                    }

                    if config.minimizable {
                        style |= winapi::WS_MINIMIZEBOX;
                    }

                    style_ex |= winapi::WS_EX_WINDOWEDGE;
                }

                if config.tool_window {
                    style_ex |= winapi::WS_EX_TOOLWINDOW;
                }

                if config.topmost {
                    style_ex |= winapi::WS_EX_TOPMOST;
                }

                match config.initial_state {
                    Windowed    => (),
                    Minimized   => style |= winapi::WS_MINIMIZE,
                    Maximized   => style |= winapi::WS_MAXIMIZE
                }

                (style, style_ex)
            };
            

            let size = match config.size {
                Some(s) => {
                    let mut size_rect = winapi::RECT {
                        left: 0,
                        top: 0,
                        right: s.0,
                        bottom: s.1
                    };

                    user32::AdjustWindowRectEx(&mut size_rect, style, 0, style_ex);
                    (size_rect.right - size_rect.left, size_rect.bottom - size_rect.top)
                }

                None => (winapi::CW_USEDEFAULT, winapi::CW_USEDEFAULT)
            };

            let window_name: SmallUcs2String = ucs2_str(&config.name);
            let window_handle = user32::CreateWindowExW(
                style_ex,
                ROOT_WINDOW_CLASS.as_ptr(),
                window_name.as_ptr() as winapi::LPCWSTR,
                style,
                winapi::CW_USEDEFAULT,
                winapi::CW_USEDEFAULT,
                size.0,
                size.1,
                // This parameter specifies the window's owner. If the window
                // is unowned, then it passes a null pointer to the parameter.
                owner.unwrap_or(ptr::null_mut()),
                ptr::null_mut(),
                kernel32::GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );

            if window_handle == ptr::null_mut() {
                return Err(NativeError::OsError(format!("Error: {}", ::std::io::Error::last_os_error())));
            }

            // If the window should be borderless, make it borderless
            if config.borderless {
                user32::SetWindowLongW(window_handle, -16, 0);
            }

            if config.show_window {
                user32::ShowWindow(window_handle, winapi::SW_SHOW);
            }

            if config.transparent {
                let blur_options = winapi::DWM_BLURBEHIND {
                    dwFlags: 0x01,
                    fEnable: 1,
                    hRgnBlur: ptr::null_mut(),
                    fTransitionOnMaximized: 0
                };

                dwmapi::DwmEnableBlurBehindWindow(window_handle, &blur_options);
            }

            if let Some(ref p) = config.icon {
                let path: SmallUcs2String = ucs2_str(p);

                // Load the 32x32 icon
                let icon = user32::LoadImageW(ptr::null_mut(), path.as_ptr(), winapi::IMAGE_ICON, 32, 32, winapi::LR_LOADFROMFILE);
                if icon != ptr::null_mut() {
                    user32::SendMessageW(window_handle, winapi::WM_SETICON, winapi::ICON_BIG as u64, icon as winapi::LPARAM);
                }
                else {
                    return Err(NativeError::IconLoadError(32));
                }

                // Load the 16x16 icon
                let icon = user32::LoadImageW(ptr::null_mut(), path.as_ptr(), winapi::IMAGE_ICON, 16, 16, winapi::LR_LOADFROMFILE);
                if icon != ptr::null_mut() {
                    user32::SendMessageW(window_handle, winapi::WM_SETICON, winapi::ICON_SMALL as u64, icon as winapi::LPARAM);
                }
                else {
                    return Err(NativeError::IconLoadError(16));
                }
            }

            let hdc = user32::GetDC(window_handle);
            if hdc == ptr::null_mut() {
                return Err(NativeError::OsError(format!("Error: {}", ::std::io::Error::last_os_error())));
            }

            Ok(WindowWrapper(window_handle, hdc))
        }
    }

    #[inline]
    pub fn set_title(&self, title: &str) {
        unsafe {
            let title: SmallVec<[u16; 128]> = ucs2_str(title);
            user32::SetWindowTextW(self.0, title.as_ptr());
        }
    }

    #[inline]
    pub fn show(&self) {
        unsafe {
            user32::ShowWindow(self.0, winapi::SW_SHOW);
        }
    }

    #[inline]
    pub fn hide(&self) {
        unsafe {
            user32::ShowWindow(self.0, winapi::SW_HIDE);
        }
    }

    #[inline]
    pub fn enable(&self) {
        unsafe {
            user32::EnableWindow(self.0, winapi::TRUE);
        }
    }

    #[inline]
    pub fn disable(&self) {
        unsafe {
            user32::EnableWindow(self.0, winapi::FALSE);
        }
    }

    #[inline]
    pub fn get_inner_pos(&self) -> Option<(i32, i32)> {
        use winapi::POINT;

        unsafe {
            let mut point = POINT {
                x: 0,
                y: 0
            };

            match user32::ClientToScreen(self.0, &mut point) {
                0 => None,
                _ => Some((point.x as i32, point.y as i32))
            }
        }
    }

    #[inline]
    pub fn get_outer_pos(&self) -> Option<(i32, i32)> {
        unsafe {
            let mut rect = mem::uninitialized();

            match user32::GetWindowRect(self.0, &mut rect) {
                0 => None,
                _ => Some((rect.left as i32, rect.top as i32))
            }
        }
    }

    #[inline]
    pub fn get_inner_size(&self) -> Option<(u32, u32)> {
        unsafe {
            let mut rect = mem::uninitialized();
            
            match user32::GetClientRect(self.0, &mut rect) {
                0 => None,
                _ => Some(((rect.right - rect.left) as u32, 
                           (rect.bottom - rect.top) as u32))
            }
        }
    }

    #[inline]
    pub fn get_outer_size(&self) -> Option<(u32, u32)> {
        unsafe {
            let mut rect = mem::uninitialized();
            
            match user32::GetWindowRect(self.0, &mut rect) {
                0 => None,
                _ => Some(((rect.right - rect.left) as u32, 
                           (rect.bottom - rect.top) as u32))
            }
        }
    }

    #[inline]
    pub fn set_pos(&self, x: i32, y: i32) -> Option<()> {
        unsafe {
            let result = user32::SetWindowPos(
                self.0,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                winapi::SWP_NOSIZE | winapi::SWP_NOZORDER | winapi::SWP_NOACTIVATE
            );

            match result {
                0 => None,
                _ => Some(())
            }
        }
    }

    #[inline]
    pub fn set_inner_size(&self, x: u32, y: u32) -> Option<()> {
        unsafe {
            let mut rect = winapi::RECT {
                left: 0,
                top: 0,
                right: x as i32,
                bottom: y as i32
            };

            user32::AdjustWindowRectEx(
                &mut rect,
                self.get_style(),
                0,
                self.get_style_ex()
            );

            let result = user32::SetWindowPos(
                self.0,
                ptr::null_mut(),
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                winapi::SWP_NOMOVE | winapi::SWP_NOZORDER | winapi::SWP_NOACTIVATE
            );

            match result {
                0 => None,
                _ => Some(())
            }
        }
    }

    pub fn get_style(&self) -> u32{
        unsafe{ user32::GetWindowLongW(self.0, -16) as u32 }
    }

    pub fn get_style_ex(&self) -> u32 {
        unsafe{ user32::GetWindowLongW(self.0, -20) as u32 }
    }
}

impl Drop for WindowWrapper {
    fn drop(&mut self) {
        unsafe{ user32::PostMessageW(self.0, winapi::WM_DESTROY, 0, 0) };
    }
}


fn ucs2_str<S: ?Sized + AsRef<OsStr>, C: FromIterator<u16>>(s: &S) -> C {
    s.as_ref().encode_wide().chain(once(0)).collect()
}

lazy_static!{
    static ref ROOT_WINDOW_CLASS: Ucs2String = unsafe{
        let class_name: Ucs2String = ucs2_str("Root Window Class");

        let window_class = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as winapi::UINT,
            style: winapi::CS_OWNDC | winapi::CS_VREDRAW | winapi::CS_HREDRAW | winapi::CS_DBLCLKS,
            lpfnWndProc: Some(callback),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: kernel32::GetModuleHandleW(ptr::null()),
            hIcon: ptr::null_mut(),
            hCursor: ptr::null_mut(),
            hbrBackground: ptr::null_mut(),
            lpszMenuName: ptr::null(),
            lpszClassName: class_name.as_ptr(),
            hIconSm: ptr::null_mut()
        };
        user32::RegisterClassExW(&window_class);

        class_name
    };
}

pub enum HwndType {
    Owned(HWND),
    Child(HWND),
    Top
}

impl HwndType {
    fn unwrap_or(self, def: HWND) -> HWND {
        use self::HwndType::*;

        match self {
            Owned(hw) |
            Child(hw)   => hw,
            Top         => def
        }
    }
}

unsafe extern "system" fn callback(hwnd: HWND, msg: UINT,
                                   wparam: WPARAM, lparam: LPARAM)
                                   -> winapi::LRESULT {
    match msg {
        winapi::WM_DESTROY  => {
            user32::DestroyWindow(hwnd);
            0
        }

        _ => user32::DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}
