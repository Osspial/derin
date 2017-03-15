mod wrapper;
mod toggle_cell;

use self::wrapper::*;
use super::WindowConfig;
use dle::hints::{WidgetHints, NodeSpan};
use dww::{msg_queue, Window as WindowTrait, OverlappedWindow, WindowBuilder};

use std::rc::Rc;
use std::ptr;
use std::iter::{self, Once};
use std::cell::RefCell;
use std::io::{Result, Error};

use ui::layout::GridLayout;
use ui::intrinsics::{TextButton, TextLabel, WidgetGroup};
use ui::{Node, Control, Parent, ChildId, NodeProcessor, NodeProcessorAT, NodeDataRegistry};

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
            action_fn: Rc::new(RefCell::new(ActionFn::new())),
            self_ptr: ptr::null()
        }
    }

    pub fn wait_actions<F>(&mut self, mut f: F) -> Result<()>
            where F: FnMut(N::Action) -> bool
    {
        let node_data_moved = self.self_ptr != self;
        self.self_ptr = self;
        self.action_fn.borrow_mut().set_fn(&mut f);

        if node_data_moved {
            self.toplevel.update_subclass_ptr();
        }

        let root_widget_hints = WidgetHints {
            node_span: NodeSpan::new(.., ..),
            ..WidgetHints::default()
        };
        NativeNodeProcessor::<_, N::Action, Once<WidgetHints>> {
            parent: &mut self.toplevel,
            action_fn: &self.action_fn,
            hint_iter: iter::once(root_widget_hints),
            children_updated: false
        }.add_child(ChildId::Num(0), &mut self.root)?;

        // Modifying the size bounds of windows inside of the toplevel window doesn't trigger a size
        // bounds check in the toplevel. This forces that check.
        self.toplevel.bound_to_size_bounds();

        for msg in msg_queue::thread_wait_queue() {
            let msg = msg.expect("Windows message error");
            unsafe{ msg.dispatch() };
            if !self.action_fn.borrow().continue_loop {
                break;
            }
        }

        self.action_fn.borrow_mut().clear();
        Ok(())
    }
}

struct NativeNodeProcessor<'a, P: 'a, A: 'a, H: Iterator<Item=WidgetHints>> {
    /// The branch that this instance of NativeNodeProcessor is currently processing
    parent: &'a mut P,
    action_fn: &'a SharedFn<A>,
    hint_iter: H,
    children_updated: bool
}

impl<'a, P, A, H> NodeProcessorAT for NativeNodeProcessor<'a, P, A, H>
        where H: Iterator<Item=WidgetHints>
{
    type Error = Error;
}

impl<'a, P, A, H, C> NodeProcessor<C> for NativeNodeProcessor<'a, P, A, H>
        where C: Node,
              H: Iterator<Item=WidgetHints>
{
    default fn add_child<'b>(&'b mut self, _: ChildId, _: &'b mut C) -> Result<()> {
        panic!("This function should never be called directly, but instead a specialized version should be")
    }
}

impl<'a, P, A, H, I> NodeProcessor<TextButton<I>> for NativeNodeProcessor<'a, P, A, H>
        where P: ParentChildAdder,
              I: AsRef<str> + Control<Action = A>,
              H: Iterator<Item=WidgetHints>
{
    fn add_child<'b>(&'b mut self, _: ChildId, button: &'b mut TextButton<I>) -> Result<()> {
        let widget_hints = self.hint_iter.next().unwrap_or(WidgetHints::default());
        button.wrapper().update_subclass_ptr();

        if button.wrapper().needs_update() {
            self.children_updated = true;
            button.wrapper_mut().update_widget(widget_hints, self.action_fn);
            self.parent.add_child_node(button);
        }
        Ok(())
    }
}

impl<'a, P, A, H, S> NodeProcessor<TextLabel<S>> for NativeNodeProcessor<'a, P, A, H>
        where P: ParentChildAdder,
              S: AsRef<str>,
              H: Iterator<Item=WidgetHints>
{
    fn add_child<'b>(&'b mut self, _: ChildId, label: &'b mut TextLabel<S>) -> Result<()> {
        let widget_hints = self.hint_iter.next().unwrap_or(WidgetHints::default());
        label.wrapper().update_subclass_ptr();

        if label.wrapper().needs_update() {
            self.children_updated = true;
            label.wrapper_mut().update_widget(widget_hints);
            self.parent.add_child_node(label);
        }
        Ok(())
    }
}

impl<'a, P, A, H, I> NodeProcessor<WidgetGroup<I>> for NativeNodeProcessor<'a, P, A, H>
        where P: ParentChildAdder,
      for<'b> I: Parent<()> +
                 Parent<NativeNodeProcessor<'b, WidgetGroupAdder, A, <<I as Parent<()>>::ChildLayout as GridLayout>::WidgetHintsIter>, ChildAction = A> +
                 Parent<ConstraintSolverTraverser<'b>>,
              H: Iterator<Item=WidgetHints>
{
    fn add_child<'b>(&'b mut self, _: ChildId, group: &'b mut WidgetGroup<I>) -> Result<()> {
        let widget_hints = self.hint_iter.next().unwrap_or(WidgetHints::default());
        group.wrapper().update_subclass_ptr();

        if group.wrapper().needs_update() {
            self.children_updated = true;
            self.parent.add_child_node(group);
        }

        let mut adder = group.wrapper().get_adder();
        let grid_layout = <I as Parent<()>>::child_layout(WidgetGroup::inner(group));
        let mut child_processor = NativeNodeProcessor {
            parent: &mut adder,
            action_fn: self.action_fn,
            hint_iter: grid_layout.widget_hints(),
            children_updated: false
        };
        group.children(&mut child_processor)?;

        if child_processor.children_updated {
            group.wrapper_mut().update_widget(
                widget_hints,
                grid_layout.grid_size(),
                grid_layout.col_hints(),
                grid_layout.row_hints()
            );
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
impl<I: Parent<()>> NodeDataRegistry<WidgetGroup<I>> for NativeWrapperRegistry {
    type NodeDataWrapper = WidgetGroupNodeData<I>;
}
