mod wrapper;

use self::wrapper::{WindowNode, CallbackData, RawEvent};

use user32;

use std::ptr;
use std::mem;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use boolinator::Boolinator;

use native::{NativeResult, NativeError};
use native::WindowConfig;

use ui::{Node, NodeProcessor, NodeProcessorAT, ParentNode};
use ui::intrinsics::TextButton;
use ui::layout::{GridLayout, EmptyNodeLayout, SingleNodeLayout};


type WindowReceiver = Receiver<NativeResult<WindowNode>>;

pub struct Window<N: Node> {
    root: N,
    node_tree_root: NodeTreeBranch,

    window_receiver: WindowReceiver,
    event_receiver: Receiver<RawEvent>
}

impl<N: Node> Window<N> {
    pub fn new(root: N, config: WindowConfig) -> NativeResult<Window<N>> {
        // Channel for the handle to the window
        let (window_sender, window_receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();

        unsafe{ self::wrapper::enable_visual_styles() };

        // Spawn a child thread in which UI windows are created. Messages are sent to this thread via the
        // `win32` `SendMessageW` function, and window handles and actions are sent back to the main thread
        // via Rust's MPSC channels.
        thread::spawn(move || {
            unsafe {
                let cd = CallbackData::new(window_sender.clone(), event_sender);

                // Create a wrapper toplevel window. If the creation succeeds, send back the window. Otherwise, send
                // back the error it created and terminate this thread.
                let wrapper_window = WindowNode::new_toplevel(&config, cd);
                match wrapper_window {
                    Ok(wr) => {
                        window_sender.send(Ok(wr)).unwrap();
                    }

                    Err(e) => {
                        window_sender.send(Err(e)).unwrap();
                        panic!("Window creation error; see sent result for details");
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

                window_receiver: window_receiver,
                event_receiver: event_receiver
            }
        )
    }

    pub fn process(&mut self) -> NativeResult<()> {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                _ => ()
            }
        }

        if self.node_tree_root.window.is_some() {
            NodeTraverser {
                node_branch: &mut self.node_tree_root,
                receiver: &self.window_receiver,
                child_index: 0,

                children_layout: SingleNodeLayout::new(),
                queue_opened: false
            }.add_child("root", &mut self.root)
        } else {
            Ok(())
        }
    }
}

/// A node in the tree that represents the nodes of the UI tree!
struct NodeTreeBranch {
    state_id: u16,
    name: &'static str,
    window: Option<WindowNode>,
    children: Vec<NodeTreeBranch>
}

/// Trait for converting `Node`s into `NodeTreeBranch`es.
trait IntoNTB: Node {
    fn into_ntb(&self, name: &'static str, parent: &WindowNode, receiver: &WindowReceiver) -> NativeResult<NodeTreeBranch>;
}

impl<N: Node> IntoNTB for N {
    default fn into_ntb(&self, name: &'static str, _: &WindowNode, _: &WindowReceiver) -> NativeResult<NodeTreeBranch> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            name: name,
            window: None,
            children: Vec::new()
        })
    }
}

struct NodeTraverser<'a, L: GridLayout> {
    /// The branch that this instance of NodeTraverser is currently processing
    node_branch: &'a mut NodeTreeBranch,
    receiver: &'a WindowReceiver,
    /// The index in the child vector to first look at when searching for a child. As new
    /// children get added, this gets incremented.
    child_index: usize,

    children_layout: L,
    queue_opened: bool
}

impl<'a, L: GridLayout> NodeTraverser<'a, L> {
    fn take<CL: GridLayout>(&mut self, child_index: usize, layout: CL) -> NodeTraverser<CL> {
        let nb = &mut self.node_branch.children[child_index];

        NodeTraverser {
            node_branch: nb,
            receiver: self.receiver,
            child_index: 0,

            children_layout: layout,
            queue_opened: false
        }
    }

