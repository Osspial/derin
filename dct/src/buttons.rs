#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
