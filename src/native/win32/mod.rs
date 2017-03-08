mod wrapper;
mod toggle_cell;

use self::wrapper::*;
use super::WindowConfig;
use dww::{msg_queue, Window as WindowTrait, OverlappedWindow, WindowBuilder};

use std::rc::Rc;
use std::{ptr, mem};
use std::cell::RefCell;
use std::io::{Result, Error};

use ui::{Node, Control, ChildId, NodeProcessor, NodeProcessorAT, NodeDataRegistry};
use ui::intrinsics::{TextButton, TextLabel};

pub struct Window<N>
        where N: Node<Wrapper = <NativeWrapperRegistry as NodeDataRegistry<N>>::NodeDataWrapper>,
              N::Wrapper: NativeDataWrapper,
              NativeWrapperRegistry: NodeDataRegistry<N>
{
    pub root: N,
    toplevel: ToplevelWindow,
    action_fn: SharedFn<N::Action>,
    self_ptr: *const Window<N>
}

impl<N> Window<N>
        where N: Node<Wrapper = <NativeWrapperRegistry as NodeDataRegistry<N>>::NodeDataWrapper>,
              N::Wrapper: NativeDataWrapper,
              NativeWrapperRegistry: NodeDataRegistry<N>
{
    pub fn new(root: N, config: &WindowConfig) -> Window<N> {

        let overlapped = WindowBuilder {
            pos: None,
            size: config.size,
            window_text: &config.name,
            show_window: config.show_window
        }.build_blank().as_overlapped(true);
        overlapped.size_border(config.resizable);
        overlapped.max_button(config.maximizable);
        overlapped.min_button(config.minimizable);

        Window {
            toplevel: ToplevelWindow::new(overlapped, &root),
            root: root,
            action_fn: Rc::new(RefCell::new(unsafe{ mem::zeroed() })),
            self_ptr: ptr::null()
        }
    }

    pub fn wait_actions<F>(&mut self, mut f: F)
            where F: FnMut(N::Action) -> bool
    {
        let node_data_moved = self.self_ptr != self;
        self.self_ptr = self;
        self.action_fn.borrow_mut().set_fn(&mut f);

        if node_data_moved {
            unsafe{ self.toplevel.update_subclass_ptr() };
        }

        NativeNodeProcessor::<_, N::Action> {
            node: &mut self.toplevel,
            action_fn: &self.action_fn,
            node_data_moved: node_data_moved,
            children_updated: false
        }.add_child(ChildId::Num(0), &mut self.root).ok();

        // Modifying the size bounds of windows inside of the toplevel window doesn't trigger a size
        // bounds check in the toplevel. This forces that check.
        self.toplevel.bound_to_size_bounds();

        for msg in msg_queue::thread_wait_queue() {
            let msg = msg.expect("Windows message error");
            unsafe{ msg.dispatch() };
        }

        self.action_fn.borrow_mut().clear();
    }
}

struct NativeNodeProcessor<'a, N, A>
        where N: 'a + Node, A: 'a
{
    /// The branch that this instance of NativeNodeProcessor is currently processing
    node: &'a mut N,
    action_fn: &'a SharedFn<A>,
    node_data_moved: bool,
    children_updated: bool
}

impl<'a, N, A> NodeProcessorAT for NativeNodeProcessor<'a, N, A>
        where N: 'a + Node
{
    type Error = Error;
}

impl<'a, N, C, A> NodeProcessor<C> for NativeNodeProcessor<'a, N, A>
        where N: 'a + Node, A: 'a, C: Node<Wrapper = <NativeWrapperRegistry as NodeDataRegistry<C>>::NodeDataWrapper>,
              NativeWrapperRegistry: NodeDataRegistry<C>
{
    default fn add_child<'b>(&'b mut self, _: ChildId, _: &'b mut C) -> Result<()> {
        panic!("This function should never be called directly, but instead a specialized version should be")
    }
}

impl<'a, N, I, A> NodeProcessor<TextButton<I>> for NativeNodeProcessor<'a, N, A>
        where N: 'a + Node, A: 'a, I: AsRef<str> + Control<Action = A>,
              N::Wrapper: ParentDataWrapper
{
    fn add_child<'b>(&'b mut self, _: ChildId, node: &'b mut TextButton<I>) -> Result<()> {
        node.wrapper().update_subclass_ptr();
        if node.wrapper().needs_update() {
            node.wrapper_mut().update_widget(self.action_fn);
            self.node.wrapper_mut().add_child_node(node);
        }
        Ok(())
    }
}

impl<'a, N, S, A> NodeProcessor<TextLabel<S>> for NativeNodeProcessor<'a, N, A>
        where N: 'a + Node, A: 'a, S: AsRef<str>,
              N::Wrapper: ParentDataWrapper
{
    fn add_child<'b>(&'b mut self, _: ChildId, node: &'b mut TextLabel<S>) -> Result<()> {
        node.wrapper().update_subclass_ptr();
        if node.wrapper().needs_update() {
            node.wrapper_mut().update_widget();
            self.node.wrapper_mut().add_child_node(node);
        }
        Ok(())
    }
}


pub struct NativeWrapperRegistry;
impl<I: AsRef<str> + Control> NodeDataRegistry<TextButton<I>> for NativeWrapperRegistry {
    type NodeDataWrapper = TextButtonNodeData<I>;
}
impl<S: AsRef<str>> NodeDataRegistry<TextLabel<S>> for NativeWrapperRegistry {
    type NodeDataWrapper = TextLabelNodeData<S>;
}
