mod subclass;
use self::subclass::*;
pub use self::subclass::GridWidgetProcessor;

use ui::{Parent, Node, NodeDataWrapper, NodeProcessorInit};
use ui::widgets::{ButtonControl, SliderControl};
use ui::widgets::status::{progbar, slider, Orientation};
use ui::hints::{GridSize, TrackHints};

use dww::*;
use dle::{Tr, GridEngine};
use dct::geometry::{OffsetRect, SizeBounds};

use std::mem;
use std::rc::Rc;
use std::cell::RefCell;
use std::borrow::Borrow;

macro_rules! impl_node_data_wrapper {
    (
        $name:ident$(<$inner_ty:ident>)*;
        $(where $($where_ty:ty: $($(for<$($lt:tt),+>)* trait $constraint:path)|+),+;)*
        $(impl where $($impl_where_ty:ty: $($(for<$($impl_lt:tt),+>)* trait $impl_constraint:path)|+),+;)*
        expr node_data($node_data_in:ident) = $node_data:expr;
        fn from_node_data($fnd_in:ident: $nd_ty:ty) -> UnsafeSubclassWrapper<_, _> $from_node_data:block;
    ) => {
        impl$(<$inner_ty>)* NodeDataWrapper<$nd_ty> for $name$(<$inner_ty>)*
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+,)+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            fn from_node_data($fnd_in: $nd_ty) -> $name$(<$inner_ty>)* {
                init();

                $name {
                    subclass: $from_node_data,
                    needs_update: true
                }
            }

            fn inner(&self) -> &$nd_ty {
                let $node_data_in = self.subclass.data();
                &$node_data
            }
            fn inner_mut(&mut self) -> &mut $nd_ty {
                self.needs_update = true;
                let $node_data_in = self.subclass.data_mut();
                &mut $node_data
            }

            fn unwrap(self) -> $nd_ty {let $node_data_in = self.subclass.unwrap_data(); $node_data}
        }
    };
    (
        $name:ident$(<$inner_ty:ident>)*;
        $(where $($where_ty:ty: $($(for<$($lt:tt),+>)* trait $constraint:path)|+),+;)*
        $(impl where $($impl_where_ty:ty: $($(for<$($impl_lt:tt),+>)* trait $impl_constraint:path)|+),+;)*
    ) => ();
}

macro_rules! subclass_node_data {
    (
        pub struct $name:ident$(<$inner_ty:ident>)*
                $(where $($where_ty:ty: $($(for<$($lt:tt),+>)* trait $constraint:path)|+),+)*
        {
            subclass: $field_ty:ty,
            needs_update: bool
        }

        impl $(where $($impl_where_ty:ty: $($(for<$($impl_lt:tt),+>)* trait $impl_constraint:path)|+),+)* {
            expr abs_size_bounds($asb_in:tt) = $abs_size_bounds:expr;
            $(
                expr node_data($node_data_in:ident) = $node_data:expr;
                fn from_node_data($fnd_in:ident: $nd_ty:ty) -> UnsafeSubclassWrapper<_, _> $from_node_data:block
            )*

            fn update_widget$(<$($uw_gen:ident),+>)*($uw_in:ident: _ $(, $uw_extra:ident: $uw_extra_ty:ty)*)
                    $(where $($uw_where_ty:ty: $($(for<$($uw_lt:tt),+>)* trait $uw_constraint:path)|+),+)*
            {
                $($update_widget:tt)*
            }
        }

        $($rest:tt)*
    ) => {
        pub struct $name$(<$inner_ty>)*
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+),+)*
        {
            subclass: $field_ty,
            needs_update: bool
        }

        impl$(<$inner_ty>)* $name$(<$inner_ty>)*
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+,)+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            #[doc(hidden)]
            #[inline]
            pub fn update_subclass_ptr(&self) {
                self.subclass.update_subclass_ptr();
            }

            #[doc(hidden)]
            pub fn update_widget$(<$($uw_gen),+>)*(&mut self $(, $uw_extra: $uw_extra_ty)*)
                    $(where $($uw_where_ty: $($(for<$($uw_lt),+>)* $uw_constraint +)+),+)*
            {
                self.needs_update = false;
                let $uw_in = &mut self.subclass;
                $($update_widget)*
            }
        }

        impl$(<$inner_ty>)* NativeDataWrapper for $name$(<$inner_ty>)*
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
            fn window_ref(&mut self) -> WindowRef {
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
            fn needs_update(&self) -> bool {
                self.needs_update
            }
        }

        impl_node_data_wrapper!{
            $name$(<$inner_ty>)*;
            $(where $($where_ty: $($(for<$($lt),+>)* trait $constraint)|+),+;)*
            $(impl where $($impl_where_ty: $($(for<$($impl_lt),+>)* trait $impl_constraint)|+),+;)*
            $(
                expr node_data($node_data_in) = $node_data;
                fn from_node_data($fnd_in: $nd_ty) -> UnsafeSubclassWrapper<_, _> $from_node_data;
            )*
        }

        subclass_node_data!{$($rest)*}
    };

    () => ();
}

