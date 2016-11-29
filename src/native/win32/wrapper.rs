use user32;
use kernel32;
use comctl32;
use dwmapi;

use winapi::winbase::*;
use winapi::dwmapi::*;
use winapi::winnt::*;
use winapi::windef::*;
use winapi::minwindef::*;
use winapi::winuser::*;
use winapi::commctrl::*;
use winapi::basetsd::*;

use super::WindowReceiver;
use super::geometry::{Rect, OffsetRect, OriginRect, Point, GridDims, HintedCell};
use ui::layout::{PlaceInCell, NodeSpan, GridSlot, GridSize};

use std::ptr;
use std::mem;
use std::io;
use std::ops::Drop;
use std::ffi::OsStr;
use std::iter::{once};
use std::sync::mpsc::{Sender};
use std::os::raw::{c_int, c_uint};
use std::os::windows::ffi::OsStrExt;
use std::ops::Deref;

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

    pub fn set_slot(&self, slot: GridSlot) {
        unsafe {            
            user32::SendMessageW(
                self.hwnd(),
                DM_SETSLOT,
                &slot as *const GridSlot as WPARAM,
                0
            );
        }
    }

    /// Create a new toplevel window. This is unsafe because it must be called on the correct thread in
    /// order to have the win32 message pump get the messages for this window.
    pub unsafe fn new_toplevel(config: &WindowConfig, callback_data: *mut CallbackData) -> NativeResult<WindowNode> {
        let (style, style_ex) = {
            use native::InitialState::*;

            let mut style = WS_SYSMENU | WS_CLIPCHILDREN;
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

        let window_name: SmallUcs2String = ucs2_str(&config.name).collect();
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

        let node_data = Box::new(NodeData::new(callback_data, GridSlot::default(), GridDims::with_grid_size(GridSize::new(1, 1))));
        comctl32::SetWindowSubclass(window_handle, Some(toplevel_callback), TOPLEVEL_SUBCLASS, Box::into_raw(node_data) as UINT_PTR);

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
            let path: SmallUcs2String = ucs2_str(p).collect();

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

        Ok(WindowNode::Toplevel(Toplevel(WindowWrapper(window_handle))))
    }

    /// Create a new zero-sized text button with no contents.
    pub fn new_text_button(&self, receiver: &WindowReceiver) -> NativeResult<WindowNode> {
        unsafe {
            user32::SendMessageW(
                self.hwnd(),
                DM_NEWTEXTBUTTON,
                0,
                0
            );
            receiver.recv()
                .expect("Unexpected close of window channel")
        }
    }
}

pub struct Toplevel( WindowWrapper );
pub struct LayoutGroup( WindowWrapper );

pub struct TextButton {
    wrapper: WindowWrapper,
    text: Ucs2String
}

unsafe impl Send for TextButton {}
unsafe impl Sync for TextButton {}

impl TextButton {
    pub fn set_text(&mut self, text: &str) {
        self.text.clear();
        self.text.extend(ucs2_str(text));
        unsafe{ self.wrapper.set_title(&self.text) }
    }

    pub fn set_rect(&mut self, rect: OffsetRect) {
        self.wrapper.set_pos(rect.topleft);
        self.wrapper.set_inner_size(rect.width(), rect.height());
    }

    pub fn get_ideal_rect(&self) -> OriginRect {
        TextButton::get_ideal_rect_raw(self.wrapper.0)
    }

    #[inline]
    fn get_ideal_rect_raw(hwnd: HWND) -> OriginRect {
        unsafe {
            let mut ideal_size = SIZE {
                cx: 0,
                cy: 0
            };
            user32::SendMessageW(
                hwnd,
                BCM_GETIDEALSIZE,
                0,
                &mut ideal_size as *mut SIZE as LPARAM
            );
            OriginRect::new(ideal_size.cx, ideal_size.cy)
        }
    }
}

/// The raw wrapper struct around `HWND`. Upon being dropped, the window is destroyed.
struct WindowWrapper( HWND );
unsafe impl Send for WindowWrapper {}
unsafe impl Sync for WindowWrapper {}

impl Deref for WindowWrapper {
    type Target = WindowWrapperRef;

    fn deref(&self) -> &WindowWrapperRef {
        unsafe{ &*(&self.0 as *const _ as *const WindowWrapperRef) }
    }
}

