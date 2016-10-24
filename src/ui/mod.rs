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

/// The general data processor through which the UI-crawling state machine sends data.
struct DataProcessor {}
impl DataProcessor {
    fn push_node(&mut self, _: NodeLink) {unimplemented!()}
    fn push_image_node(&mut self, _: NodeLink, _: &Image) {unimplemented!()}
    fn push_text_node(&mut self, _: NodeLink, _: &Text) {unimplemented!()}
    fn call_control<C: Control>(&mut self, _: &mut C) {unimplemented!()}
}

pub struct Image {}
pub struct Text {}

#[derive(Debug, Clone, Copy)]
struct NodeData<'a> {
    name: &'a str,
    ty: Option<&'a str>,
    state: Option<&'a str>,
}

impl<'a> NodeData<'a> {
    fn new(name: &'a str) -> NodeData<'a> {
        NodeData {
            name: name,
            ty: None,
            state: None
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct NodeLink<'l, 'a: 'l> {
    last_node: Option<&'l NodeLink<'l, 'a>>,
    node: NodeData<'a>
}


// These three structs represent the state machine used to crawl a UI tree. Really, they mainly
// exist as an abstraction layer over the `DataProcessor` struct, where all the real processing
// happens. They're also zero-cost, which is nice.

pub struct NodeProcessor<'a> {
    link: NodeLink<'a, 'static>,
    proccer: &'a mut DataProcessor
}

pub struct ChildProcessor<'a> {
    link: NodeLink<'a, 'static>,
    proccer: &'a mut DataProcessor
}

pub struct ContentProcessor<'a> {
    link: NodeLink<'a, 'static>,
    proccer: &'a mut DataProcessor
}


impl<'a> NodeProcessor<'a> {
    pub fn node_type(&mut self, node_type: &'static str) {
        self.link.node.ty = Some(node_type);
    }

    pub fn node_state(&mut self, node_state: &'static str) {
        self.link.node.state = Some(node_state);
    }


    pub fn node_control<C: Control>(&mut self, control: &mut C) {
        self.proccer.call_control(control);
    }

    pub fn children(self) -> ChildProcessor<'a> {
        ChildProcessor {
            link: self.link,
            proccer: self.proccer
        }
    }

    pub fn contents(self) -> ContentProcessor<'a> {
        ContentProcessor {
            link: self.link,
            proccer: self.proccer
        }
    }
}

impl<'a> Drop for NodeProcessor<'a> {
    fn drop(&mut self) {
        self.proccer.push_node(self.link)
    }
}

impl<'a> ChildProcessor<'a> {
    pub fn take(&mut self, name: &'static str) -> NodeProcessor {
        NodeProcessor {
            link: NodeLink {
                last_node: Some(&self.link),
                node: NodeData::new(name)
            },
            proccer: self.proccer
        }
    }
}

impl<'a> ContentProcessor<'a> {
    pub fn push_image(self, image: &Image) {
        self.proccer.push_image_node(self.link, image);
    }

    pub fn push_text(self, text: &Text) {
        self.proccer.push_text_node(self.link, text);
    }
}

pub trait Node {
    fn node_data(&mut self, NodeProcessor);
}

pub trait Control {
    type Action;

    fn on_mouse_event(&mut self, MouseEvent) -> Option<Self::Action> {None}
}
