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


#[derive(Debug, Clone, Copy)]
pub struct NodeMeta<'a> {
    pub typ: &'a str,
    pub id: Option<&'a str>,
    pub class: Option<&'a str>
}

pub struct Selector {}

pub trait Node {
    fn metadata(&self) -> NodeMeta;
}

pub trait Control: Node {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Self::Action;
}

pub trait Container: Node {
    type Action;

    fn get_control(&self, Selector) -> Option<&Control<Action = Self::Action>>;
}
