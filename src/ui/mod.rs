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
    pub ty: &'a str,
    pub ident: Option<&'a str>,
    pub state: Option<&'a str>,
    pub class: Option<&'a str>
}

pub trait TreeCrawler {
    fn add_node<N: Node>(&mut self, N);
    fn add_control<C: Control>(&mut self, C);
}

pub trait Node {
    fn metadata(&self) -> NodeMeta;
    fn crawl_children<T: TreeCrawler>(&self, T);
}

pub trait Control: Node {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Self::Action;
}
