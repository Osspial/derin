mod wrapper;

use self::wrapper::{WindowNode, CallbackData};

use user32;

use std::ptr;
use std::mem;

use boolinator::Boolinator;

use native::{NativeResult, NativeError};
use native::WindowConfig;

use ui::{ChildId, Node, ActionNode, Control, NodeProcessor, NodeProcessorAT, ParentNode};
use ui::intrinsics::{TextButton, TextLabel};
use ui::layout::{GridLayout, SingleNodeLayout};

use dle::Tr;


pub struct Window<N: ActionNode> {
    pub root: N,
    node_tree_root: NodeTreeBranch<N::Action>,
    action: Option<N::Action>,
    self_ptr: *const Window<N>
}

impl<N: ActionNode> Window<N> {
    pub fn new(root: N, config: &WindowConfig) -> NativeResult<Window<N>> {
        unsafe {
            self::wrapper::enable_visual_styles();

            let cd = CallbackData::new();

            let wrapper_window = WindowNode::new_toplevel(config, cd)?;

            Ok(
                Window {
                    root: root,
                    // The node that contains the top-level node and all other nodes, including root.
                    node_tree_root: NodeTreeBranch {
                        state_id: 0,
                        child_id: ChildId::Num(0),
                        window: Some(wrapper_window),
                        children: Vec::with_capacity(1)
                    },
                    action: None,
                    self_ptr: ptr::null()
                }
            )
        }
    }

    pub fn wait_actions(&mut self) -> NativeResult<WaitActionsIter<N>>
            where for<'a> N: ParentNode<NodeTraverser<'a, SingleNodeLayout, <N as ActionNode>::Action>> {
        unsafe {
            let mut force_child_updates = false;
            if self.self_ptr != self {
                if let Some(WindowNode::Toplevel(ref tl)) = self.node_tree_root.window {
                    tl.set_action_ptr(&mut self.action);
                }
                force_child_updates = true;
            }
            self.self_ptr = self;

            let res = NodeTraverser::<SingleNodeLayout, N::Action> {
                force_child_updates: force_child_updates,
                node_branch: &mut self.node_tree_root,
                child_index: 0,

                child_widget_hints: SingleNodeLayout::default().widget_hints(),
                queue_opened: false
            }.add_child(ChildId::Num(0), &mut self.root);

            if res.is_ok() {
                Ok(WaitActionsIter{ window: self })
            } else {
                Err(res.unwrap_err())
            }
        }
    }
}

pub struct WaitActionsIter<'a, N: 'a + ActionNode> {
    window: &'a mut Window<N>
}

impl<'a, N: ActionNode> Iterator for WaitActionsIter<'a, N> {
    type Item = N::Action;

    fn next(&mut self) -> Option<N::Action> {
        unsafe {
            // Win32 message loop
            let mut msg = mem::uninitialized();
            while self.window.action.is_none() && user32::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                user32::TranslateMessage(&msg);
                user32::DispatchMessageW(&msg);
            }

            self.window.action.take()
        }
    }
}


/// A node in the tree that represents the nodes of the UI tree!
struct NodeTreeBranch<A> {
    state_id: u16,
    child_id: ChildId,
    window: Option<WindowNode<A>>,
    children: Vec<NodeTreeBranch<A>>
}

/// Trait for converting `Node`s into `NodeTreeBranch`es.
trait IntoNTB<A>: Node {
    fn into_ntb(&self, child_id: ChildId, parent: &WindowNode<A>) -> NativeResult<NodeTreeBranch<A>>;
}

impl<N: Node, A> IntoNTB<A> for N {
    default fn into_ntb(&self, child_id: ChildId, _: &WindowNode<A>) -> NativeResult<NodeTreeBranch<A>> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            child_id: child_id,
            window: None,
            children: Vec::new()
        })
    }
}

#[doc(hidden)]
pub struct NodeTraverser<'a, L: GridLayout, A: 'a> {
    /// The branch that this instance of NodeTraverser is currently processing
    node_branch: &'a mut NodeTreeBranch<A>,
    /// The index in the child vector to first look at when searching for a child. As new
    /// children get added, this gets incremented.
    child_index: usize,
    force_child_updates: bool,

    child_widget_hints: L::WidgetHintsIter,
    queue_opened: bool
}

