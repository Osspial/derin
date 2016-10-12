pub enum MouseEvent {
    Click(MouseButton),
    Scroll(f32, f32)
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8)
}

pub type Selector<'a> = &'a str;

pub trait Control {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Self::Action;
    fn selector(&self) -> Selector;
}