thread_local!{
    static HOLDING_PARENT: BlankBase = WindowBuilder::default().show_window(false).build_blank();
}
lazy_static!{
    static ref CAPTION_FONT: Font = Font::sys_caption_font();
}

subclass_node_data!{
    pub struct TextButtonNodeData<I>
            where I: trait Borrow<str> | trait ButtonControl
    {
        subclass: UnsafeSubclassWrapper<PushButtonBase<&'static Font>, TextButtonSubclass<I>>,
        needs_update: bool
    }
    impl {
        expr abs_size_bounds(subclass_data) = subclass_data.abs_size_bounds;
        expr node_data(subclass_data) = subclass_data.node_data;

        fn from_node_data(node_data: I) -> UnsafeSubclassWrapper<_, _> {
            HOLDING_PARENT.with(|hp| {
                let button_window = WindowBuilder::default().build_push_button_with_font(hp, &*CAPTION_FONT);
                let subclass = TextButtonSubclass::new(node_data);

                unsafe{ UnsafeSubclassWrapper::new(button_window, subclass) }
            })
        }
        fn update_widget(subclass: _, action_fn: &SharedFn<I::Action>) {
            subclass.set_text_noprefix_fn(|subcl| subcl.data().node_data.borrow());
            subclass.data_mut().action_fn = Some(action_fn.clone());
        }
    }

    pub struct WidgetGroupNodeData<I>
            where I: trait Parent<!>
    {
        subclass: UnsafeSubclassWrapper<BlankBase, WidgetGroupSubclass<I>>,
        needs_update: bool
    }
    impl where I: for<'a> trait Parent<GridWidgetProcessor<'a>> | for<'a> trait Parent<EngineTypeHarvester<'a>> {
        expr abs_size_bounds(subclass_data) = subclass_data.layout_engine.actual_size_bounds();
        fn update_widget(subclass: _) {
            let WidgetGroupSubclass {
                ref mut layout_engine,
                ref mut node_data
            } = *subclass.data_mut();

            // Update the layout engine track hints and size
            node_data.children(EngineTypeHarvester(layout_engine)).ok();

            layout_engine.update_engine(&mut ParentContainer(node_data)).ok();
        }
    }

    pub struct TextLabelNodeData<S>
            where S: trait AsRef<str>
    {
        subclass: UnsafeSubclassWrapper<TextLabelBase<&'static Font>, TextLabelSubclass<S>>,
        needs_update: bool
    }
    impl {
        expr abs_size_bounds(subclass_data) = subclass_data.abs_size_bounds;
        expr node_data(subclass_data) = subclass_data.text;

        fn from_node_data(text: S) -> UnsafeSubclassWrapper<_, _> {
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

    pub struct ProgressBarNodeData {
        subclass: UnsafeSubclassWrapper<ProgressBarBase, ProgressBarSubclass>,
        needs_update: bool
    }
    impl {
        expr abs_size_bounds(_) = SizeBounds::default();
        expr node_data(subclass_data) = subclass_data.status;

        fn from_node_data(status: progbar::Status) -> UnsafeSubclassWrapper<_, _> {
            HOLDING_PARENT.with(|hp| {
                let progbar_window = WindowBuilder::default().build_progress_bar(hp);
                let subclass = ProgressBarSubclass::new(status);

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
                progbar::Completion::Frac(prog) => {
                    if subclass.is_marquee() {
                        subclass.set_marquee(false);
                    }
                    subclass.set_progress((prog * 100.0) as u16);
                }
                progbar::Completion::Working if !subclass.is_marquee() => subclass.set_marquee(true),
                _ => ()
            }
        }
    }

    pub struct SliderNodeData<C>
            where C: trait SliderControl
    {
        subclass: UnsafeSubclassWrapper<BlankBase, SliderSubclass<C>>,
        needs_update: bool
    }
    impl {
        expr abs_size_bounds(_) = SizeBounds::default();
        expr node_data(subclass_data) = subclass_data.control;

        fn from_node_data(control: C) -> UnsafeSubclassWrapper<_, _> {
            let container_window = WindowBuilder::default().build_blank();
            let subclass = SliderSubclass::new(control);

            let mut window = unsafe{ UnsafeSubclassWrapper::new(container_window, subclass) };
            window.data_mut().slider_window = WindowBuilder::default().build_trackbar(&window);
            window.add_child_window(&window.data().slider_window);
            window
        }
        fn update_widget(subclass: _, action_fn: &SharedFn<C::Action>) {
            subclass.data_mut().action_fn = Some(action_fn.clone());

            let status = subclass.data().control.status();
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
                slider::TickPosition::BottomRight => TickPosition::BottomRight,
                slider::TickPosition::TopLeft => TickPosition::TopLeft,
                slider::TickPosition::Both => TickPosition::Both,
                slider::TickPosition::None => TickPosition::None
            };
            slider_window.set_tick_position(tick_position);
        }
    }
}

impl<I> NodeDataWrapper<I> for WidgetGroupNodeData<I>
        where for<'a> I: Parent<!>
{
    fn from_node_data(node_data: I) -> Self {
        HOLDING_PARENT.with(|hp| {
            let mut wrapper_window = WindowBuilder::default().show_window(false).build_blank();
            hp.add_child_window(&wrapper_window);
            wrapper_window.show(true);
            let subclass = WidgetGroupSubclass::new(node_data);

            WidgetGroupNodeData {
                subclass: unsafe{ UnsafeSubclassWrapper::new(wrapper_window, subclass) },
                needs_update: true
            }
        })
    }

    fn inner(&self) -> &I {
        &self.subclass.data().node_data
    }

    fn inner_mut(&mut self) -> &mut I {
        self.needs_update = true;
        &mut self.subclass.data_mut().node_data
    }

    fn unwrap(self) -> I {
        self.subclass.unwrap_data().node_data
    }
}

impl<I> ParentDataWrapper for WidgetGroupNodeData<I>
        where for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>>
{
    type Adder = WidgetGroupAdder;
    fn get_adder(&mut self) -> WidgetGroupAdder {
        WidgetGroupAdder(self.subclass.parent_ref())
    }
}

pub struct WidgetGroupAdder(ParentRef);

impl ParentChildAdder for WidgetGroupAdder {
    fn add_child_node<W>(&mut self, child: &mut W)
            where W: NativeDataWrapper
    {
        if child.window_ref().get_parent() != Some(self.0.parent_ref()) {
            self.0.add_child_window(&mut child.window_ref());
        }
    }
}


type ToplevelWindowBase = OverlapWrapper<BlankBase>;
pub struct ToplevelWindow( UnsafeSubclassWrapper<ToplevelWindowBase, ToplevelSubclass> );

impl ToplevelWindow {
    pub unsafe fn new<'a>(window: ToplevelWindowBase, node_ref: UnsafeSubclassRef<'a, DerinMsg>) -> ToplevelWindow {
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

    pub fn update_subclass_ptr(&self) {
        self.0.update_subclass_ptr()
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
    type Inner = ();
    type Action = !;

    fn type_name(&self) -> &'static str {""}
    fn wrapper(&self) -> &Self {self}
    fn wrapper_mut(&mut self) -> &mut Self {self}
}

impl NodeDataWrapper<()> for ToplevelWindow {
    fn from_node_data(_: ()) -> Self   {panic!("Shouldn't be called")}
    fn inner(&self) -> &()             {panic!("Shouldn't be called")}
    fn inner_mut(&mut self) -> &mut () {panic!("Shouldn't be called")}
    fn unwrap(self) -> ()              {panic!("Shouldn't be called")}
}


pub trait NativeDataWrapper {
    fn abs_size_bounds(&self) -> SizeBounds;
    fn set_rect(&mut self, OffsetRect);
    fn window_ref(&mut self) -> WindowRef;
    fn unsafe_subclass_ref(&mut self) -> UnsafeSubclassRef<DerinMsg>;
    fn post_user_msg(&self, DerinMsg);
    fn needs_update(&self) -> bool;
}

impl NativeDataWrapper for ! {
    fn abs_size_bounds(&self) -> SizeBounds {match self {}}
    fn set_rect(&mut self, _: OffsetRect) {match self {}}
    fn window_ref(&mut self) -> WindowRef {match self {}}
    fn unsafe_subclass_ref(&mut self) -> UnsafeSubclassRef<DerinMsg> {match self {}}
    fn post_user_msg(&self, _: DerinMsg) {}
    fn needs_update(&self) -> bool {match self {}}
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
