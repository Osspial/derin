pub enum MouseEvent {
    Clicked(MouseButton),
    DoubleClicked(MouseButton),
    Scroll(f32, f32)
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}
