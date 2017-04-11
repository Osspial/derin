mod wrapper;

use self::wrapper::*;
use super::WindowConfig;
use dww::{msg_queue, WindowOwned, OverlappedWindow, WindowBuilder};

use std::rc::Rc;
use std::ptr;
use std::borrow::Borrow;
use std::cell::RefCell;

use ui::{Node, Parent, ChildId, NodeProcessorGridMut, NodeProcessor, NodeProcessorInit, NodeDataWrapper, NodeDataRegistry};
use ui::widgets::{ButtonControl, TextButton, TextLabel, WidgetGroup, ProgressBar, Slider, SliderControl};
use ui::hints::{WidgetHints, GridSize, TrackHints, NodeSpan};

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
    pub fn new(mut root: N, config: &WindowConfig) -> Window<N> {
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
            toplevel: unsafe{ ToplevelWindow::new(overlapped, root.wrapper_mut().unsafe_subclass_ref()) },
            root: root,
            action_fn: Rc::new(RefCell::new(ActionFn::new())),
            self_ptr: ptr::null()
        }
    }

    pub fn wait_actions<F>(&mut self, mut f: F) -> Result<(), !>
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
        NativeNodeProcessor::<_, N::Action> {
            parent: &mut self.toplevel,
            action_fn: &self.action_fn,
            children_updated: &mut false
        }.add_child_mut(ChildId::Num(0), root_widget_hints, &mut self.root)?;

        // Modifying the size bounds of windows inside of the toplevel window doesn't trigger a size
        // bounds check in the toplevel. This forces that check.
        self.toplevel.bound_to_size_bounds();

        for msg in msg_queue::thread_wait_queue() {
            let msg = msg.expect("Windows message error");
            unsafe{ msg.dispatch() };
            if !<RefCell<_>>::borrow(&self.action_fn).continue_loop {
                break;
            }
        }

        self.action_fn.borrow_mut().clear();
        Ok(())
    }
}

struct NativeNodeProcessor<'a, P: 'a, A: 'a> {
    /// The branch that this instance of NativeNodeProcessor is currently processing
    parent: &'a mut P,
    action_fn: &'a SharedFn<A>,
    children_updated: &'a mut bool
}

impl<'a, P, A> NodeProcessor for NativeNodeProcessor<'a, P, A> {
    type Error = !;
}

impl<'a, P, A> NodeProcessorInit for NativeNodeProcessor<'a, P, A> {
    type Error = !;
    type GridProcessor = Self;
    fn init_grid<C, R>(self, _: GridSize, _: C, _: R) -> Self::GridProcessor
            where C: Iterator<Item = TrackHints>,
                  R: Iterator<Item = TrackHints>
    {
        self
    }
}

impl<'a, P, A, C> NodeProcessorGridMut<C> for NativeNodeProcessor<'a, P, A>
        where C: Node
{
    default fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, _: &'b mut C) -> Result<(), !> {
        panic!("This function should never be called directly, but instead a specialized version should be")
    }
}

impl<'a, P, A, I> NodeProcessorGridMut<TextButton<I>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
              I: ButtonControl<Action = A> + Borrow<str>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, button: &'b mut TextButton<I>) -> Result<(), !> {
        button.wrapper().update_subclass_ptr();

        if button.wrapper().needs_update() {
            *self.children_updated = true;
            button.wrapper_mut().update_widget(self.action_fn);
            self.parent.add_child_node(button.wrapper_mut());
        }
        Ok(())
    }
}

impl<'a, P, A, S> NodeProcessorGridMut<TextLabel<S>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
              S: AsRef<str>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, label: &'b mut TextLabel<S>) -> Result<(), !> {
        label.wrapper().update_subclass_ptr();

        if label.wrapper().needs_update() {
            *self.children_updated = true;
            label.wrapper_mut().update_widget();
            self.parent.add_child_node(label.wrapper_mut());
        }
        Ok(())
    }
}

impl<'a, P, A, I> NodeProcessorGridMut<WidgetGroup<I>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
      for<'b> I: Parent<!> +
                 Parent<NativeNodeProcessor<'b, WidgetGroupAdder, A>, ChildAction = A> +
                 Parent<GridWidgetProcessor<'b>> +
                 Parent<EngineTypeHarvester<'b>>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, group: &'b mut WidgetGroup<I>) -> Result<(), !> {
        let group_wrapper = group.wrapper_mut();
        group_wrapper.update_subclass_ptr();

        if group_wrapper.needs_update() {
            *self.children_updated = true;
            self.parent.add_child_node(group_wrapper);
        }

        let mut adder = group_wrapper.get_adder();
        let mut children_updated = false;
        {
            let child_processor = NativeNodeProcessor {
                parent: &mut adder,
                action_fn: self.action_fn,
                children_updated: &mut children_updated
            };
            group_wrapper.inner_mut().children_mut(child_processor)?;
        }

        if children_updated {
            group_wrapper.update_widget();
        }
        Ok(())
    }
}

impl<'a, P, A> NodeProcessorGridMut<ProgressBar> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, progress_bar: &'b mut ProgressBar) -> Result<(), !> {
        progress_bar.wrapper().update_subclass_ptr();

        if progress_bar.wrapper().needs_update() {
            *self.children_updated = true;
            progress_bar.wrapper_mut().update_widget();
            self.parent.add_child_node(progress_bar.wrapper_mut());
        }
        Ok(())
    }
}

impl<'a, P, A, C> NodeProcessorGridMut<Slider<C>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
              C: SliderControl<Action = A>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, label: &'b mut Slider<C>) -> Result<(), !> {
        label.wrapper().update_subclass_ptr();

        if label.wrapper().needs_update() {
            *self.children_updated = true;
            label.wrapper_mut().update_widget(self.action_fn);
            self.parent.add_child_node(label.wrapper_mut());
        }
        Ok(())
    }
}


pub struct NativeWrapperRegistry;
impl<I: ButtonControl + Borrow<str>> NodeDataRegistry<TextButton<I>> for NativeWrapperRegistry {
    type NodeDataWrapper = TextButtonNodeData<I>;
}
impl<S: AsRef<str>> NodeDataRegistry<TextLabel<S>> for NativeWrapperRegistry {
    type NodeDataWrapper = TextLabelNodeData<S>;
}
impl<I: Parent<!>> NodeDataRegistry<WidgetGroup<I>> for NativeWrapperRegistry {
    type NodeDataWrapper = WidgetGroupNodeData<I>;
}
impl NodeDataRegistry<ProgressBar> for NativeWrapperRegistry {
    type NodeDataWrapper = ProgressBarNodeData;
}
impl<C: SliderControl> NodeDataRegistry<Slider<C>> for NativeWrapperRegistry {
    type NodeDataWrapper = SliderNodeData<C>;
}
