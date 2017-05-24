mod subclass;
use self::subclass::*;
pub use self::subclass::GridWidgetProcessor;

use ui::{Parent, ParentMut, Node, NodeDataWrapper, NodeProcessorInit, EventActionMap};
use ui::widgets::{MouseEvent, RangeEvent};
use ui::widgets::content::{SliderStatus, ProgbarStatus, LabelGroupContents,  Completion, Orientation, TickPosition as SliderTickPosition};
use ui::hints::{GridSize, TrackHints};

use dww::window::*;
use dww::window::refs::*;
use dww::window::wrappers::*;
use dww::gdi::text::Font;
use dle::GridEngine;
use dct::geometry::OffsetRect;
use dct::hints::{Tr, SizeBounds};

use std::mem;
use std::rc::Rc;
use std::cell::RefCell;
use std::marker::PhantomData;

static EMPTY: () = ();
static mut EMPTY_MUT: () = ();

macro_rules! subclass_node_data {
    (
        pub struct $name:ident$(<$inner_ty:ident>)*
                $(where $($where_ty:ty: $($(for<$($lt:tt),+>)* trait $constraint:path)|+),+)*
        {
            subclass: $field_ty:ty,
            needs_widget_update: bool
        }

        impl $(where $($impl_where_ty:ty: $($(for<$($impl_lt:tt),+>)* trait $impl_constraint:path)|+),+)* {
            expr abs_size_bounds($asb_in:tt) = $abs_size_bounds:expr;
            $(
                expr event_map($event_map_in:tt) = $event_map:expr;
                $(expr event_map_mut($event_map_mut_in:tt) = $event_map_mut:expr;)*
            )*
            expr content_data($content_data_in:tt) = $content_data:expr;
            fn from_node_data($eam_ident:tt: $eam_ty:ty, $wd_ident:ident: $wd_ty:ty) -> UnsafeSubclassWrapper<_, _> $from_node_data:block

            fn update_widget$(<$($uw_gen:ident),+>)*($uw_in:tt: _ $(, $uw_extra:tt: $uw_extra_ty:ty)*)
                    $(where $($uw_where_ty:ty: $($(for<$($uw_lt:tt),+>)* trait $uw_constraint:path)|+),+)*
            {
                $($update_widget:tt)*
            }
        }

        $($rest:tt)*
    ) => {
        pub struct $name<$($inner_ty),*>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+),+)*
        {
            subclass: $field_ty,
            needs_widget_update: bool
        }

        impl<$($inner_ty),*> $name<$($inner_ty),*>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+,)+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            #[doc(hidden)]
            #[inline]
            pub fn update_window(&self) {
                self.subclass.move_to_top();
                self.subclass.update_subclass_ptr();
            }

            #[doc(hidden)]
            pub fn update_widget$(<$($uw_gen),+>)*(&mut self $(, $uw_extra: $uw_extra_ty)*)
                    $(where $($uw_where_ty: $($(for<$($uw_lt),+>)* $uw_constraint +)+),+)*
            {
                self.needs_widget_update = false;
                let $uw_in = &mut self.subclass;
                $($update_widget)*
            }
        }

        impl<$($inner_ty),*> NativeDataWrapper for $name<$($inner_ty),*>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+,)+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            #[inline]
            fn abs_size_bounds(&self) -> SizeBounds {
                let $asb_in = self.subclass.data();
                $abs_size_bounds
            }

            #[inline]
            fn set_rect(&mut self, rect: OffsetRect) {
                self.subclass.set_rect(rect);
            }

            #[inline]
            fn window_ref(&self) -> WindowRef {
                self.subclass.window_ref()
            }

            #[inline]
            fn unsafe_subclass_ref(&mut self) -> UnsafeSubclassRef<DerinMsg> {
                self.subclass.unsafe_subclass_ref()
            }

            #[inline]
            fn post_user_msg(&self, msg: DerinMsg) {
                self.subclass.post_user_msg(msg);
            }

            #[inline]
            fn needs_widget_update(&self) -> bool {
                self.needs_widget_update
            }
        }

        impl<$($inner_ty,)*> NodeDataWrapper<$eam_ty> for $name<$($inner_ty),*>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+,)+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            type ContentData = $wd_ty;

            fn from_node_data($eam_ident: $eam_ty, $wd_ident: $wd_ty) -> $name<$($inner_ty),*> {
                $name {
                    subclass: $from_node_data,
                    needs_widget_update: true
                }
            }

            fn event_map(&self) -> &$eam_ty {
                if_tokens!{($($event_map_in)*) {$(
                    let $event_map_in = self.subclass.data();
                    &$event_map
                )*} else {
                    &EMPTY
                }}
            }

            fn event_map_mut(&mut self) -> &mut $eam_ty {
                self.needs_widget_update = true;
                if_tokens!{($($event_map_in)*) {$(
                    if_tokens!{($($event_map_mut_in)*) {$(
                        let $event_map_mut_in = self.subclass.data_mut();
                        &mut $event_map_mut
                    )*} else {
                        let $event_map_in = self.subclass.data_mut();
                        &mut $event_map
                    }}
                )*} else {
                    unsafe{ &mut EMPTY_MUT }
                }}
            }

            fn content_data(&self) -> &$wd_ty {
                let $content_data_in = self.subclass.data();
                &$content_data
            }
            fn content_data_mut(&mut self) -> &mut $wd_ty {
                self.needs_widget_update = true;
                let $content_data_in = self.subclass.data_mut();
                &mut $content_data
            }

            fn unwrap(self) -> ($eam_ty, $wd_ty) {
                let $content_data_in = self.subclass.unwrap_data();
                if_tokens!{($($event_map)*) {
                    ($($event_map)*, $content_data)
                } else {
                    ((), $content_data)
                }}
            }
        }

        subclass_node_data!{$($rest)*}
    };

    () => ();
}

