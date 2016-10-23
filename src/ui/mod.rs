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

pub struct Text {}
pub struct Image {}

pub struct NodeDataCollector {}
pub struct ChildDataCollector {}
pub struct ContentCollector {}

impl NodeDataCollector {
    pub fn node_type(&mut self, _: &str) {unimplemented!()}
    pub fn node_state(&mut self, _: &str) {unimplemented!()}
    pub fn node_class(&mut self, _: &str) {unimplemented!()}

    pub fn node_control<C: Control>(&mut self, _: &mut C) {unimplemented!()}

    pub fn children(self) -> ChildDataCollector {unimplemented!()}
    pub fn contents(self) -> ContentCollector {unimplemented!()}
}

impl ChildDataCollector {
    pub fn take(&mut self, _: &str) -> NodeDataCollector {unimplemented!()}
}

impl ContentCollector {
    pub fn push_image(&mut self, _: Image) {unimplemented!()}
    pub fn push_text(&mut self, _: Text) {unimplemented!()}
}

pub trait Node {
    fn node_data(&mut self, NodeDataCollector);
}

pub trait Control {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Option<Self::Action> {None}
}
