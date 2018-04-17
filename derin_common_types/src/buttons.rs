// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// A button on the mouse.
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MouseButton {
    Left = MOUSE_L,
    Right = MOUSE_R,
    Middle = MOUSE_M,
    X1 = MOUSE_X1,
    X2 = MOUSE_X2
}

const MOUSE_L: u8  = 0b001;
const MOUSE_R: u8  = 0b010;
const MOUSE_M: u8  = 0b011;
const MOUSE_X1: u8 = 0b100;
const MOUSE_X2: u8 = 0b101;

#[doc(hidden)]
pub const MOUSE_INT_MASK: u16 = 0b111;
#[doc(hidden)]
pub const MOUSE_INT_MASK_LEN: u16 = 3;
#[doc(hidden)]
pub const NUM_MOUSE_BUTTONS: usize = 5;

bitflags!{
    /// A set of flags that contains the state of the keyboard's modifier keys.
    pub struct ModifierKeys: u8 {
        /// The Shift key.
        const SHIFT = 1 << 0;
        /// The Control key.
        const CTRL  = 1 << 1;
        /// The Alt Key.
        const ALT   = 1 << 2;
        /// On Windows and Linux, the key between Control and Alt. On OSX, the key between Control
        /// and Command.
        const LOGO  = 1 << 3;
    }
}

/// A key on the keyboard.
#[repr(u8)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// Backspace
    Back,
    Tab,
    Clear,
    Enter,
    Pause,
    Escape,
    Space,
    PageUp,
    PageDown,
    End,
    Home,
    Select,
    Print,
    Execute,
    PrntScr,
    Insert,
    Delete,
    Help,

    /// The `0` key above the alphabetic keys.
    Alpha0,
    /// The `1` key above the alphabetic keys.
    Alpha1,
    /// The `2` key above the alphabetic keys.
    Alpha2,
    /// The `3` key above the alphabetic keys.
    Alpha3,
    /// The `4` key above the alphabetic keys.
    Alpha4,
    /// The `5` key above the alphabetic keys.
    Alpha5,
    /// The `6` key above the alphabetic keys.
    Alpha6,
    /// The `7` key above the alphabetic keys.
    Alpha7,
    /// The `8` key above the alphabetic keys.
    Alpha8,
    /// The `9` key above the alphabetic keys.
    Alpha9,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// ';:' on US standard keyboards, though it may not be this on other layouts
    Semicolon,
    Equals,
    Comma,
    Minus,
    /// The period key
    Period,
    /// '/?' on US standard keyboards, though it may not be this on other layouts
    Slash,
    /// '`~' on US standard keyboards, though it may not be this on other layouts
    Accent,

    /// '[{' on US standard keyboards, though it may not be this on other layouts
    LBracket,
    /// ']}' on US standard keyboards, though it may not be this on other layouts
    RBracket,
    /// '\|' on US standard keyboards, though it may not be this on other layouts
    Backslash,
    /// `"'` on US standard keyboards, though it may not be this on other layouts
    Apostrophe,

    Sleep,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    NumStar,
    NumPlus,
    NumSub,
    NumDot,
    NumSlash,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    NumLock,
    CapsLock,
    ScrollLock,

    LShift,
    RShift,
    LCtrl,
    RCtrl,
    LAlt,
    RAlt,

    /// Browser back key
    BrowserBack,
    /// Browser forward key
    BrowserFwd,
    /// Browser refresh key
    BrowserRef,
    /// Browser stop key
    BrowserStop,
    /// Browser search key
    BrowserSearch,
    /// Browser favorites key
    BrowserFav,
    /// Browser start/home key
    BrowserHome,

    /// Next track key
    MediaNextTrack,
    /// Previous track key
    MediaPrevTrack, // B)
    /// Stop media key
    MediaStop,
    /// Play/pause media key
    MediaPause,

    /// Left arrow key
    LArrow,
    /// Up arrow key
    UArrow,
    /// Right arrow key
    RArrow,
    /// Down arrow key
    DArrow,

    // IME keys
    Kana,
    Junja,
    Final,
    Kanji,
    Convert,
    Nonconvert,
    Accept,
    ModeChange,
    Process,

    // Come back to these
    Shift,
    Control,
    Menu
}

impl From<MouseButton> for u8 {
    #[inline]
    fn from(button: MouseButton) -> u8 {
        use std::mem;

        unsafe{ mem::transmute(button) }
    }
}

impl MouseButton {
    #[inline]
    pub fn from_u8(u: u8) -> Option<MouseButton> {
        use self::MouseButton::*;
        match u {
            MOUSE_L  => Some(Left),
            MOUSE_R  => Some(Right),
            MOUSE_M  => Some(Middle),
            MOUSE_X1 => Some(X1),
            MOUSE_X2 => Some(X2),
            _        => None
        }
    }
}
