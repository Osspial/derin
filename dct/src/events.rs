pub enum MouseEvent {
    Clicked(MouseButton),
    DoubleClicked(MouseButton)
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}