thread_local!{
    pub static HOLDING_PARENT: BlankBase = WindowBuilder::default().show_window(false).build_blank();
}
lazy_static!{
    static ref CAPTION_FONT: Font = Font::sys_caption_font();
}

subclass_node_data!{
    pub struct TextButtonNodeData<B><S>
            where B: trait EventActionMap<MouseEvent>,
                  S: trait AsRef<str>
    {
        subclass: UnsafeSubclassWrapper<PushButtonBase<&'static Font>, TextButtonSubclass<B, S>>,
        needs_widget_update: bool
    }
    impl {
        expr abs_size_bounds(subclass_data) = subclass_data.abs_size_bounds;
        expr event_map(subclass_data) = subclass_data.button_action_map;
        expr content_data(subclass_data) = subclass_data.text;

        fn from_node_data(button_action_map: B, text: S) -> UnsafeSubclassWrapper<_, _> {
            HOLDING_PARENT.with(|hp| {
                let button_window = WindowBuilder::default().build_push_button_with_font(hp, &*CAPTION_FONT);
                let subclass = TextButtonSubclass::new(button_action_map, text);

                unsafe{ UnsafeSubclassWrapper::new(button_window, subclass) }
            })
        }
        fn update_widget(subclass: _, action_fn: &SharedFn<B::Action>) {
            subclass.set_text_noprefix_fn(|subcl| subcl.data().text.as_ref());
            subclass.data_mut().action_fn = Some(action_fn.clone());
        }
    }

    pub struct GroupNodeData<I>
            where I: trait Parent<!>
    {
        subclass: UnsafeSubclassWrapper<BlankBase, GroupSubclass<I>>,
        needs_widget_update: bool
    }
    impl where I: for<'a> trait Parent<GridWidgetProcessor<'a>> | for<'a> trait Parent<EngineTypeHarvester<'a>> {
        expr abs_size_bounds(subclass_data) = subclass_data.layout_engine.actual_size_bounds();
        expr event_map(_) = *unsafe{ &*(&EMPTY as *const () as *const _) };
        expr event_map_mut(_) = *unsafe{ &mut*(&mut EMPTY_MUT as *mut () as *mut _) };
        expr content_data(subclass_data) = subclass_data.content_data;

        fn from_node_data(_: PhantomData<<I as ParentMut<!>>::ChildAction>, content_data: I) -> UnsafeSubclassWrapper<_, _> {
            HOLDING_PARENT.with(|hp| {
                let mut wrapper_window = WindowBuilder::default().show_window(false).build_blank();
                hp.add_child_window(&wrapper_window);
                wrapper_window.show(true);
                let subclass = GroupSubclass::new(content_data);

                unsafe{ UnsafeSubclassWrapper::new(wrapper_window, subclass) }
            })
        }
        fn update_widget(subclass: _) {
            let GroupSubclass {
                ref mut layout_engine,
                ref mut content_data
            } = *subclass.data_mut();

            // Update the layout engine track hints and size
            content_data.children(EngineTypeHarvester(layout_engine)).ok();

            layout_engine.update_engine(&mut ParentContainer(content_data)).ok();
        }
    }

    pub struct LabelGroupNodeData<S><I>
            where S: trait AsRef<str>,
                  I: trait Parent<!>
    {
        subclass: UnsafeSubclassWrapper<BlankBase, LabelGroupSubclass<S, I>>,
        needs_widget_update: bool
    }
    impl where I: for<'a> trait Parent<GridWidgetProcessor<'a>> | for<'a> trait Parent<EngineTypeHarvester<'a>> {
        expr abs_size_bounds(subclass_data) = subclass_data.layout_engine.actual_size_bounds();
        expr event_map(_) = *unsafe{ &*(&EMPTY as *const () as *const _) };
        expr event_map_mut(_) = *unsafe{ &mut*(&mut EMPTY_MUT as *mut () as *mut _) };
        expr content_data(subclass_data) = subclass_data.contents;

        fn from_node_data(
            _: PhantomData<<I as ParentMut<!>>::ChildAction>,
            content_data: LabelGroupContents<S, I>
        ) -> UnsafeSubclassWrapper<_, _> {
            HOLDING_PARENT.with(|hp| {
                let wrapper_window = WindowBuilder::default().build_blank();
                hp.add_child_window(&wrapper_window);
                let subclass = LabelGroupSubclass::new(content_data, &wrapper_window);

                unsafe{ UnsafeSubclassWrapper::new(wrapper_window, subclass) }
            })
        }
        fn update_widget(subclass: _) {
            let LabelGroupSubclass {
                ref mut layout_engine,
                ref mut contents,
                ref mut groupbox_window
            } = *subclass.data_mut();

            groupbox_window.set_text_noprefix(contents.text.as_ref());

            // Update the layout engine track hints and size
            contents.children.children(EngineTypeHarvester(layout_engine)).ok();

            layout_engine.update_engine(&mut ParentContainer(&mut contents.children)).ok();
        }
    }

    pub struct TextLabelNodeData<S>
            where S: trait AsRef<str>
    {
        subclass: UnsafeSubclassWrapper<TextLabelBase<&'static Font>, TextLabelSubclass<S>>,
        needs_widget_update: bool
    }
    impl {
        expr abs_size_bounds(subclass_data) = subclass_data.abs_size_bounds;
        expr content_data(subclass_data) = subclass_data.text;

        fn from_node_data(_: (), text: S) -> UnsafeSubclassWrapper<_, _> {
            HOLDING_PARENT.with(|hp| {
                let label_window = WindowBuilder::default().build_text_label_with_font(hp, &*CAPTION_FONT);
                let subclass = TextLabelSubclass::new(text);

                unsafe{ UnsafeSubclassWrapper::new(label_window, subclass) }
            })
        }
        fn update_widget(subclass: _) {
            subclass.set_text_noprefix_fn(|subcl| subcl.data().text.as_ref());
        }
    }

    pub struct ProgbarNodeData {
        subclass: UnsafeSubclassWrapper<ProgressBarBase, ProgbarSubclass>,
        needs_widget_update: bool
    }
    impl {
        expr abs_size_bounds(_) = SizeBounds::default();
        expr content_data(subclass_data) = subclass_data.status;

        fn from_node_data(_: (), status: ProgbarStatus) -> UnsafeSubclassWrapper<_, _> {
            HOLDING_PARENT.with(|hp| {
                let progbar_window = WindowBuilder::default().build_progress_bar(hp);
                let subclass = ProgbarSubclass::new(status);

                unsafe{ UnsafeSubclassWrapper::new(progbar_window, subclass) }
            })
        }
        fn update_widget(subclass: _) {
            let status = subclass.data().status;

            match status.orientation {
                Orientation::Horizontal if subclass.is_vertical() => subclass.set_vertical(false),
                Orientation::Vertical  if !subclass.is_vertical() => subclass.set_vertical(true),
                _ => ()
            }
            match status.completion {
                Completion::Frac(prog) => {
                    if subclass.is_marquee() {
                        subclass.set_marquee(false);
                    }
                    subclass.set_progress((prog * 100.0) as u16);
                }
                Completion::Working if !subclass.is_marquee() => subclass.set_marquee(true),
                _ => ()
            }
        }
    }

    pub struct SliderNodeData<C>
            where C: trait EventActionMap<RangeEvent>
    {
        subclass: UnsafeSubclassWrapper<BlankBase, SliderSubclass<C>>,
        needs_widget_update: bool
    }
    impl {
        expr abs_size_bounds(_) = SizeBounds::default();
        expr event_map(subclass_data) = subclass_data.range_action_map;
        expr content_data(subclass_data) = subclass_data.status;

        fn from_node_data(range_action_map: C, status: SliderStatus) -> UnsafeSubclassWrapper<_, _> {
            let container_window = WindowBuilder::default().build_blank();
            let subclass = SliderSubclass::new(range_action_map, status, &container_window);

            unsafe{ UnsafeSubclassWrapper::new(container_window, subclass) }
        }
        fn update_widget(subclass: _, action_fn: &SharedFn<C::Action>) {
            subclass.data_mut().action_fn = Some(action_fn.clone());

            let status = subclass.data().status.clone();
            let slider_window = &mut subclass.data_mut().slider_window;

            slider_window.set_pos(status.position);
            slider_window.set_range(status.range.start, status.range.end);
            slider_window.auto_ticks(status.tick_interval);

            match status.orientation {
                Orientation::Horizontal => slider_window.set_vertical(false),
                Orientation::Vertical   => slider_window.set_vertical(true)
            }

            // Dww tick position and slider tick position are different types, so translate between then.
            // I don't have this type in DCT because then the documentation is uglier.
            let tick_position = match status.tick_position {
                SliderTickPosition::BottomRight => TickPosition::BottomRight,
                SliderTickPosition::TopLeft => TickPosition::TopLeft,
                SliderTickPosition::Both => TickPosition::Both,
                SliderTickPosition::None => TickPosition::None
            };
            slider_window.set_tick_position(tick_position);
        }
    }
}

