#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Clicked(MouseButton),
    DoubleClicked(MouseButton)
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}
