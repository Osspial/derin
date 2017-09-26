#[repr(u8)]
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
pub const NUM_MOUSE_BUTTONS: usize = 5;

#[repr(u8)]
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
    PgUp,
    PgDn,
    End,
    Home,
    Select,
    Print,
    Execute,
    PrntScr,
    Insert,
    Delete,
    Help,

    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,

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
    Semi,
    Plus,
    Comma,
    Minus,
    /// The period key
    Dot,
    /// '/?' on US standard keyboards, though it may not be this on other layouts
    Slash,
    /// '`~' on US standard keyboards, though it may not be this on other layouts
    Tilde,

    /// '[{' on US standard keyboards, though it may not be this on other layouts
    LBrac,
    /// ']}' on US standard keyboards, though it may not be this on other layouts
    RBrac,
    /// '\|' on US standard keyboards, though it may not be this on other layouts
    Pipe,
    /// `"'` on US standard keyboards, though it may not be this on other layouts
    Quote,

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

    Num,
    Caps,
    Scroll,

    LShift,
    RShift,
    LCtrl,
    RCtrl,
    LAlt,
    RAlt,

    /// Browser back key
    BBack,
    /// Browser forward key
    BFwd,
    /// Browser refresh key
    BRef,
    /// Browser stop key
    BStop,
    /// Browser search key
    BSearch,
    /// Browser favorites key
    BFav,
    /// Browser start/home key
    BHome,

    /// Next track key
    MNTrack,
    /// Previous track key
    MPTrack, // B)
    /// Stop media key
    MStop,
    /// Play/pause media key
    MPause,

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