impl<I> ParentDataWrapper for GroupNodeData<I>
        where for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>>
{
    type Adder = GroupAdder;
    fn get_adder(&mut self) -> GroupAdder {
        GroupAdder(self.subclass.parent_ref())
    }
}

impl<S, I> LabelGroupNodeData<S, I>
        where S: AsRef<str>,
              for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>>
{
    pub(super) fn groupbox_window_ref(&self) -> WindowRef {
        self.subclass.data().groupbox_window.window_ref()
    }
}

impl<S, I> ParentDataWrapper for LabelGroupNodeData<S, I>
        where S: AsRef<str>,
              for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>>
{
    type Adder = GroupAdder;
    fn get_adder(&mut self) -> GroupAdder {
        GroupAdder(self.subclass.parent_ref())
    }
}

pub struct GroupAdder(ParentRef);

impl ParentChildAdder for GroupAdder {
    fn add_child_node<W>(&mut self, child: &mut W)
            where W: NativeDataWrapper
    {
        if child.window_ref().get_parent() != Some(self.0.parent_ref()) {
            self.0.add_child_window(&mut child.window_ref());
        }
    }
}


type ToplevelBaseWindow = OverlapWrapper<BlankBase>;
pub struct ToplevelWindow( UnsafeSubclassWrapper<ToplevelBaseWindow, ToplevelSubclass> );

