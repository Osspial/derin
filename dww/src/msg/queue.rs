use winapi::*;
use user32;

use window::refs::WindowRefMut;

use std::marker::PhantomData;
use std::io::{Result, Error};

pub struct QueueMsg( MSG );

impl QueueMsg {
    #[inline]
    pub fn window(&mut self) -> WindowRefMut {
        unsafe{ WindowRefMut::from_raw(self.0.hwnd) }
    }

    #[inline]
    pub fn message_id(&self) -> u32 {
        self.0.message as u32
    }

    #[inline]
    pub fn repost(self) {
        unsafe{ user32::PostMessageW(
            self.0.hwnd,
            self.0.message,
            self.0.wParam,
            self.0.lParam
        ) };
    }

    /// Dispatch the message to the specified window. Unsafe, because it cannot guarantee that the
    /// window isn't being borrowed somewhere else in the code.
    #[inline]
    pub unsafe fn dispatch(self) {
        user32::DispatchMessageW(&self.0);
    }
}

pub fn thread_wait_queue() -> WaitMessageQueue {
    WaitMessageQueue(PhantomData)
}

pub struct WaitMessageQueue(PhantomData<*const ()>); // PhantomData used to opt out of send and sync

impl Iterator for WaitMessageQueue {
    type Item = Result<QueueMsg>;

    fn next(&mut self) -> Option<Result<QueueMsg>> {
        use std::{mem, ptr};

        unsafe {
            let mut msg = mem::uninitialized();
            let result = user32::GetMessageW(&mut msg, ptr::null_mut(), 0, 0);

            if 1 <= result {
                user32::TranslateMessage(&msg);
                Some(Ok(QueueMsg(msg)))
            } else if 0 == result {
                None
            } else {
                Some(Err(Error::last_os_error()))
            }
        }
    }
}
