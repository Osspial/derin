use user32;
use kernel32;
use dwmapi;

use winapi::winbase::*;
use winapi::dwmapi::*;
use winapi::winnt::*;
use winapi::windef::*;
use winapi::minwindef::*;
use winapi::winuser::*;

use std::ptr;
use std::mem;
use std::io;
use std::ops::Drop;
use std::ffi::OsStr;
use std::iter::{FromIterator, once};
use std::cell::RefCell;
use std::sync::mpsc::{Sender, Receiver};
use std::os::raw::{c_uint};
use std::os::windows::ffi::OsStrExt;

use smallvec::SmallVec;
use boolinator::Boolinator;

use native::{WindowConfig, NativeResult, NativeError};


pub type SmallUcs2String = SmallVec<[u16; 128]>;
pub type Ucs2String = Vec<u16>;

pub enum WindowNode {
    Toplevel(Toplevel),
    TextButton(TextButton)
}

impl WindowNode {
    fn hwnd(&self) -> HWND {
        match *self {
            WindowNode::Toplevel(ref tl) => (tl.0).0,
            WindowNode::TextButton(ref tb) => tb.wrapper.0
        }
    }

    fn root_hwnd(&self) -> HWND {
        const GA_ROOT: c_uint = 2;

        match *self {
            WindowNode::Toplevel(ref tl) => (tl.0).0,
            WindowNode::TextButton(ref tb) => unsafe{ user32::GetAncestor(tb.wrapper.0, GA_ROOT) }
        }
    }

    pub fn new_text_button(&self, text: &str, receiver: &Receiver<NativeResult<WindowNode>>) -> NativeResult<WindowNode> {
        unsafe {
            let button_create_data = TextButtonCreateData {
                text: text,
                parent: self.hwnd()
            };
            user32::SendMessageW(
                self.root_hwnd(),
                TM_NEWTEXTBUTTON,
                &button_create_data as *const _ as WPARAM,
                0
            );
            receiver.recv()
                .expect("Unexpected close of window channel")
        }
    }
}

pub struct Toplevel( WindowWrapper );

impl Toplevel {
    /// Create a new toplevel window. This is unsafe because it must be called on the correct thread in
    /// order to have the win32 message pump get the messages for this window.
    pub unsafe fn new(config: &WindowConfig) -> NativeResult<Toplevel> {
        let (style, style_ex) = {
            use native::InitialState::*;

            let mut style = WS_SYSMENU;
            let mut style_ex = 0;

            if !config.borderless && !config.tool_window {
                style |= WS_CAPTION;

                if config.resizable {
                    style |= WS_SIZEBOX;

                    if config.maximizable {
                        style |= WS_MAXIMIZEBOX;
                    }
                }

                if config.minimizable {
                    style |= WS_MINIMIZEBOX;
                }

                style_ex |= WS_EX_WINDOWEDGE;
            }

            if config.tool_window {
                style_ex |= WS_EX_TOOLWINDOW;
            }

            if config.topmost {
                style_ex |= WS_EX_TOPMOST;
            }

            match config.initial_state {
                Windowed    => (),
                Minimized   => style |= WS_MINIMIZE,
                Maximized   => style |= WS_MAXIMIZE
            }

            (style, style_ex)
        };
        

        let size = match config.size {
            Some(s) => {
                let mut size_rect = RECT {
                    left: 0,
                    top: 0,
                    right: s.0,
                    bottom: s.1
                };

                user32::AdjustWindowRectEx(&mut size_rect, style, 0, style_ex);
                (size_rect.right - size_rect.left, size_rect.bottom - size_rect.top)
            }

            None => (CW_USEDEFAULT, CW_USEDEFAULT)
        };

        let window_name: SmallUcs2String = ucs2_str(&config.name);
        let window_handle = user32::CreateWindowExW(
            style_ex,
            TOPLEVEL_WINDOW_CLASS.as_ptr(),
            window_name.as_ptr() as LPCWSTR,
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            size.0,
            size.1,
            ptr::null_mut(),
            ptr::null_mut(),
            kernel32::GetModuleHandleW(ptr::null()),
            ptr::null_mut()
        );

        if window_handle == ptr::null_mut() {
            return Err(NativeError::OsError(format!("{}", io::Error::last_os_error())));
        }

        // If the window should be borderless, make it borderless
        if config.borderless {
            user32::SetWindowLongW(window_handle, -16, 0);
        }

        if config.show_window {
            user32::ShowWindow(window_handle, SW_SHOW);
        }

        if config.transparent {
            let blur_options = DWM_BLURBEHIND {
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
            let icon = user32::LoadImageW(ptr::null_mut(), path.as_ptr(), IMAGE_ICON, 32, 32, LR_LOADFROMFILE);
            if icon != ptr::null_mut() {
                user32::SendMessageW(window_handle, WM_SETICON, ICON_BIG as u64, icon as LPARAM);
            }
            else {
                return Err(NativeError::IconLoadError(32));
            }

            // Load the 16x16 icon
            let icon = user32::LoadImageW(ptr::null_mut(), path.as_ptr(), IMAGE_ICON, 16, 16, LR_LOADFROMFILE);
            if icon != ptr::null_mut() {
                user32::SendMessageW(window_handle, WM_SETICON, ICON_SMALL as u64, icon as LPARAM);
            }
            else {
                return Err(NativeError::IconLoadError(16));
            }
        }

        Ok(Toplevel(WindowWrapper(window_handle)))
    }
}

pub struct TextButton {
    wrapper: WindowWrapper,
    text: Ucs2String
}

unsafe impl Send for TextButton {}
unsafe impl Sync for TextButton {}

/// The raw wrapper struct around `HWND`. Upon being dropped, the window is destroyed.
struct WindowWrapper( HWND );
unsafe impl Send for WindowWrapper {}
unsafe impl Sync for WindowWrapper {}

impl WindowWrapper {
    unsafe fn set_title(&self, title: &[u16]) {
        user32::SetWindowTextW(self.0, title.as_ptr());
    }