impl ToplevelWindow {
    pub unsafe fn new<'a>(window: ToplevelBaseWindow, node_ref: UnsafeSubclassRef<'a, DerinMsg>) -> ToplevelWindow {
        // Expand child window lifetime to 'static with transmute. This is safe because the toplevel
        // window struct will only exist for the length of the child, and even if the child is
        // destroyed any messages sent won't be processed.
        let subclass = ToplevelSubclass(mem::transmute(node_ref));
        let window = ToplevelWindow(UnsafeSubclassWrapper::new(window, subclass));
        window.0.add_child_window(&node_ref);
        window
    }

    pub fn bound_to_size_bounds(&mut self) {
        self.0.bound_to_size_bounds()
    }

    pub fn update_window(&self) {
        self.0.update_subclass_ptr();
    }
}

impl ParentChildAdder for ToplevelWindow {
    fn add_child_node<W>(&mut self, child: &mut W)
            where W: NativeDataWrapper
    {
        let mut child_ref = child.unsafe_subclass_ref();

        // Only perform updates if the child isn't already the stored window ref. Otherwise we get
        // flickering.
        if child_ref.window_ref() != self.0.data_mut().0.window_ref() {
            HOLDING_PARENT.with(|hp| {
                // Reset the Toplevel's current child window parent to the holding parent.
                hp.add_child_window(&self.0.data_mut().0);
            });
            self.0.add_child_window(&mut child_ref);

            // Expand the lifetime to 'static with transmute. See creation for reason this is done.
            self.0.data_mut().0 = unsafe{ mem::transmute(child_ref) };
        }
    }
}

