#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CursorIcon {
    Pointer,
    Wait,
    Crosshair,
    Hand,
    NotAllowed,
    Text,
    Move,
    SizeNS,
    SizeWE,
    SizeNeSw,
    SizeNwSe,
    SizeAll,
    Hide
}

impl Default for CursorIcon {
    #[inline]
    fn default() -> CursorIcon {
        CursorIcon::Pointer
    }
}
