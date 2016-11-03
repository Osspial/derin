pub mod wrapper;
use self::wrapper::{HwndType, WindowWrapper};

use user32;

use std::ptr;
use std::mem;
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use boolinator::Boolinator;

use native::NativeResult;
use native::WindowConfig;

use ui::{Node, NodeProcessor, ParentNode};
use ui::intrinsics::TextButton;

pub struct Window<N: Node> {
    root: N,
    node_tree_root: NodeTreeBranch,

    wrapper: WindowWrapper,
    window_receiver: Receiver<NativeResult<WindowWrapper>>
}

impl<N: Node> Window<N> {
    pub fn new(root: N, config: WindowConfig) -> NativeResult<Window<N>> {
        // Channel for the handle to the window
        let (tx, rx) = mpsc::channel();
        let config = Arc::new(config);

        let config_arc = config.clone();
        thread::spawn(move || {
            unsafe {
                let wrapper_window = WindowWrapper::new(&config_arc, HwndType::Top);
                mem::drop(config_arc);

                match wrapper_window {
                    Ok(wr) => {
                        tx.send(Ok(wr)).unwrap();
                    }

                    Err(e) => {
                        tx.send(Err(e)).unwrap();
                        panic!("Window creation error: see sent result for details");
                    }
                }
                

                // Win32 message loop
                let mut msg = mem::uninitialized();
                while user32::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                    user32::TranslateMessage(&msg);
                    user32::DispatchMessageW(&msg);
                }
            }
        });

        let wrapper_window = try!(rx.recv().unwrap());

        Ok(
            Window {
                root: root,
                node_tree_root: NodeTreeBranch::default(),

                wrapper: wrapper_window,
                window_receiver: rx
            }
        )
    }

    pub fn process(&mut self) {
        NodeTraverser::new(&mut self.node_tree_root).add_child("root", &mut self.root);
    }
}

/// A node in the tree that represents the nodes of the UI tree!
#[derive(Default)]
struct NodeTreeBranch {
    state_id: u64,
    name: &'static str,
    children: Vec<NodeTreeBranch>
}

struct NodeTraverser<'a> {
    node_branch: &'a mut NodeTreeBranch,
    /// The index in the child vector to first look at when searching for a child. As new
    /// children get added, this gets incremented.
    child_index: usize
}

impl<'a> NodeTraverser<'a> {
    fn new(node_branch: &'a mut NodeTreeBranch) -> NodeTraverser<'a> {
        NodeTraverser {
            node_branch: node_branch,
            child_index: 0
        }
    }

    fn process_node<N, F>(&'a mut self, name: &'static str, node: &mut N, mut f: F)
            where N: Node,
                  F: FnMut(&'a mut NodeTreeBranch, &mut N)
    {
        if let Some(i) = self.node_branch.children.get(self.child_index)
                             .and_then(|branch| (branch.name == name).as_some(self.child_index))
                             .or(self.node_branch.children
                                    .iter().enumerate()
                                    .filter_map(|(i, branch)| (branch.name == name).as_some(i))
                                    .next()) {
            let branch = &mut self.node_branch.children[i];

            self.child_index += 1;
            let new_state_id = node.state_id();
            if branch.state_id != new_state_id {
                branch.state_id = new_state_id;
                f(branch, node);                
            }
        }
    }
}

impl<'a, N: Node> NodeProcessor<'a, N> for NodeTraverser<'a> {
    default fn add_child(&'a mut self, name: &'static str, node: &mut N) {
        // We have no information about what's in the child node, so we can't really do anything.
        // It still needs to get added to the tree though.
        self.process_node(name, node, |_, _| ());
    }
}

impl<'a, N> NodeProcessor<'a, N> for NodeTraverser<'a> 
        where N: ParentNode<NodeTraverser<'a>> {
    default fn add_child(&'a mut self, name: &'static str, node: &mut N) {
        self.process_node(name, node,
            |branch, node| node.children(NodeTraverser::new(branch)));
    }
}
