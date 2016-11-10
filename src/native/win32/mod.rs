pub mod wrapper;
use self::wrapper::{WindowNode, Toplevel, CallbackData, CALLBACK_DATA};

use user32;

use std::ptr;
use std::mem;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use boolinator::Boolinator;

use native::{NativeResult, NativeError};
use native::WindowConfig;

use ui::{Node, NodeProcessor, ParentNode};
use ui::intrinsics::TextButton;

pub struct Window<N: Node> {
    root: N,
    node_tree_root: NodeTreeBranch,

    window_receiver: Receiver<NativeResult<WindowNode>>
}

impl<N: Node> Window<N> {
    pub fn new(root: N, config: WindowConfig) -> NativeResult<Window<N>> {
        // Channel for the handle to the window
        let (window_sender, window_receiver) = mpsc::channel();
        unsafe{ self::wrapper::enable_visual_styles() };

        // Spawn a child thread in which UI windows are created. Messages are sent to this thread via the
        // `win32` `SendMessageW` function, and window handles and actions are sent back to the main thread
        // via Rust's MPSC channels.
        thread::spawn(move || {
            unsafe {
                // Create a wrapper toplevel window. If the creation succeeds, send back the window. Otherwise, send
                // back the error it created and terminate this thread.
                let wrapper_window = Toplevel::new(&config);
                match wrapper_window {
                    Ok(wr) => {
                        window_sender.send(Ok(WindowNode::Toplevel(wr))).unwrap();
                    }

                    Err(e) => {
                        window_sender.send(Err(e)).unwrap();
                        panic!("Window creation error: see sent result for details");
                    }
                }
                
                // Populate the thread-local callback data with the data needed for the `win32` callback.
                CALLBACK_DATA.with(|cd| {
                    let mut cd = cd.borrow_mut();
                    *cd = Some(CallbackData {
                        window_sender: window_sender
                    });
                });

                // Win32 message loop
                let mut msg = mem::uninitialized();
                while user32::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                    user32::TranslateMessage(&msg);
                    user32::DispatchMessageW(&msg);
                }
            }
        });

        // Receive the top-level window from the child thread. We know that we're going to get SOMEthing at
        // the very least, so we can unwrap the `recv()` call. However, we might receive an error in which
        // case that gets propagated.
        let wrapper_window = window_receiver.recv().unwrap()?;

        Ok(
            Window {
                root: root,
                // The node that contains the top-level node and all other nodes, including root.
                node_tree_root: NodeTreeBranch {
                    state_id: 0,
                    name: "toplevel",
                    window: Some(wrapper_window),
                    children: Vec::with_capacity(1)
                },

                window_receiver: window_receiver
            }
        )
    }

    pub fn process(&mut self) -> NativeResult<()> {
        NodeTraverser {
            node_branch: &mut self.node_tree_root,
            parent_window: None,
            receiver: &self.window_receiver,
            child_index: 0
        }.add_child("root", &mut self.root)
    }
}

/// A node in the tree that represents the nodes of the UI tree!
struct NodeTreeBranch {
    state_id: u64,
    name: &'static str,
    window: Option<WindowNode>,
    children: Vec<NodeTreeBranch>
}

/// Trait for converting `Node`s into `NodeTreeBranch`es.
trait IntoNTB: Node {
    fn into_ntb(&self, name: &'static str, parent: &WindowNode, receiver: &Receiver<NativeResult<WindowNode>>) -> NativeResult<NodeTreeBranch>;
}

impl<N: Node> IntoNTB for N {
    default fn into_ntb(&self, name: &'static str, _: &WindowNode, _: &Receiver<NativeResult<WindowNode>>) -> NativeResult<NodeTreeBranch> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            name: name,
            window: None,
            children: Vec::new()
        })
    }
}

