mod mcvec;

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
use dle::{LayoutEngine, UpdateQueue, LayoutUpdate, Container, ContainerRef, Widget, WidgetData};
use dle::hints::{SizeBounds, WidgetLayoutInfo};
use dle::geometry::{Rect, OffsetRect, OriginRect};
use ui::layout::GridSize;

use self::mcvec::MCVec;

use std::ptr;
use std::mem;
use std::io;
use std::slice;
use std::cmp;
use std::ops::Drop;
use std::ffi::OsStr;
use std::iter::{once};
use std::sync::mpsc::{Sender};
use std::os::raw::{c_int};
use std::os::windows::ffi::OsStrExt;
use std::ops::Deref;
use std::rc::Rc;
use std::cell::RefCell;

use smallvec::SmallVec;
use boolinator::Boolinator;

use native::{WindowConfig, NativeResult, NativeError};


pub type SmallUcs2String = SmallVec<[u16; 128]>;
pub type Ucs2String = Vec<u16>;

pub enum WindowNode {
    Toplevel(Toplevel),
    LayoutGroup(LayoutGroup),
    TextButton(TextButton)
}

impl WindowNode {
    fn hwnd(&self) -> HWND {
        match *self {
            WindowNode::Toplevel(ref tl) => (tl.0).0,
            WindowNode::LayoutGroup(ref lg) => (lg.0).0,
            WindowNode::TextButton(ref tb) => tb.wrapper.0
        }
    }

    pub fn set_layout_info(&self, wli: WidgetLayoutInfo) {
        unsafe {
            user32::SendMessageW(
                self.hwnd(),
                DM_SETLAYOUTINFO,
                &wli as *const WidgetLayoutInfo as WPARAM,
                0
            );
        }
    }

    /// Create a new toplevel window. This is unsafe because it must be called on the correct thread in
    /// order to have the win32 message pump get the messages for this window.
    pub unsafe fn new_toplevel(config: &WindowConfig, callback_data: CallbackData) -> NativeResult<WindowNode> {
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
            BLANK_WINDOW_CLASS.as_ptr(),
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
        assert_ne!(window_handle, ptr::null_mut());

        // Create the toplevel node data node, and initialize the subclass.
        let node_data = Box::new(NodeData::new(window_handle, TOPLEVEL_SUBCLASS, Rc::new(callback_data)));
        node_data.update_callback_ptr_location();
        mem::forget(node_data);

        if window_handle == ptr::null_mut() {
            return Err(NativeError::OsError(format!("{}", io::Error::last_os_error())));
        }

        // Initialize the grid size of the toplevel window to (1, 1)
        let grid_size_update: LayoutUpdate<HWND> = LayoutUpdate::GridSize(GridSize::new(1, 1));
        user32::SendMessageW(
            window_handle,
            DM_QUEUECHILDUPDATES,
            &grid_size_update as *const _ as WPARAM,
            1
        );

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

    pub fn new_layout_group(&self, receiver: &WindowReceiver) -> NativeResult<WindowNode> {
        unsafe {
            user32::PostMessageW(
                self.hwnd(),
                DM_NEWLAYOUTGROUP,
                0, 0
            );
            receiver.recv()
                .expect("Unexpected close of window channel")
        }
    }

    /// Create a new zero-sized text button with no contents.
    pub fn new_text_button(&self, receiver: &WindowReceiver) -> NativeResult<WindowNode> {
        unsafe {
            user32::PostMessageW(
                self.hwnd(),
                DM_NEWTEXTBUTTON,
                0,
                0
            );
            receiver.recv()
                .expect("Unexpected close of window channel")
        }
    }

    pub fn open_update_queue(&self) {
        let hwnd = self.hwnd();

        match unsafe{ user32::SendMessageW(hwnd, DM_OPENUPDATEQUEUE, 0, 0) } {
            -1 => panic!("Attempted to open an already-open update queue"),
             0 => panic!("Attempted to open update queue on window with no queue"),
             1 => (),
             _ => panic!("Invalid return value for DM_OPENUPDATEQUEUE")
        }
    }

    pub fn flush_update_queue(&self) {
        let hwnd = self.hwnd();

        match unsafe{ user32::SendMessageW(hwnd, DM_FLUSHUPDATEQUEUE, 0, 0) } {
            -1 => panic!("Attempted to flush a closed update queue"),
             0 => panic!("Attempted to flush update queue on window with no queue"),
             1 => (),
             _ => panic!("Invalid return value for DM_FLUSHUPDATEQUEUE")
        }
    }
}

pub struct Toplevel( WindowWrapper );
pub struct LayoutGroup( WindowWrapper );

impl LayoutGroup {
    pub fn set_grid_size(&self, grid_size: GridSize) {
        unsafe {
            let update: LayoutUpdate<HWND> = LayoutUpdate::GridSize(grid_size);
            user32::SendMessageW(
                (self.0).0,
                DM_QUEUECHILDUPDATES,
                &update as *const _ as WPARAM,
                1
            );
        }
    }
}

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
            OriginRect::new(ideal_size.cx as u32, ideal_size.cy as u32)
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

