mod wrapper;

use self::wrapper::*;
use super::WindowConfig;
use dww::msg;
use dww::window::{WindowOwned, WindowBase, ParentWindow, OverlappedWindow, WindowBuilder};
use dww::window::refs::WindowRef;

use std::rc::Rc;
use std::ptr;
use std::cell::RefCell;

use ui::*;
use ui::widgets::{TextButton, TextLabel, Group, LabelGroup, Progbar, Slider, MouseEvent, RangeEvent};
use ui::hints::{WidgetHints, GridSize, TrackHints, NodeSpan};

pub struct Window<N>
        where N: Node<Wrapper = <NativeWrapperRegistry as NodeDataRegistry<N>>::NodeDataWrapper>,
              N::Wrapper: NativeDataWrapper,
              NativeWrapperRegistry: NodeDataRegistry<N>
{
    pub root: N,
    toplevel: ToplevelWindow,
    action_fn: SharedFn<<N::Map as EventActionMap<N::Event>>::Action>,
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
            where F: FnMut(<N::Map as EventActionMap<N::Event>>::Action) -> bool
    {
        let node_data_moved = self.self_ptr != self;
        self.self_ptr = self;
        self.action_fn.borrow_mut().set_fn(&mut f);

        if node_data_moved {
            self.toplevel.update_window();
        }

        let root_widget_hints = WidgetHints {
            node_span: NodeSpan::new(.., ..),
            ..WidgetHints::default()
        };
        let mut children_updated = false;
        NativeNodeProcessor::<_, <N::Map as EventActionMap<N::Event>>::Action> {
            parent: &mut self.toplevel,
            action_fn: &self.action_fn,
            bottom_window: None,
            children_updated: &mut children_updated
        }.add_child_mut(ChildId::Num(0), root_widget_hints, &mut self.root)?;

        // Modifying the size bounds of windows inside of the toplevel window doesn't trigger a size
        // bounds check in the toplevel. This forces that check.
        self.toplevel.bound_to_size_bounds();

        for msg in msg::queue::thread_wait_queue() {
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
    bottom_window: Option<WindowRef>,
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

impl<'a, P, A> Drop for NativeNodeProcessor<'a, P, A> {
    fn drop(&mut self) {
        if let Some(bottom_window) = self.bottom_window {
            HOLDING_PARENT.with(|hp| {
                for window in bottom_window.windows_below() {
                    hp.add_child_window(&window);
                }
            });
        }
    }
}

impl<'a, P, A, C> NodeProcessorGridMut<C> for NativeNodeProcessor<'a, P, A>
        where C: Node
{
    default fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, _: &'b mut C) -> Result<(), !> {
        panic!("This function should never be called directly, but instead a specialized version should be")
    }
}

impl<'a, P, A, B, S> NodeProcessorGridMut<TextButton<B, S>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
              B: EventActionMap<MouseEvent, Action = A>,
              S: AsRef<str>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, button: &'b mut TextButton<B, S>) -> Result<(), !> {
        button.wrapper().update_window();

        if self.bottom_window.is_none() {
            self.bottom_window = Some(button.wrapper().window_ref());
        }

        if button.wrapper().needs_widget_update() {
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
        label.wrapper().update_window();

        if self.bottom_window.is_none() {
            self.bottom_window = Some(label.wrapper().window_ref());
        }

        if label.wrapper().needs_widget_update() {
            *self.children_updated = true;
            label.wrapper_mut().update_widget();
            self.parent.add_child_node(label.wrapper_mut());
        }
        Ok(())
    }
}

impl<'a, P, A, I> NodeProcessorGridMut<Group<I>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
      for<'b> I: Parent<!> +
                 ParentMut<NativeNodeProcessor<'b, GroupAdder, A>, ChildAction = A> +
                 Parent<GridWidgetProcessor<'b>> +
                 Parent<EngineTypeHarvester<'b>>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, group: &'b mut Group<I>) -> Result<(), !> {
        let group_wrapper = group.wrapper_mut();
        group_wrapper.update_window();

        if self.bottom_window.is_none() {
            self.bottom_window = Some(group_wrapper.window_ref());
        }

        if group_wrapper.needs_widget_update() {
            *self.children_updated = true;
            self.parent.add_child_node(group_wrapper);
        }

        let mut adder = group_wrapper.get_adder();
        let mut children_updated = false;
        {
            let child_processor = NativeNodeProcessor {
                parent: &mut adder,
                action_fn: self.action_fn,
                bottom_window: None,
                children_updated: &mut children_updated
            };
            group_wrapper.content_data_mut().children_mut(child_processor)?;
        }

        if children_updated {
            group_wrapper.update_widget();
        }
        Ok(())
    }
}

