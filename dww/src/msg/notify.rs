use dct::buttons::Key;

use msg::RepeatedPress;
use window::refs::WindowRef;

#[derive(Debug)]
pub struct Notification {
    pub source: WindowRef,
    pub notify_type: NotifyType
}

#[derive(Debug)]
pub enum NotifyType {
    Char(char),
    FontChanged,
    Hover,
    KeyDown(Key, RepeatedPress),
    KillFocus,
    LDown,
    OutOfMemory,
    ReleasedCapture,
    Return,
    SetFocus,
    ThemeChanged,
    TooltipCreated(WindowRef),
    TrackbarThumbPosChanging(u32, ThumbReason)
}

#[derive(Debug)]
pub enum ThumbReason {
    LineDown,
    LineUp,
    PageDown,
    PageUp,
    EndTrack,
    ThumbPosition,
    ThumbTrack,
    Bottom,
    Top
}