    fn get_size_bounds(&self) -> Option<SizeBounds> {
        unsafe {
            let mut mmi = MINMAXINFO {
                ptMinTrackSize: POINT{ x: 0, y: 0 },
                ptMaxTrackSize: POINT{
                    x: user32::GetSystemMetrics(SM_CXMAXTRACK),
                    y: user32::GetSystemMetrics(SM_CYMAXTRACK)
                },
                ..mem::zeroed()
            };

            let result = user32::SendMessageW(
                self.0,
                WM_GETMINMAXINFO,
                0,
                &mut mmi as *mut _ as LPARAM
            );

            if result == 0 {
                Some(SizeBounds {
                    min: OriginRect::new(mmi.ptMinTrackSize.x as u32, mmi.ptMinTrackSize.y as u32),
                    max: OriginRect::new(mmi.ptMaxTrackSize.x as u32, mmi.ptMaxTrackSize.y as u32)
                })
            } else {
                None
            }
        }
    }

    fn set_inner_rect(&self, rect: OffsetRect) {
        unsafe {
            let (wparam, lparam) = offset_rect_encode_wlparams(rect);

            user32::PostMessageW(
                self.0,
                DM_RECT,
                wparam,
                lparam
            );
        }
    }

    fn get_style(&self) -> u32 {
        unsafe{ user32::GetWindowLongW(self.0, -16) as u32 }
    }

    fn get_style_ex(&self) -> u32 {
        unsafe{ user32::GetWindowLongW(self.0, -20) as u32 }
    }

    fn get_parent(&self) -> Option<WindowWrapperRef> {
        let parent_hwnd = unsafe{ user32::GetParent(self.0) };
        if parent_hwnd == ptr::null_mut() {
            None
        } else {
            Some(WindowWrapperRef(parent_hwnd))
        }
    }

    fn update_size_bounds(&self, size_bounds: SizeBounds) {
        unsafe {
            if let Some(parent) = self.get_parent() {
                let size_bounds_update = LayoutUpdate::WidgetSizeBounds(self.0, size_bounds);
                user32::SendMessageW(
                    parent.0,
                    DM_QUEUECHILDUPDATES,
                    &size_bounds_update as *const _ as WPARAM,
                    1
                );
            }

        }
    }
}


fn ucs2_str<'a, S: ?Sized + AsRef<OsStr>>(s: &'a S) -> impl 'a + Iterator<Item=u16> {
    s.as_ref().encode_wide().chain(once(0))
}