struct NodeTraverser<'a> {
    /// The branch that this instance of NodeTraverser is currently processing
    node_branch: &'a mut NodeTreeBranch,
    /// The `WindowNode` belonging to the parent `NodeTreeBranch` of this `NodeTraverser`. In the event
    /// that this `NodeTraverser` represents the root node, this is `None`.
    parent_window: Option<&'a WindowNode>,
    receiver: &'a Receiver<NativeResult<WindowNode>>,
    /// The index in the child vector to first look at when searching for a child. As new
    /// children get added, this gets incremented.
    child_index: usize
}

impl<'a> NodeTraverser<'a> {
    fn take(&'a mut self, child_index: usize) -> NodeTraverser<'a> {
        NodeTraverser {
            node_branch: &mut self.node_branch.children[child_index],
            parent_window: self.node_branch.window.as_ref(),
            receiver: self.receiver,
            child_index: 0
        }
    }

    /// Take a name, a node, and a function to run upon updating the node and either add the node to the
    /// node tree (which runs the function as well) or determine whether or not to run the update function
    /// and run it if necessary.
    fn process_node_child<N, PF>(&'a mut self, name: &'static str, node: &mut N, mut proc_func: PF)
            -> NativeResult<()>
            where N: Node,
                  PF: FnMut(&mut N, NodeTraverser<'a>) -> NativeResult<()>
    {
        // If the desired node branch is at the current child index, get that and run the contents of the
        // `if` statement. Otherwise, search the entire children vector for the desired node and run the
        // `if` statement if that's found. If both of those fail, insert a new node branch and run the
        // necessary processing function.
        if let Some(i) = self.node_branch.children.get(self.child_index)
                             .and_then(|branch| (branch.name == name).as_some(self.child_index))
                             .or(self.node_branch.children
                                    .iter().enumerate()
                                    .filter_map(|(i, branch)| (branch.name == name).as_some(i))
                                    .next()) {
            self.child_index = i + 1;

            // Compare the newly-generated state id and the cached state id. If there is a mismatch, update the
            // cached id and run the processing function.
            let new_state_id = node.state_id();
            if self.node_branch.children[i].state_id != new_state_id {
                self.node_branch.children[i].state_id = new_state_id;
                proc_func(node, self.take(i))
            } else {
                Ok(())
            }
        } else {
            self.node_branch.children.insert(
                self.child_index, 
                node.into_ntb(
                    name,
                    self.parent_window.or(self.node_branch.window.as_ref()).expect("Attempted to create child window without parent"),
                    self.receiver
                )?
            );
            let child_index = self.child_index;
            self.child_index += 1;
            
            proc_func(node, self.take(child_index))
        }
    }
}

impl<'a, N: Node> NodeProcessor<'a, N> for NodeTraverser<'a> {
    type Error = NativeError;

    default fn add_child(&'a mut self, name: &'static str, node: &mut N) -> NativeResult<()> {
        // We have no information about what's in the child node, so we can't really do anything.
        // It still needs to get added to the tree though.
        self.process_node_child(name, node, |_, _| Ok(()))
    }
}


impl<'a, N> NodeProcessor<'a, N> for NodeTraverser<'a> 
        where N: ParentNode<NodeTraverser<'a>, NativeError> {
    default fn add_child(&'a mut self, name: &'static str, node: &mut N) -> NativeResult<()> {
        self.process_node_child(name, node,
            |node, traverser| node.children(traverser))
    }
}

impl<'a, S: AsRef<str>> NodeProcessor<'a, TextButton<S>> for NodeTraverser<'a> {
    fn add_child(&'a mut self, name: &'static str, node: &mut TextButton<S>) -> NativeResult<()> {
        self.process_node_child(name, node, |_, _| Ok(()))
    }
}

impl<'a, S: AsRef<str>> IntoNTB for TextButton<S> {
    fn into_ntb(&self, name: &'static str, parent: &WindowNode, receiver: &Receiver<NativeResult<WindowNode>>) -> NativeResult<NodeTreeBranch> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            name: name,
            window: Some(parent.new_text_button(self.as_ref(), receiver)?),
            children: Vec::new()
        })
    }
}
