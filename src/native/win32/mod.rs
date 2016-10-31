pub mod wrapper;
use self::wrapper::{HwndType, WindowWrapper};

use user32;

use std::ptr;
use std::mem;
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use native::NativeResult;
use native::WindowConfig;

/// An internal duplicate of WindowType that holds the internal window
#[derive(Clone)]
pub enum WindowType<'p> {
    Owned(&'p Window<'p>),
    Child(&'p Window<'p>),
    Top
}

pub struct Event {}

pub struct Window<'p> {
    pub wrapper: WindowWrapper,
    window_receiver: Receiver<NativeResult<WindowWrapper>>,
    win_type: WindowType<'p>,
    /// Used when setting the pixel format on context creation
    config: WindowConfig,
}

impl<'p> Window<'p> {
    #[inline]
    pub fn new(config: WindowConfig) -> NativeResult<Window<'p>> {
        // Channel for the handle to the window
        let (tx, rx) = mpsc::channel();
        let config = Arc::new(config);

        let config_arc = config.clone();
        thread::spawn(move || {
            unsafe {
                let wrapper_window = WindowWrapper::new(&config_arc, HwndType::Top);
                mem::drop(config_arc);

                match wrapper_window {
                    Ok(wr) => {
                        tx.send(Ok(wr)).unwrap();
                    }

                    Err(e) => {
                        tx.send(Err(e)).unwrap();
                        panic!("Window creation error: see sent result for details");
                    }
                }
                

                let mut msg = mem::uninitialized();

                while user32::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                    user32::TranslateMessage(&msg);
                    user32::DispatchMessageW(&msg);
                }
            }
        });

        let wrapper_window = try!(rx.recv().unwrap());

        Ok(
            Window {
                wrapper: wrapper_window,
                window_receiver: rx,
                win_type: WindowType::Top,
                config: Arc::try_unwrap(config).unwrap()
            }
        )
    }

    #[inline]
    pub fn get_type(&self) -> WindowType {
        self.win_type.clone()
    }

    /// Get a non-blocking iterator over the window's events
    #[inline]
    pub fn poll_events(&self) -> PollEventsIter {
        PollEventsIter {
        }
    }

    /// Get a blocking iterator over the window's events
    #[inline]
    pub fn wait_events(&self) -> WaitEventsIter {
        WaitEventsIter {
        }
    }

    #[inline]
    pub fn get_config(&self) -> &WindowConfig {
        &self.config
    }
}

pub struct PollEventsIter {}

pub struct WaitEventsIter {}