impl Node for ToplevelWindow {
    type Wrapper = Self;
    type Map = !;
    type Event = !;

    fn type_name(&self) -> &'static str {""}
    fn wrapper(&self) -> &Self {self}
    fn wrapper_mut(&mut self) -> &mut Self {self}
}

impl NodeDataWrapper<!> for ToplevelWindow {
    type ContentData = !;

    fn from_node_data(never: !, _: !) -> Self {never}

    fn event_map(&self) -> &! {panic!("Shouldn't be called")}
    fn event_map_mut(&mut self) -> &mut ! {panic!("Shouldn't be called")}
    fn content_data(&self) -> &! {panic!("Shouldn't be called")}
    fn content_data_mut(&mut self) -> &mut ! {panic!("Shouldn't be called")}

    fn unwrap(self) -> (!, !) {panic!("Shouldn't be called")}
}


pub trait NativeDataWrapper {
    fn abs_size_bounds(&self) -> SizeBounds;
    fn set_rect(&mut self, OffsetRect);
    fn window_ref(&self) -> WindowRef;
    fn unsafe_subclass_ref(&mut self) -> UnsafeSubclassRef<DerinMsg>;
    fn post_user_msg(&self, DerinMsg);
    fn needs_widget_update(&self) -> bool;
}

impl NativeDataWrapper for ! {
    fn abs_size_bounds(&self) -> SizeBounds {match self {}}
    fn set_rect(&mut self, _: OffsetRect) {match self {}}
    fn window_ref(&self) -> WindowRef {match self {}}
    fn unsafe_subclass_ref(&mut self) -> UnsafeSubclassRef<DerinMsg> {match self {}}
    fn post_user_msg(&self, _: DerinMsg) {}
    fn needs_widget_update(&self) -> bool {match self {}}
}

pub trait ParentDataWrapper {
    type Adder: ParentChildAdder; // ssssSSsssSSsss
    fn get_adder(&mut self) -> Self::Adder;
}

pub trait ParentChildAdder {
    fn add_child_node<W>(&mut self, &mut W)
            where W: NativeDataWrapper;
}

#[derive(Debug, Clone, Copy, UserMsg)]
pub enum DerinMsg {
    SetRect(OffsetRect)
}


pub type SharedFn<A> = Rc<RefCell<ActionFn<A>>>;

pub struct ActionFn<A> {
    func: *mut FnMut(A) -> bool,
    pub continue_loop: bool
}

impl<A> ActionFn<A> {
    pub fn new() -> ActionFn<A> {
        ActionFn {
            func: unsafe{ mem::zeroed() },
            continue_loop: true
        }
    }

    pub fn set_fn(&mut self, f: &mut FnMut(A) -> bool) {
        self.func = unsafe{ mem::transmute(f) };
        self.continue_loop = true;
    }

    pub unsafe fn call_fn(&mut self, action: A) {
        self.continue_loop = (*self.func)(action);
    }

    pub fn clear(&mut self) {
        self.func = unsafe{ mem::zeroed() };
    }
}

pub struct EngineTypeHarvester<'a>( &'a mut GridEngine );

impl<'a> NodeProcessorInit for EngineTypeHarvester<'a> {
    type Error = !;
    type GridProcessor = ();
    fn init_grid<C, R>(self, grid_size: GridSize, col_hints: C, row_hints: R) -> ()
            where C: Iterator<Item = TrackHints>,
                  R: Iterator<Item = TrackHints>
    {
        self.0.set_grid_size(grid_size);
        for (i, hint) in col_hints.enumerate().take(grid_size.x as usize) {
            self.0.set_col_hints(i as Tr, hint);
        }
        for (i, hint) in row_hints.enumerate().take(grid_size.y as usize) {
            self.0.set_row_hints(i as Tr, hint);
        }
    }
}