impl Drop for WindowWrapper {
    fn drop(&mut self) {
        unsafe{ user32::PostMessageW(self.0, DM_DESTROYWINDOW, 0, 0) };
    }
}

struct WindowWrapperRef( HWND );
unsafe impl Send for WindowWrapperRef {}
unsafe impl Sync for WindowWrapperRef {}

impl WindowWrapperRef {
    /// Take a null-terminated UCS2-formatted string slice and set the window title to it
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

    fn set_pos(&self, pos: Point) -> Option<()> {
        unsafe {
            let result = user32::SetWindowPos(
                self.0,
                ptr::null_mut(),
                pos.x,
                pos.y,
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

    fn set_inner_size(&self, x: c_int, y: c_int) -> Option<()> {
        unsafe {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: x,
                bottom: y
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

    #[inline(always)]
    fn enum_children<F>(&self, mut func: F)
            where for<'a> F: FnMut(&'a WindowWrapperRef) -> bool 
    {
        unsafe extern "system" fn enum_child_proc<F>(hwnd: HWND, func: LPARAM) -> BOOL
                where for<'a> F: FnMut(&'a WindowWrapperRef) -> bool
        {
            let func = &mut *(func as *mut F);
            match func(&WindowWrapperRef(hwnd)) {
                true => TRUE,
                false => FALSE
            }
        }

        unsafe {
            user32::EnumChildWindows(
                self.0,
                Some(enum_child_proc::<F>),
                &mut func as *mut F as LPARAM
            );
        }
    }

    fn get_style(&self) -> u32 {
        unsafe{ user32::GetWindowLongW(self.0, -16) as u32 }
    }

    fn get_style_ex(&self) -> u32 {
        unsafe{ user32::GetWindowLongW(self.0, -20) as u32 }
    }

    fn update_child_scales(&self, layout: &mut GridDims) {
        self.enum_children(|child| unsafe {
            user32::BringWindowToTop(child.0);
            user32::SendMessageW(child.0, DM_RECALCSCALE, layout as *mut _ as WPARAM, 0);
            true
        });
    }
}


fn ucs2_str<'a, S: ?Sized + AsRef<OsStr>>(s: &'a S) -> impl 'a + Iterator<Item=u16> {
    s.as_ref().encode_wide().chain(once(0))
}

lazy_static!{
    static ref TOPLEVEL_WINDOW_CLASS: Ucs2String = unsafe{
        let class_name: Ucs2String = ucs2_str("Root Window Class").collect();

        let window_class = WNDCLASSEXW {
            cbSize: mem::size_of::<WNDCLASSEXW>() as UINT,
            style: 0,
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
}

pub enum RawEvent {
    CloseClicked,
    ToplevelResized(c_int, c_int)
}

pub struct CallbackData {
    pub window_sender: Sender<NativeResult<WindowNode>>,
    pub event_sender: Sender<RawEvent>
}

struct NodeData {
    callback_data: *mut CallbackData,
    slot: GridSlot,
    layout: GridDims
}

impl NodeData {
    #[inline]
    fn new(callback_data: *mut CallbackData, slot: GridSlot, layout: GridDims) -> NodeData {
        NodeData {
            callback_data: callback_data,
            slot: slot,
            layout: layout
        }
    }
}


const TOPLEVEL_SUBCLASS: UINT_PTR = 0;
const BUTTON_SUBCLASS: UINT_PTR = 1;

// A bunch of different derin messages for creating controls and such. These are all handled by the
// toplevel window, as the child controls each have their own callback specified by windows.

const DM_DESTROYWINDOW: UINT = WM_APP + 0;
/// Create a title-less push button.
///
/// # Callback parameters
/// * `wparam`: Parent `HWND` handle
const DM_NEWTEXTBUTTON: UINT = WM_APP + 1;
const DM_RECALCSCALE: UINT = WM_APP + 2;
const DM_SETSLOT: UINT = WM_APP + 3;

unsafe fn try_create_children(hwnd: HWND, msg: UINT, 
                       wparam: WPARAM, lparam: LPARAM,
                       nd: &mut NodeData) -> LRESULT
{
    let cd = &mut *nd.callback_data;
    match msg {
        DM_NEWTEXTBUTTON => {
            let node_data = Box::new(NodeData::new(
                cd,
                GridSlot::default(),
                GridDims::new()
            ));

            let button_hwnd = user32::CreateWindowExW(
                0,
                BUTTON_CLASS.as_ptr(),
                ptr::null(),
                WS_TABSTOP | WS_VISIBLE | WS_CHILD | BS_PUSHBUTTON,
                0,
                0,
                0,
                0,
                hwnd,
                ptr::null_mut(),
                kernel32::GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );
            comctl32::SetWindowSubclass(
                button_hwnd,
                Some(pushbutton_callback),
                BUTTON_SUBCLASS,
                Box::into_raw(node_data) as DWORD_PTR
            );

            cd.window_sender.send(
                (button_hwnd != ptr::null_mut()).as_result(
                    WindowNode::TextButton(TextButton {
                        wrapper: WindowWrapper(button_hwnd),
                        text: Ucs2String::new()
                    }),
                    NativeError::OsError(format!("{}", io::Error::last_os_error()))
                )).ok();

            WindowWrapperRef(hwnd).update_child_scales(&mut nd.layout);
            0
        },
        _ => comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}

unsafe extern "system"
    fn toplevel_callback(hwnd: HWND, msg: UINT,
                         wparam: WPARAM, lparam: LPARAM,
                         _: UINT_PTR, nd: DWORD_PTR) -> LRESULT
{
    let nd = &mut *(nd as *mut NodeData);
    let cd = &mut *nd.callback_data;

    match msg {
        WM_CLOSE => {
            cd.event_sender.send(RawEvent::CloseClicked).ok();
            0
        },

        DM_DESTROYWINDOW => {
            user32::DestroyWindow(hwnd);
            user32::PostQuitMessage(0);
            Box::from_raw(nd);
            0
        },

        WM_SIZE => {
            let hwnd_ref = WindowWrapperRef(hwnd);
            let new_rect = OriginRect::new(loword(lparam) as c_int, hiword(lparam) as c_int);

            cd.event_sender.send(RawEvent::ToplevelResized(new_rect.width(), new_rect.height())).ok();

            nd.layout.expand_size_px(new_rect).unwrap_or_else(|_| {
                nd.layout.zero_all();
                nd.layout.expand_size_px(new_rect).unwrap();
            });

            hwnd_ref.update_child_scales(&mut nd.layout);
            0
        },
        _ => try_create_children(hwnd, msg, wparam, lparam, nd)
    }
}

unsafe extern "system"
    fn pushbutton_callback(hwnd: HWND, msg: UINT,
                           wparam: WPARAM, lparam: LPARAM,
                           _: UINT_PTR, nd: DWORD_PTR) -> LRESULT
{
    let nd = &mut *(nd as *mut NodeData);
    match msg {
        WM_LBUTTONDOWN => {
            println!("button pressed!");
            comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
        }

        WM_NCDESTROY => {
            // Free the memory for the node data associated with this button.
            Box::from_raw(nd);
            comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
        }
 
        DM_RECALCSCALE => {
            let cell_x = nd.slot.node_span.x.start.unwrap();
            let cell_y = nd.slot.node_span.y.start.unwrap();

            let layout = &mut *(wparam as *mut GridDims);
            let min_rect = TextButton::get_ideal_rect_raw(hwnd);
            let mut rect_hint = HintedCell::new(
                layout.get_cell_rect(
                    cell_x,
                    cell_y
                ).unwrap(), nd.slot.place_in_cell);
            let new_rect = rect_hint.transform_min_rect(min_rect);
            layout.expand_cell_rect(cell_x, cell_y, rect_hint.outer_rect().into());

            let hwnd_ref = WindowWrapperRef(hwnd);
            hwnd_ref.set_pos(new_rect.topleft);
            hwnd_ref.set_inner_size(new_rect.width(), new_rect.height());
            1
        }

        DM_SETSLOT => {
            nd.slot = *(wparam as *const GridSlot);
            1
        }

        _ => comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
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
    let dll_file_name: SmallUcs2String = ucs2_str("shell32.dll").collect();

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

#[inline(always)]
fn loword(lparam: LPARAM) -> WORD {
    lparam as WORD
}

#[inline(always)]
fn hiword(lparam: LPARAM) -> WORD {
    (lparam >> 16) as WORD
}