impl<'a, P, A, S, I> NodeProcessorGridMut<LabelGroup<S, I>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
              S: AsRef<str>,
      for<'b> I: Parent<!> +
                 ParentMut<NativeNodeProcessor<'b, GroupAdder, A>, ChildAction = A> +
                 Parent<GridWidgetProcessor<'b>> +
                 Parent<EngineTypeHarvester<'b>>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, group: &'b mut LabelGroup<S, I>) -> Result<(), !> {
        let group_wrapper = group.wrapper_mut();
        group_wrapper.update_window();

        if self.bottom_window.is_none() {
            self.bottom_window = Some(group_wrapper.window_ref());
        }

        if group_wrapper.needs_widget_update() {
            group_wrapper.groupbox_window_ref().move_to_top();
            *self.children_updated = true;
            self.parent.add_child_node(group_wrapper);

            let mut adder = group_wrapper.get_adder();
            let mut children_updated = false;
            {
                let child_processor = NativeNodeProcessor {
                    parent: &mut adder,
                    action_fn: self.action_fn,
                    bottom_window: Some(group_wrapper.groupbox_window_ref()),
                    children_updated: &mut children_updated
                };
                group_wrapper.content_data_mut().children.children_mut(child_processor)?;
            }

            group_wrapper.update_widget();
        }

        Ok(())
    }
}

impl<'a, P, A> NodeProcessorGridMut<Progbar> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, progress_bar: &'b mut Progbar) -> Result<(), !> {
        progress_bar.wrapper().update_window();

        if self.bottom_window.is_none() {
            self.bottom_window = Some(progress_bar.wrapper().window_ref());
        }

        if progress_bar.wrapper().needs_widget_update() {
            *self.children_updated = true;
            progress_bar.wrapper_mut().update_widget();
            self.parent.add_child_node(progress_bar.wrapper_mut());
        }
        Ok(())
    }
}

impl<'a, P, A, C> NodeProcessorGridMut<Slider<C>> for NativeNodeProcessor<'a, P, A>
        where P: ParentChildAdder,
              C: EventActionMap<RangeEvent, Action = A>
{
    fn add_child_mut<'b>(&'b mut self, _: ChildId, _: WidgetHints, slider: &'b mut Slider<C>) -> Result<(), !> {
        slider.wrapper().update_window();

        if self.bottom_window.is_none() {
            self.bottom_window = Some(slider.wrapper().window_ref());
        }

        if slider.wrapper().needs_widget_update() {
            *self.children_updated = true;
            slider.wrapper_mut().update_widget(self.action_fn);
            self.parent.add_child_node(slider.wrapper_mut());
        }
        Ok(())
    }
}


pub struct NativeWrapperRegistry;
impl<B: EventActionMap<MouseEvent>, S: AsRef<str>> NodeDataRegistry<TextButton<B, S>> for NativeWrapperRegistry {
    type NodeDataWrapper = TextButtonNodeData<B, S>;
}
impl<S: AsRef<str>> NodeDataRegistry<TextLabel<S>> for NativeWrapperRegistry {
    type NodeDataWrapper = TextLabelNodeData<S>;
}
impl<I> NodeDataRegistry<Group<I>> for NativeWrapperRegistry
        where for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>> + Parent<EngineTypeHarvester<'a>>
{
    type NodeDataWrapper = GroupNodeData<I>;
}
impl<S, I> NodeDataRegistry<LabelGroup<S, I>> for NativeWrapperRegistry
        where S: AsRef<str>,
              for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>> + Parent<EngineTypeHarvester<'a>>
{
    type NodeDataWrapper = LabelGroupNodeData<S, I>;
}
impl NodeDataRegistry<Progbar> for NativeWrapperRegistry {
    type NodeDataWrapper = ProgbarNodeData;
}
impl<C: EventActionMap<RangeEvent>> NodeDataRegistry<Slider<C>> for NativeWrapperRegistry {
    type NodeDataWrapper = SliderNodeData<C>;
}