impl<'a, L: GridLayout, A> NodeTraverser<'a, L, A> {
    fn take<CL: GridLayout>(&mut self, child_index: usize, layout: CL) -> NodeTraverser<CL, A> {
        let nb = &mut self.node_branch.children[child_index];

        NodeTraverser {
            node_branch: nb,
            child_index: 0,
            force_child_updates: self.force_child_updates,

            child_widget_hints: layout.widget_hints(),
            queue_opened: false
        }
    }

    fn get_child_index(&self, child_id: ChildId) -> Option<usize> {
        // If the desired node branch is at the current child index, get that and return it.
        // Otherwise, search the entire children vector for the desired node and return it if
        // that's found.
        self.node_branch.children.get(self.child_index)
            .and_then(|branch| (branch.child_id == child_id).as_some(self.child_index))
            .or(self.node_branch.children
               .iter().enumerate()
               .filter_map(|(i, branch)| (branch.child_id == child_id).as_some(i))
               .next())
    }

    /// Process a child node, where that child node has no children.
    fn process_leaf_child_node<N, PF>(&mut self, child_id: ChildId, node: &N, proc_func: PF)
            -> NativeResult<()>
            where N: Node,
                  PF: FnOnce(&N, &mut NodeTreeBranch<A>) -> NativeResult<()>
    {
        self.process_child_node(child_id, node,
            |this, node, child_index| proc_func(node, &mut this.node_branch.children[child_index]))
    }

    /// Take a name, a node, and a function to run upon updating the node and either add the node to the
    /// node tree (which runs the function as well) or determine whether or not to run the update function
    /// and run it if necessary.
    fn process_parent_child_node<N, CL, PF>(&mut self, child_id: ChildId, node: &N, child_layout: CL, proc_func: PF)
            -> NativeResult<()>
            where N: ActionNode<Action = A>,
                  CL: GridLayout,
                  PF: FnOnce(&N, NodeTraverser<CL, A>) -> NativeResult<()>
    {
        self.process_child_node(child_id, node,
            |this, node, child_index| proc_func(node, this.take(child_index, child_layout)))
    }

    fn process_child_node<N, PF>(&mut self, child_id: ChildId, node: &N, proc_func: PF)
            -> NativeResult<()>
            where N: Node,
                  PF: FnOnce(&mut Self, &N, usize) -> NativeResult<()>
    {
        if !self.queue_opened {
            self.queue_opened = true;
            self.node_branch.window.as_ref().expect("Attempted to perform `take` on node with no window")
                        .open_update_queue();
        }

        if let Some(layout_info) = self.child_widget_hints.next() {
            // If the node with the desired ID can be found in `node_branch.children`, run the update function
            // if the state ID's been changed. Otherwise add it to the `children` vector.
            if let Some(i) = self.get_child_index(child_id) {
                self.child_index = i + 1;

                // Compare the newly-generated state id and the cached state id. If there is a mismatch, update the
                // cached id and run the processing function.
                let new_state_id = node.state_id();
                if self.force_child_updates || self.node_branch.children[i].state_id != new_state_id {
                    if let Some(ref window) = self.node_branch.children[i].window {
                        window.set_widget_hints(layout_info);
                    }

                    proc_func(self, node, i)?;
                    self.node_branch.children[i].state_id = new_state_id;
                }
            } else {
                self.node_branch.children.insert(
                    self.child_index,
                    node.into_ntb(
                        child_id,
                        self.node_branch.window.as_ref().expect("Attempted to create child window without parent")
                    )?
                );
                let child_index = self.child_index;
                self.child_index += 1;

                if let Some(ref window) = self.node_branch.children[child_index].window {
                    window.set_widget_hints(layout_info);
                }

                proc_func(self, node, child_index)?;
                // Store the state id after running the processor function, so that the cached ID
                // is accurate.
                self.node_branch.children[child_index].state_id = node.state_id();

            }
        }
        Ok(())
    }
}

impl<'a, L: GridLayout, A> Drop for NodeTraverser<'a, L, A> {
    fn drop(&mut self) {
        if self.queue_opened {
            self.node_branch.window.as_ref().unwrap()
                .flush_update_queue();
        }
    }
}

impl<'a, N, L, A> NodeProcessor<N> for NodeTraverser<'a, L, A>
        where N: Node, L: GridLayout
{
    default unsafe fn add_child(&mut self, child_id: ChildId, node: &N) -> NativeResult<()> {
        // We have no information about what's in the child node, so we can't really do anything.
        // It still needs to get added to the tree though.
        self.process_leaf_child_node(child_id, node, |_, _| Ok(()))
    }
}