    /// Process a child node, where that child node has no children.
    fn process_child_node_nochildren<N, PF>(&mut self, name: &'static str, node: &mut N, proc_func: PF)
            -> NativeResult<()>
            where N: Node,
                  PF: FnOnce(&mut N, NodeTraverser<EmptyNodeLayout>) -> NativeResult<()>
    {
        self.process_child_node(name, node, EmptyNodeLayout, proc_func)
    }

    /// Take a name, a node, and a function to run upon updating the node and either add the node to the
    /// node tree (which runs the function as well) or determine whether or not to run the update function
    /// and run it if necessary.
    fn process_child_node<N, CL, PF>(&mut self, name: &'static str, node: &mut N, child_layout: CL, proc_func: PF)
            -> NativeResult<()>
            where N: Node,
                  CL: GridLayout,
                  PF: FnOnce(&mut N, NodeTraverser<CL>) -> NativeResult<()>
    {
        if !self.queue_opened {
            self.queue_opened = true;
            self.node_branch.window.as_ref().expect("Attempted to perform `take` on node with no window")
                        .open_update_queue();
        }

        if let Some(layout_info) = self.children_layout.next() {
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
                    if let Some(ref window) = self.node_branch.children[i].window {
                        window.set_layout_info(layout_info);
                    }

                    proc_func(node, self.take(i, child_layout))?;
                    self.node_branch.children[i].state_id = new_state_id;
                }
            } else {
                self.node_branch.children.insert(
                    self.child_index,
                    node.into_ntb(
                        name,
                        self.node_branch.window.as_ref().expect("Attempted to create child window without parent"),
                        self.receiver
                    )?
                );
                let child_index = self.child_index;
                self.child_index += 1;

                if let Some(ref window) = self.node_branch.children[child_index].window {
                    window.set_layout_info(layout_info);
                }

                proc_func(node, self.take(child_index, child_layout))?;
                // Store the state id after running the processor function, so that the cached ID
                // is accurate.
                self.node_branch.children[child_index].state_id = node.state_id();

            }
        }
        Ok(())
    }
}

impl<'a, L: GridLayout> Drop for NodeTraverser<'a, L> {
    fn drop(&mut self) {
        if self.queue_opened {
            self.node_branch.window.as_ref().unwrap()
                .flush_update_queue();
        }
    }
}

impl<'a, N: Node, L: GridLayout> NodeProcessor<N> for NodeTraverser<'a, L> {
    default fn add_child(&mut self, name: &'static str, node: &mut N) -> NativeResult<()> {
        // We have no information about what's in the child node, so we can't really do anything.
        // It still needs to get added to the tree though.
        self.process_child_node_nochildren(name, node, |_, _| Ok(()))
    }
}

impl<'a, L: GridLayout> NodeProcessorAT for NodeTraverser<'a, L> {
    type Error = NativeError;
}

impl<'a, N, L> NodeProcessor<N> for NodeTraverser<'a, L>
        where L: GridLayout,
  for<'b, 'c> N: ParentNode<NodeTraverser<'b, <N as ParentNode<NodeTraverser<'c, EmptyNodeLayout>>>::Layout>> +
                 ParentNode<NodeTraverser<'c, EmptyNodeLayout>>
                // ...yup, that's one ugly-ass type annotation. The reason we're doing this is because we assume
                // that ParentNode is generic over all types that implement NodeProcessor, NodeTraverser being one
                // of them. Because we can't just HKT our way into saying that, we provide two annotations: the
                // first saying that N is a ParentNode for a NodeTraverser which has the layout of the ParentNode,
                // and the second one allowing us to access the Layout associated type without causing the type
                // system to recurse forever. That second one is used to get the layout used for the actual ParentNode.
{
    default fn add_child(&mut self, name: &'static str, node: &mut N) -> NativeResult<()> {
        let child_layout = <N as ParentNode<NodeTraverser<EmptyNodeLayout>>>::child_layout(node);

        self.process_child_node(name, node, child_layout,
            |node, traverser| {
                if let Some(WindowNode::LayoutGroup(ref lg)) = traverser.node_branch.window {
                    lg.set_grid_size(traverser.children_layout.grid_size());
                } else {unreachable!()}
                node.children(traverser)
            })
    }
}

impl<N> IntoNTB for N where
    for<'b, 'c> N: ParentNode<NodeTraverser<'b, <N as ParentNode<NodeTraverser<'c, EmptyNodeLayout>>>::Layout>> +
                   ParentNode<NodeTraverser<'c, EmptyNodeLayout>>
{
    default fn into_ntb(&self, name: &'static str, parent: &WindowNode, receiver: &WindowReceiver) -> NativeResult<NodeTreeBranch> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            name: name,
            window: Some(parent.new_layout_group(receiver)?),
            children: Vec::new()
        })
    }
}

impl<'a, S, L> NodeProcessor<TextButton<S>> for NodeTraverser<'a, L>
        where S: AsRef<str>,
              L: GridLayout {
    fn add_child(&mut self, name: &'static str, node: &mut TextButton<S>) -> NativeResult<()> {
        self.process_child_node_nochildren(name, node, |node, traverser| {
            if let Some(WindowNode::TextButton(ref mut b)) = traverser.node_branch.window {
                b.set_text(node.as_ref());

                Ok(())
            } else {panic!("Mismatched WindowNode in TextButton. Please report code that caused this in derin repository.")}
        })
    }
}

impl<'a, S: AsRef<str>> IntoNTB for TextButton<S> {
    fn into_ntb(&self, name: &'static str, parent: &WindowNode, receiver: &WindowReceiver) -> NativeResult<NodeTreeBranch> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            name: name,
            window: Some(parent.new_text_button(receiver)?),
            children: Vec::new()
        })
    }
}