    fn get_inner_pos(&self) -> Option<(i32, i32)> {
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

    fn get_outer_pos(&self) -> Option<(i32, i32)> {
        unsafe {
            let mut rect = mem::uninitialized();

            match user32::GetWindowRect(self.0, &mut rect) {
                0 => None,
                _ => Some((rect.left as i32, rect.top as i32))
            }
        }
    }

    fn get_inner_size(&self) -> Option<(u32, u32)> {
        unsafe {
            let mut rect = mem::uninitialized();
            
            match user32::GetClientRect(self.0, &mut rect) {
                0 => None,
                _ => Some(((rect.right - rect.left) as u32, 
                           (rect.bottom - rect.top) as u32))
            }
        }
    }

    fn get_outer_size(&self) -> Option<(u32, u32)> {
        unsafe {
            let mut rect = mem::uninitialized();
            
            match user32::GetWindowRect(self.0, &mut rect) {
                0 => None,
                _ => Some(((rect.right - rect.left) as u32, 
                           (rect.bottom - rect.top) as u32))
            }
        }
    }

    fn set_pos(&self, x: i32, y: i32) -> Option<()> {
        unsafe {
            let result = user32::SetWindowPos(
                self.0,
                ptr::null_mut(),
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE
            );

            match result {
                0 => None,
                _ => Some(())
            }
        }
    }

    fn set_inner_size(&self, x: u32, y: u32) -> Option<()> {
        unsafe {
            let mut rect = RECT {
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
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE
            );

            match result {
                0 => None,
                _ => Some(())
            }
        }
    }

    fn get_style(&self) -> u32 {
        unsafe{ user32::GetWindowLongW(self.0, -16) as u32 }
    }

    fn get_style_ex(&self) -> u32 {
        unsafe{ user32::GetWindowLongW(self.0, -20) as u32 }
    }
}

impl Drop for WindowWrapper {
    fn drop(&mut self) {
        unsafe{ user32::PostMessageW(self.0, WM_DESTROY, 0, 0) };
    }
}


fn ucs2_str<S: ?Sized + AsRef<OsStr>, C: FromIterator<u16>>(s: &S) -> C {
    s.as_ref().encode_wide().chain(once(0)).collect()
}

lazy_static!{
    static ref TOPLEVEL_WINDOW_CLASS: Ucs2String = unsafe{
        let class_name: Ucs2String = ucs2_str("Root Window Class");

        let window_class = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
            style: CS_OWNDC | CS_VREDRAW | CS_HREDRAW | CS_DBLCLKS,
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
    static ref BUTTON_CLASS: Ucs2String = ucs2_str("BUTTON");
}

pub struct CallbackData {
    pub window_sender: Sender<NativeResult<WindowNode>>
}

thread_local!{
    pub static CALLBACK_DATA: RefCell<Option<CallbackData>> = RefCell::new(None);
}


// A bunch of different tint messages for creating controls and such. These are all handled by the
// toplevel window, as the child controls each have their own callback specified by windows.

/// Create a title-less push button.
///
/// # Callback parameters
/// * `wparam`: Pointer to `TextButtonCreateData` struct
const TM_NEWTEXTBUTTON: UINT = WM_USER + 0;
struct TextButtonCreateData {
    text: *const str,
    parent: HWND,
}

unsafe extern "system" fn callback(hwnd: HWND, msg: UINT,
                                   wparam: WPARAM, lparam: LPARAM)
                                   -> LRESULT {
    match msg {
        WM_DESTROY  => {
            user32::DestroyWindow(hwnd);
            0
        }

        TM_NEWTEXTBUTTON => {
            println!("NEW TEXT BUTTON");
            let creation_data = &*(wparam as *const TextButtonCreateData);
            let text_ucs2: Ucs2String = ucs2_str(&*creation_data.text);
            let button_hwnd = user32::CreateWindowExW(
                0,
                BUTTON_CLASS.as_ptr(),
                text_ucs2.as_ptr(),
                WS_TABSTOP | WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
                0,
                0,
                64,
                64,
                creation_data.parent,
                ptr::null_mut(),
                kernel32::GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );
            
            CALLBACK_DATA.with(|cd| {
                let cd = cd.borrow();
                let cd = cd.as_ref().unwrap();
                cd.window_sender.send(
                    (button_hwnd != ptr::null_mut()).as_result(
                        WindowNode::TextButton(TextButton {
                            wrapper: WindowWrapper(button_hwnd),
                            text: text_ucs2
                        }),
                        NativeError::OsError(format!("{}", io::Error::last_os_error()))
                    )).ok();
            });
            0
        }

        _ => user32::DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

/// Enables win32 visual styles in the hackiest of methods. Basically, this steals the application
/// manifest from `shell32.dll`, which contains the visual styles code, and then enables that
/// manifest here.
pub unsafe fn enable_visual_styles() {
    const ACTCTX_FLAG_ASSEMBLY_DIRECTORY_VALID: DWORD = 0x004;
    const ACTCTX_FLAG_RESOURCE_NAME_VALID: DWORD = 0x008;
    const ACTCTX_FLAG_SET_PROCESS_DEFAULT: DWORD = 0x010;

    let mut dir = [0u16; MAX_PATH];
    kernel32::GetSystemDirectoryW(dir.as_mut_ptr(), MAX_PATH as u32);
    let dll_file_name: SmallUcs2String = ucs2_str("shell32.dll");

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
}