lazy_static!{
    static ref BLANK_WINDOW_CLASS: Ucs2String = unsafe{
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
    CloseClicked
}

pub struct CallbackData {
    window_sender: Sender<NativeResult<WindowNode>>,
    event_sender: Sender<RawEvent>,
    update_queue: RefCell<UpdateQueue<HWND>>
}

impl CallbackData {
    pub fn new(window_sender: Sender<NativeResult<WindowNode>>, event_sender: Sender<RawEvent>) -> CallbackData {
        CallbackData {
            window_sender: window_sender,
            event_sender: event_sender,
            update_queue: RefCell::new(UpdateQueue::new())
        }
    }
}

struct NodeData {
    hwnd: HWND,
    subclass: UINT_PTR,
    callback_data: Rc<CallbackData>,
    child_layout: LayoutEngine<NodeVec>
}

impl NodeData {
    fn new(hwnd: HWND, subclass: UINT_PTR, callback_data: Rc<CallbackData>) -> NodeData {
        NodeData {
            hwnd: hwnd,
            subclass: subclass,
            callback_data: callback_data,
            child_layout: LayoutEngine::new(NodeVec::new(
                NodeData::widget_data_ucpl
            ))
        }
    }

    fn widget_data_ucpl(wd: &mut WidgetData<NodeData>) {
        wd.widget.update_callback_ptr_location();
    }

    fn update_callback_ptr_location(&self) {
        let callback: SUBCLASSPROC = match self.subclass {
            LAYOUTGROUP_SUBCLASS => Some(parent_callback),
            BUTTON_SUBCLASS      => Some(pushbutton_callback),
            TOPLEVEL_SUBCLASS    => Some(toplevel_callback),
            _                    => panic!("Invalid subclass")
        };
        unsafe {
            comctl32::SetWindowSubclass(self.hwnd, callback, self.subclass,
                                        self as *const NodeData as DWORD_PTR);
        }
    }
}


const TOPLEVEL_SUBCLASS: UINT_PTR = 0;
const LAYOUTGROUP_SUBCLASS: UINT_PTR = 1;
const BUTTON_SUBCLASS: UINT_PTR = 2;

// A bunch of different derin messages for creating controls and such. These are all handled by the
// toplevel window, as the child controls each have their own callback specified by windows.

const DM_DESTROYWINDOW: UINT = WM_APP + 0;
/// Create a title-less push button.
///
/// # Callback parameters
/// * `wparam`: Parent `HWND` handle
const DM_NEWTEXTBUTTON: UINT = WM_APP + 1;
const DM_NEWLAYOUTGROUP: UINT = WM_APP + 2;

const DM_SETLAYOUTINFO: UINT = WM_APP + 4;
/// Remove the child window specified in `wparam`.
const DM_REMOVECHILD: UINT = WM_APP + 5;
/// Queue up updates for the child window, with a pointer to an slice of `LayoutUpdate` enums in the
/// `wparam` parameter and the length of the slice in the `lparam` parameter.
///
/// # What happens if this message is sent while the queue is closed?
/// After all, there are perfectly valid situations for that to happen; notably, if the user begins
/// rescaling the window. When that happens, the "queue" of one item is simply flushed immediately.
const DM_QUEUECHILDUPDATES: UINT = WM_APP + 6;
/// Open up the update queue, into which child windows can queue layout updates.
///
/// # Return values
/// * `-1`: Error, update queue already open
/// * `0`: Error, no update queue present for window
/// * `1`: Success
const DM_OPENUPDATEQUEUE: UINT = WM_APP + 7;
/// Flush and close the update queue, updating the positions of all of the child windows, if
/// necessary.
///
/// # Return values
/// * `-1`: Error, update queue already closed
/// * `0`: Error, no update queue present for window
/// * `1`: Success
const DM_FLUSHUPDATEQUEUE: UINT = WM_APP + 8;
/// Resize the window rect, with the new rect encoded in the `wparam` and `lparam` parameters.
const DM_RECT: UINT = WM_APP + 9;



unsafe extern "system"
    fn parent_callback(hwnd: HWND, msg: UINT,
                       wparam: WPARAM, lparam: LPARAM,
                       _: UINT_PTR, nd: DWORD_PTR) -> LRESULT
{
    let nd = &mut *(nd as *mut NodeData);
    parent_proc(hwnd, msg, wparam, lparam, nd)
}

unsafe extern "system"
    fn toplevel_callback(hwnd: HWND, msg: UINT,
                         wparam: WPARAM, lparam: LPARAM,
                         _: UINT_PTR, nd: DWORD_PTR) -> LRESULT
{
    let nd = &mut *(nd as *mut NodeData);

    match msg {
        WM_CLOSE => {
            nd.callback_data.event_sender.send(RawEvent::CloseClicked).ok();
            0
        },

        WM_NCDESTROY => {
            Box::from_raw(nd);
            comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
        }

        WM_SIZE => {
            let new_rect = OriginRect::new(loword(lparam) as u32, hiword(lparam) as u32);
            let mut update_queue = nd.callback_data.update_queue.borrow_mut();

            update_queue.push_engine(&nd.child_layout);
            update_queue.push_update(LayoutUpdate::PixelSize(new_rect));
            update_queue.pop_engine(&mut nd.child_layout);
            0
        },

        _ => parent_proc(hwnd, msg, wparam, lparam, nd)
    }
}

unsafe extern "system"
    fn pushbutton_callback(hwnd: HWND, msg: UINT,
                           wparam: WPARAM, lparam: LPARAM,
                           _: UINT_PTR, nd: DWORD_PTR) -> LRESULT
{
    let nd = &mut *(nd as *mut NodeData);
    match msg {
        WM_SETTEXT => {
            let ret = comctl32::DefSubclassProc(hwnd, msg, wparam, lparam);

            let window = WindowWrapperRef(hwnd);

            let size_bounds = SizeBounds {
                min: TextButton::get_ideal_rect_raw(hwnd),
                max: OriginRect::max()
            };

            window.update_size_bounds(size_bounds);

            ret
        }

        _ => common_proc(hwnd, msg, wparam, lparam, nd)
    }
}

/// Handles resizing children and creating children for all parent windows.
unsafe fn parent_proc(hwnd: HWND, msg: UINT,
                      wparam: WPARAM, lparam: LPARAM,
                      nd: &mut NodeData) -> LRESULT
{
    match msg {
        WM_GETMINMAXINFO => {
            let mmi = &mut *(lparam as *mut MINMAXINFO);

            let window = WindowWrapperRef(hwnd);

            let size_bounds = nd.child_layout.actual_size_bounds();

            // The `MINMAXINFO` struct takes sizes that include the window dressings, so we have to
            // expand the rectangles to include the dressings.
            let mut min_rect = RECT {
                right: size_bounds.min.width() as c_int,
                bottom: size_bounds.min.height() as c_int,
                left: 0,
                top: 0
            };
            user32::AdjustWindowRectEx(
                &mut min_rect,
                window.get_style(),
                0,
                window.get_style_ex()
            );

            let mut max_rect = RECT {
                right: cmp::min(size_bounds.max.width(), (c_int::max_value() / 2) as u32) as c_int,
                bottom: cmp::min(size_bounds.max.height(), (c_int::max_value() / 2) as u32) as c_int,
                left: 0,
                top: 0
            };
            user32::AdjustWindowRectEx(
                &mut max_rect,
                window.get_style(),
                0,
                window.get_style_ex()
            );

            mmi.ptMinTrackSize = POINT {
                x: min_rect.right - min_rect.left,
                y: min_rect.bottom - min_rect.top
            };
            mmi.ptMaxTrackSize = POINT {
                x: max_rect.right - max_rect.left,
                y: max_rect.bottom - max_rect.top
            };

            0
        }

        DM_RECT => {
            let old_size_bounds = nd.child_layout.actual_size_bounds();

            {
                let mut update_queue = nd.callback_data.update_queue.borrow_mut();

                let new_size = OriginRect::from(offset_rect_decode_wlparams(wparam, lparam));

                let size_update = LayoutUpdate::PixelSize(new_size);

                update_queue.push_engine(&nd.child_layout);
                update_queue.push_update(size_update);
                update_queue.pop_engine(&mut nd.child_layout);
            }

            if old_size_bounds != nd.child_layout.actual_size_bounds() {
                WindowWrapperRef(hwnd).update_size_bounds(nd.child_layout.actual_size_bounds());
            }

            common_proc(hwnd, msg, wparam, lparam, nd)
        }

        DM_OPENUPDATEQUEUE => {
            let mut update_queue = nd.callback_data.update_queue.borrow_mut();
            if update_queue.engine_is_top(&nd.child_layout) {
                -1
            } else {
                update_queue.push_engine(&nd.child_layout);
                1
            }
        }

        DM_FLUSHUPDATEQUEUE => {
            let old_size_bounds = nd.child_layout.actual_size_bounds();

            let ret = {
                let mut update_queue = nd.callback_data.update_queue.borrow_mut();
                if update_queue.engine_is_top(&nd.child_layout) {
                    update_queue.pop_engine(&mut nd.child_layout);
                    1
                } else {
                    -1
                }
            };

            if old_size_bounds != nd.child_layout.actual_size_bounds() {
                WindowWrapperRef(hwnd).update_size_bounds(nd.child_layout.actual_size_bounds());
            }

            ret
        }

        DM_QUEUECHILDUPDATES => {
            let updates_ptr = wparam as *const LayoutUpdate<HWND>;
            let num_updates = lparam as usize;

            let updates = slice::from_raw_parts(updates_ptr, num_updates);

            let old_size_bounds = nd.child_layout.actual_size_bounds();
            let mut update_size_bounds = false;

            {
                let mut update_queue = nd.callback_data.update_queue.borrow_mut();
                if update_queue.engine_is_top(&nd.child_layout) {
                    for update in updates {
                        update_queue.push_update(*update);
                    }
                } else {
                    update_queue.push_engine(&nd.child_layout);

                    for update in updates {
                        update_queue.push_update(*update);
                    }

                    update_queue.pop_engine(&mut nd.child_layout);

                    update_size_bounds = old_size_bounds != nd.child_layout.actual_size_bounds();
                }
            }

            if update_size_bounds {
                WindowWrapperRef(hwnd).update_size_bounds(nd.child_layout.actual_size_bounds());
            }

            0
        }

        DM_REMOVECHILD => {
            let child_hwnd = wparam as HWND;

            let mut update_queue = nd.callback_data.update_queue.borrow_mut();
            if let Some(child_nd) = update_queue.remove_widget(child_hwnd, &mut nd.child_layout) {
                // We *could* remove the subclass here, but setting the subclass parameters to null causes
                // the program to crash if something's gone wrong, which is preferable to the program slowly
                // self-destructing via memory errors.
                comctl32::SetWindowSubclass(child_hwnd, None, child_nd.subclass, 0);
            }

            0
        }

        DM_NEWTEXTBUTTON => {
            let button_hwnd = user32::CreateWindowExW(
                0,
                BUTTON_CLASS.as_ptr(),
                ptr::null(),
                WS_TABSTOP | WS_VISIBLE | WS_CHILD | WS_CLIPSIBLINGS | BS_PUSHBUTTON,
                0, 0, 0, 0,
                hwnd,
                ptr::null_mut(),
                kernel32::GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );

            let node_data = NodeData::new(button_hwnd, BUTTON_SUBCLASS, nd.callback_data.clone());
            let mut update_queue = nd.callback_data.update_queue.borrow_mut();
            update_queue.insert_widget(button_hwnd, node_data, &mut nd.child_layout);

            nd.callback_data.window_sender.send(
                (button_hwnd != ptr::null_mut()).as_result(
                    WindowNode::TextButton(TextButton {
                        wrapper: WindowWrapper(button_hwnd),
                        text: Ucs2String::new()
                    }),
                    NativeError::OsError(format!("{}", io::Error::last_os_error()))
                )
            ).ok();

            0
        },

        DM_NEWLAYOUTGROUP => {
            let group_hwnd = user32::CreateWindowExW(
                0,
                BLANK_WINDOW_CLASS.as_ptr(),
                ptr::null(),
                WS_VISIBLE | WS_CLIPCHILDREN | WS_CHILD,
                0, 0, 0, 0,
                hwnd,
                ptr::null_mut(),
                kernel32::GetModuleHandleW(ptr::null()),
                ptr::null_mut()
            );

            let node_data = NodeData::new(group_hwnd, LAYOUTGROUP_SUBCLASS, nd.callback_data.clone());
            let mut update_queue = nd.callback_data.update_queue.borrow_mut();
            update_queue.insert_widget(group_hwnd, node_data, &mut nd.child_layout);

            nd.callback_data.window_sender.send(
                (group_hwnd != ptr::null_mut()).as_result(
                    WindowNode::LayoutGroup(LayoutGroup(WindowWrapper(group_hwnd))),
                    NativeError::OsError(format!("{}", io::Error::last_os_error()))
                )
            ).ok();
            0
        }
        _ => common_proc(hwnd, msg, wparam, lparam, nd)
    }
}

unsafe fn common_proc(hwnd: HWND, msg: UINT,
                      wparam: WPARAM, lparam: LPARAM,
                      _: &mut NodeData) -> LRESULT
{
    match msg {
        WM_NCDESTROY => {
            if let Some(parent_hwnd) = WindowWrapperRef(hwnd).get_parent() {
                user32::SendMessageW(
                    parent_hwnd.0,
                    DM_REMOVECHILD,
                    hwnd as WPARAM,
                    0
                );
            }

            comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
        }

        DM_RECT => {
            let window = WindowWrapperRef(hwnd);

            let rect = offset_rect_decode_wlparams(wparam, lparam);

            // The size of the rect with window dressings.
            let mut dressed_rect = RECT {
                left: rect.topleft.x as c_int,
                top: rect.topleft.y as c_int,
                right: rect.lowright.x as c_int,
                bottom: rect.lowright.y as c_int
            };

            user32::AdjustWindowRectEx(
                &mut dressed_rect,
                window.get_style(),
                0,
                window.get_style_ex()
            );

            user32::SetWindowPos(
                hwnd,
                ptr::null_mut(),
                dressed_rect.left,
                dressed_rect.top,
                dressed_rect.right - dressed_rect.left,
                dressed_rect.bottom - dressed_rect.top,
                SWP_NOACTIVATE | SWP_NOZORDER
            );

            0
        }

        DM_SETLAYOUTINFO => {
            let layout_info = *(wparam as *const WidgetLayoutInfo);

            if let Some(parent_hwnd) = WindowWrapperRef(hwnd).get_parent() {
                let update = LayoutUpdate::WidgetLayoutInfo(hwnd, layout_info);

                user32::SendMessageW(
                    parent_hwnd.0,
                    DM_QUEUECHILDUPDATES,
                    &update as *const _ as WPARAM,
                    1
                );
            }

            0
        }

        _ => comctl32::DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}

type NodeVec = MCVec<WidgetData<NodeData>, fn(&mut WidgetData<NodeData>)>;

impl NodeVec {
    fn binary_search_hwnd(&self, hwnd: HWND) -> Result<usize, usize> {
        self.binary_search_by(|probe| (probe.widget.hwnd as usize).cmp(&(hwnd as usize)))
    }
}

impl Container for NodeVec {
    type Widget = NodeData;
    type Key = HWND;

    fn get_widget(&self, key: HWND) -> Option<&WidgetData<NodeData>> {
        self.binary_search_hwnd(key).ok().map(|index| unsafe{ self.get_unchecked(index) })
    }

    fn get_widget_mut(&mut self, key: HWND) -> Option<&mut WidgetData<NodeData>> {
        match self.binary_search_hwnd(key) {
            Ok(index) => unsafe{ Some(self.get_unchecked_mut(index)) },
            Err(_)     => None
        }
    }

    fn insert_widget(&mut self, key: HWND, widget: NodeData) -> Option<NodeData> {
        match self.binary_search_hwnd(key) {
            // If the key already exists in the vector, swap in the new widget and return the old
            // widget.
            Ok(index)  => Some(self.replace(index, WidgetData::new(widget)).widget),
            Err(index) => {self.insert(index, WidgetData::new(widget)); None}
        }
    }

    fn remove_widget(&mut self, key: HWND) -> Option<NodeData> {
        match self.binary_search_hwnd(key) {
            Ok(index) => Some(self.remove(index).widget),
            Err(_) => None
        }
    }

    fn get_widget_iter(&self) -> slice::Iter<WidgetData<NodeData>> {
        self.iter()
    }

    fn get_widget_iter_mut(&mut self) -> slice::IterMut<WidgetData<NodeData>> {
        self.iter_mut()
    }
}

impl<'a> ContainerRef<'a> for &'a NodeVec {
    type Widget = NodeData;
    type WDIter = slice::Iter<'a, WidgetData<NodeData>>;
    type WDIterMut = slice::IterMut<'a, WidgetData<NodeData>>;
}

impl Widget for NodeData {
    fn set_rect(&mut self, rect: OffsetRect) {
        WindowWrapper(self.hwnd).set_inner_rect(rect);
    }
}

fn offset_rect_encode_wlparams(rect: OffsetRect) -> (WPARAM, LPARAM) {
    (rect.topleft.x as u64 | (rect.topleft.y as u64) << 32,
     unsafe{ mem::transmute(rect.lowright.x as u64 | (rect.lowright.y as u64) << 32) })
}

fn offset_rect_decode_wlparams(wparam: WPARAM, lparam: LPARAM) -> OffsetRect {
    let lparam = unsafe{ mem::transmute::<LPARAM, u64>(lparam) };
    OffsetRect::new(
        wparam as u32,
        (wparam >> 32) as u32,
        lparam as u32,
        (lparam >> 32) as u32
    )
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

#[cfg(test)]
mod tests {
    use dle::geometry::OffsetRect;

    quickcheck!{
        fn offset_rect_encode(top: u32, left: u32, right: u32, bottom: u32) -> bool {
            let rect = OffsetRect::new(top, left, right, bottom);

            let (wparam, lparam) = super::offset_rect_encode_wlparams(rect);
            let decoded_rect = super::offset_rect_decode_wlparams(wparam, lparam);
            println!("wparam: {}, lparam: {}\nraw: {:?}\ndecoded: {:?}\n", wparam, lparam, rect, decoded_rect);
            rect == decoded_rect
        }
    }
}