impl<'a, L: GridLayout, A> NodeProcessorAT for NodeTraverser<'a, L, A> {
    type Error = NativeError;
}

impl<'a, N, L> NodeProcessor<N> for NodeTraverser<'a, L, N::Action>
        where L: GridLayout,
      for<'b> N: ParentNode<NodeTraverser<'b, <N as ParentNode<()>>::Layout, <N as ActionNode>::Action>> +
                 ParentNode<()>
{
    default unsafe fn add_child(&mut self, child_id: ChildId, node: &N) -> NativeResult<()> {
        let child_layout = <N as ParentNode<()>>::child_layout(node);
        let child_grid_size = child_layout.grid_size();
        let col_hints = child_layout.col_hints().take(child_grid_size.x as usize);
        let row_hints = child_layout.row_hints().take(child_grid_size.y as usize);

        self.process_parent_child_node(child_id, node, child_layout,
            |node, traverser| {
                if let Some(WindowNode::LayoutGroup(ref lg)) = traverser.node_branch.window {
                    lg.set_grid_size(child_grid_size);

                    lg.set_row_hints(0, ::dle::hints::TrackHints::default());
                    for (col, hint) in col_hints.enumerate() {
                        lg.set_col_hints(col as Tr, hint);
                    }
                    for (row, hint) in row_hints.enumerate() {
                        lg.set_row_hints(row as Tr, hint);
                    }
                } else {unreachable!()}
                node.children(traverser)
            })
    }
}

impl<N> IntoNTB<N::Action> for N where
    for<'b, 'c> N: ParentNode<NodeTraverser<'b, <N as ParentNode<()>>::Layout, <N as ActionNode>::Action>> +
                   ParentNode<()>
{
    default fn into_ntb(&self, child_id: ChildId, parent: &WindowNode<N::Action>) -> NativeResult<NodeTreeBranch<N::Action>> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            child_id: child_id,
            window: Some(parent.new_layout_group()?),
            children: Vec::new()
        })
    }
}

impl<'a, I, L> NodeProcessor<TextButton<I>> for NodeTraverser<'a, L, I::Action>
        where I: AsRef<str> + Control,
              L: GridLayout
{
    unsafe fn add_child(&mut self, child_id: ChildId, node: &TextButton<I>) -> NativeResult<()> {
        self.process_leaf_child_node(child_id, node, |node, node_branch| {
            if let Some(WindowNode::TextButton(ref mut b)) = node_branch.window {
                b.set_control_ptr(TextButton::inner(node));
                b.set_text(TextButton::inner(node).as_ref());

                Ok(())
            } else {panic!("Mismatched WindowNode in TextButton. Please report code that caused this in derin repository.")}
        })
    }
}

impl<'a, I: 'static + AsRef<str> + Control> IntoNTB<I::Action> for TextButton<I> {
    fn into_ntb(&self, child_id: ChildId, parent: &WindowNode<I::Action>) -> NativeResult<NodeTreeBranch<I::Action>> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            child_id: child_id,
            window: Some(parent.new_text_button()?),
            children: Vec::new()
        })
    }
}

impl<'a, S, L, A> NodeProcessor<TextLabel<S>> for NodeTraverser<'a, L, A>
        where S: AsRef<str>,
              L: GridLayout
{
    unsafe fn add_child(&mut self, child_id: ChildId, node: &TextLabel<S>) -> NativeResult<()> {
        self.process_leaf_child_node(child_id, node, |node, node_branch| {
            if let Some(WindowNode::TextLabel(ref mut l)) = node_branch.window {
                l.set_text(TextLabel::text(node).as_ref());
                Ok(())
            } else {panic!("Mismatched WindowNode in TextLabel. Please report code that caused this in derin repository.")}
        })
    }
}

impl<'a, S: AsRef<str>, A> IntoNTB<A> for TextLabel<S> {
    fn into_ntb(&self, child_id: ChildId, parent: &WindowNode<A>) -> NativeResult<NodeTreeBranch<A>> {
        Ok(NodeTreeBranch {
            state_id: self.state_id(),
            child_id: child_id,
            window: Some(parent.new_text_label()?),
            children: Vec::new()
        })
    }
}
