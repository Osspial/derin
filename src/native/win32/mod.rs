mod wrapper;
mod geometry;
use self::wrapper::{WindowNode, Toplevel, CallbackData, RawEvent};
use self::geometry::{HintedCell, GridDims, OriginRect};

use user32;

use std::ptr;
use std::mem;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::os::raw::c_int;

use boolinator::Boolinator;

use native::{NativeResult, NativeError};
use native::WindowConfig;

use ui::{Node, NodeProcessor, ParentNode};
use ui::intrinsics::TextButton;
use ui::layout::{GridLayout, EmptyNodeLayout, SingleNodeLayout, GridSize};


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
                let cd = CallbackData {
                    window_sender: window_sender.clone(),
                    event_sender: event_sender
                };

                // Create a wrapper toplevel window. If the creation succeeds, send back the window. Otherwise, send
                // back the error it created and terminate this thread.
                let wrapper_window = Toplevel::new(&config, &cd as *const CallbackData);
                match wrapper_window {
                    Ok(wr) => {
                        window_sender.send(Ok(WindowNode::Toplevel(wr))).unwrap();
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

        let grid = if let WindowNode::Toplevel(ref tl) = wrapper_window {
            let (width, height) = tl.get_inner_size().unwrap();
            GridDims::with_size(GridSize::new(1, 1), OriginRect::new(width as c_int, height as c_int))
        } else {unreachable!()};

        Ok(
            Window {
                root: root,
                // The node that contains the top-level node and all other nodes, including root.
                node_tree_root: NodeTreeBranch {
                    state_id: 0,
                    name: "toplevel",
                    window: Some(wrapper_window),
                    grid: grid,
                    children: Vec::with_capacity(1),
                    cascade_change: true
                },

                window_receiver: window_receiver,
                event_receiver: event_receiver
            }
        )
    }

    pub fn process(&mut self) -> NativeResult<()> {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                RawEvent::ToplevelResized(x, y) => {
                    self.node_tree_root.grid.zero_all();
                    self.node_tree_root.grid.expand_size_px(OriginRect::new(x, y));
                    self.node_tree_root.cascade_change = true;
                }
                _ => ()
            }
        }

        NodeTraverser {
            cascade_change: self.node_tree_root.cascade_change,

            node_branch: &mut self.node_tree_root,
            parent_window: None,
            receiver: &self.window_receiver,
            child_index: 0,

            children_layout: SingleNodeLayout::new()
        }.add_child("root", &mut self.root)
    }
}

/// A node in the tree that represents the nodes of the UI tree!
struct NodeTreeBranch {
    state_id: u16,
    name: &'static str,
    window: Option<WindowNode>,
    grid: GridDims,
    children: Vec<NodeTreeBranch>,
    cascade_change: bool
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
            grid: GridDims::new(),
            children: Vec::new(),
            cascade_change: true
        })
    }
}

struct NodeTraverser<'a, L: GridLayout> {
    /// The branch that this instance of NodeTraverser is currently processing
    node_branch: &'a mut NodeTreeBranch,
    /// The `WindowNode` belonging to the parent `NodeTreeBranch` of this `NodeTraverser`. In the event
    /// that this `NodeTraverser` represents the root node, this is `None`.
    parent_window: Option<&'a WindowNode>,
    receiver: &'a WindowReceiver,
    /// The index in the child vector to first look at when searching for a child. As new
    /// children get added, this gets incremented.
    child_index: usize,

    children_layout: L,
    cascade_change: bool
}

impl<'a, L: GridLayout> NodeTraverser<'a, L> {
    fn take<CL: GridLayout>(&mut self, child_index: usize, layout: CL) -> NodeTraverser<CL> {
        let nb = &mut self.node_branch.children[child_index];
        nb.grid.set_grid_size(layout.grid_size());

        NodeTraverser {
            cascade_change: self.cascade_change || nb.cascade_change,

            node_branch: nb,
            parent_window: self.node_branch.window.as_ref(),
            receiver: self.receiver,
            child_index: 0,

            children_layout: layout
        }
    }

    /// Process a child node, where that child node has no children.
    fn process_child_node_nochildren<N, PF>(&mut self, name: &'static str, node: &mut N, proc_func: PF)
            -> NativeResult<()>
            where N: Node,
                  PF: FnOnce(&mut N, NodeTraverser<EmptyNodeLayout>, &mut HintedCell) -> NativeResult<()>
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
                  PF: FnOnce(&mut N, NodeTraverser<CL>, &mut HintedCell) -> NativeResult<()>
    {
        if let Some(slot) = self.children_layout.next() {
            let (slot_x, slot_y) = (slot.node_span.x.start.unwrap(), slot.node_span.y.start.unwrap());
            let mut hinted_cell = HintedCell::new(
                self.node_branch.grid
                    .get_cell_rect(slot_x, slot_y)
                    .expect("Out of bounds error; handle without panicing in future"),
                slot.place_in_cell
            );

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
                if self.cascade_change || self.node_branch.children[i].state_id != new_state_id {
                    self.node_branch.children[i].state_id = new_state_id;
                    proc_func(node, self.take(i, child_layout), &mut hinted_cell)?;
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
                proc_func(node, self.take(child_index, child_layout), &mut hinted_cell)?;
            }

            self.node_branch.grid.expand_cell_rect(
                slot_x, slot_y, 
                hinted_cell.inner_rect()
                    .map(|r| OriginRect::from(r))
                    .unwrap_or(OriginRect::default())
                );
        }
        Ok(())
    }
}

impl<'a, L: GridLayout> Drop for NodeTraverser<'a, L> {
    fn drop(&mut self) {
        self.node_branch.cascade_change = false;
    }
}

impl<'a, N: Node, L: GridLayout> NodeProcessor<N> for NodeTraverser<'a, L> {
    type Error = NativeError;
    default fn add_child(&mut self, name: &'static str, node: &mut N) -> NativeResult<()> {
        // We have no information about what's in the child node, so we can't really do anything.
        // It still needs to get added to the tree though.
        self.process_child_node_nochildren(name, node, |_, _, _| Ok(()))
    }
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
            |node, traverser, _| node.children(traverser))
    }
}

impl<'a, S, L> NodeProcessor<TextButton<S>> for NodeTraverser<'a, L>
        where S: AsRef<str>,
              L: GridLayout {
    fn add_child(&mut self, name: &'static str, node: &mut TextButton<S>) -> NativeResult<()> {
        self.process_child_node_nochildren(name, node, |node, traverser, rhint| {
            if let Some(WindowNode::TextButton(ref mut b)) = traverser.node_branch.window {
                b.set_text(node.as_ref());
                let button_rect = rhint.transform_min_rect(b.get_ideal_rect());
                b.set_rect(button_rect);

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
            grid: GridDims::new(),
            children: Vec::new(),
            cascade_change: true
        })
    }
}
