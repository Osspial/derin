use super::toggle_cell::ToggleCell;

use ui::{Control, Parent, Node, ChildId, NodeProcessor, NodeProcessorAT, NodeDataWrapper};
use dww::*;
use dle::{Tr, Container, LayoutEngine, WidgetData, WidgetConstraintSolver, SolveError};
use dle::hints::{WidgetHints, GridSize, TrackHints};
use dct::events::{MouseEvent};
use dct::geometry::{OriginRect, OffsetRect, SizeBounds};
use void::Void;

use std::mem;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ToplevelWindowBase = OverlapWrapper<BlankBase>;

macro_rules! impl_node_data_wrapper {
    (
        $name:ident<$inner_ty:ident>;
        $(where $($where_ty:ty: $($(for<$($lt:tt),+>)* trait $constraint:path)|+),+)*;
        $(impl where $($impl_where_ty:ty: $($(for<$($impl_lt:tt),+>)* trait $impl_constraint:path)|+),+)*;
        fn from_node_data($fnd_in:ident: _) -> UnsafeSubclassWrapper<_, _> $from_node_data:block;
        expr node_data($node_data_in:ident) = $node_data:expr;
    ) => {
        impl<$inner_ty> NodeDataWrapper<$inner_ty> for $name<$inner_ty>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+),+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            fn from_node_data($fnd_in: $inner_ty) -> $name<$inner_ty> {
                enable_visual_styles();

                $name {
                    subclass: $from_node_data,
                    needs_update: true
                }
            }

            fn inner(&self) -> &$inner_ty {let $node_data_in = &self.subclass; &$node_data}
            fn inner_mut(&mut self) -> &mut $inner_ty {
                self.needs_update = true;
                let $node_data_in = &mut self.subclass;
                &mut $node_data
            }

            fn unwrap(self) -> $inner_ty {let $node_data_in = self.subclass; $node_data}
        }
    };
    ($($t:tt)*) => ();
}

macro_rules! subclass_node_data {
    (
        pub struct $name:ident<$inner_ty:ident>
                $(where $($where_ty:ty: $($(for<$($lt:tt),+>)* trait $constraint:path)|+),+)*
        {
            subclass: $field_ty:ty,
            needs_update: bool
        }

        impl $(where $($impl_where_ty:ty: $($(for<$($impl_lt:tt),+>)* trait $impl_constraint:path)|+),+)* {
            expr widget_data($widget_data_in:ident) = $widget_data:expr;
            $(
                expr node_data($node_data_in:ident) = $node_data:expr;
                fn from_node_data($fnd_in:ident: _) -> UnsafeSubclassWrapper<_, _> $from_node_data:block
            )*

            fn update_widget$(<$($uw_gen:ident),+>)*($uw_in:ident: _, $hints:ident: WidgetHints $(, $uw_extra:ident: $uw_extra_ty:ty)*)
                    $(where $($uw_where_ty:ty: $($(for<$($uw_lt:tt),+>)* trait $uw_constraint:path)|+),+)*
            {
                $($update_widget:tt)*
            }
        }

        $($rest:tt)*
    ) => {
        pub struct $name<$inner_ty>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+),+)*
        {
            subclass: $field_ty,
            needs_update: bool
        }

        impl<$inner_ty> $name<$inner_ty>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+,)+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            #[doc(hidden)]
            #[inline]
            pub fn update_subclass_ptr(&self) {
                self.subclass.update_subclass_ptr();
            }

            #[doc(hidden)]
            pub fn update_widget$(<$($uw_gen),+>)*(&mut self, $hints: WidgetHints $(, $uw_extra: $uw_extra_ty)*)
                    $(where $($uw_where_ty: $($(for<$($uw_lt),+>)* $uw_constraint +)+),+)*
            {
                self.needs_update = false;
                let $uw_in = &mut self.subclass;
                $($update_widget)*
            }
        }

        impl<$inner_ty> NativeDataWrapper for $name<$inner_ty>
                $(where $($where_ty: $($(for<$($lt),+>)* $constraint +)+,)+)*
                $($($impl_where_ty: $($(for<$($impl_lt),+>)* $impl_constraint +)+),+)*
        {
            #[inline]
            fn size_bounds(&self) -> SizeBounds {
                self.subclass.size_bounds()
            }

            #[inline]
            fn set_rect(&self, rect: OffsetRect) {
                self.subclass.set_rect(rect);
            }

            #[inline]
            fn get_widget_data(&self) -> WidgetData {
                let $widget_data_in = &self.subclass;
                $widget_data
            }

            #[inline]
            fn window_ref(&self) -> WindowRef {
                self.subclass.window_ref()
            }

            #[inline]
            fn unsafe_subclass_ref(&self) -> UnsafeSubclassRef<DerinMsg> {
                self.subclass.unsafe_subclass_ref()
            }

            #[inline]
            fn needs_update(&self) -> bool {
                self.needs_update
            }
        }

        impl_node_data_wrapper!{
            $name<$inner_ty>;
            $(where $($where_ty: $($(for<$($lt),+>)* trait $constraint)|+),+)*;
            $(impl where $($impl_where_ty:ty: $($(for<$($impl_lt:tt),+>)* trait $impl_constraint:path)|+),+)*;
            $(
                fn from_node_data($fnd_in: _) -> UnsafeSubclassWrapper<_, _> $from_node_data;
                expr node_data($node_data_in) = $node_data;
            )*
        }

        subclass_node_data!{$($rest)*}
    };

    () => ();
}

subclass_node_data!{
    pub struct TextButtonNodeData<I>
            where I: trait AsRef<str> | trait Control
    {
        subclass: UnsafeSubclassWrapper<PushButtonBase, TextButtonSubclass<I>>,
        needs_update: bool
    }
    impl {
        expr widget_data(subclass) = subclass.data.mutable_data.borrow().widget_data;
        expr node_data(subclass) = subclass.data.node_data;

        fn from_node_data(node_data: _) -> UnsafeSubclassWrapper<_, _> {
            let button_window = WindowBuilder::default().show_window(false).build_push_button();
            let subclass = TextButtonSubclass::new(node_data);

            unsafe{ UnsafeSubclassWrapper::new(button_window, subclass) }
        }
        fn update_widget(subclass: _, hints: WidgetHints, action_fn: &SharedFn<I::Action>) {
            subclass.data.mutable_data.get_mut().widget_data.widget_hints = hints;
            subclass.set_text(subclass.data.node_data.as_ref());
            subclass.data.action_fn = Some(action_fn.clone());
        }
    }

    pub struct WidgetGroupNodeData<I>
            where I: trait Parent<()>
    {
        subclass: UnsafeSubclassWrapper<BlankBase, WidgetGroupSubclass<I>>,
        needs_update: bool
    }
    impl where I: for<'a> trait Parent<ConstraintSolverTraverser<'a>> {
        expr widget_data(subclass) = subclass.data.widget_data;
        fn update_widget<C, R>(subclass: _, hints: WidgetHints, grid_size: GridSize, col_hints: C, row_hints: R)
                where C: trait Iterator<Item=TrackHints>,
                      R: trait Iterator<Item=TrackHints>
        {
            {
                let mutable_data = subclass.data.mutable_data.get_mut();
                let layout_engine = &mut mutable_data.layout_engine;

                layout_engine.set_grid_size(grid_size);
                for (i, hint) in col_hints.enumerate() {
                    layout_engine.set_col_hints(i as Tr, hint);
                }
                for (i, hint) in row_hints.enumerate() {
                    layout_engine.set_row_hints(i as Tr, hint);
                }
                layout_engine.update_engine(&mut mutable_data.node_data).ok();
            }
            subclass.data.widget_data.widget_hints = hints;

            subclass.data.mutable_data.intern_mode();
        }
    }

    pub struct TextLabelNodeData<S>
            where S: trait AsRef<str>
    {
        subclass: UnsafeSubclassWrapper<TextLabelBase, TextLabelSubclass<S>>,
        needs_update: bool
    }
    impl {
        expr widget_data(subclass) = subclass.data.widget_data.get();
        expr node_data(subclass) = subclass.data.text;

        fn from_node_data(text: _) -> UnsafeSubclassWrapper<_, _> {
            let label_window = WindowBuilder::default().show_window(false).build_text_label();
            let subclass = TextLabelSubclass::new(text);

            unsafe{ UnsafeSubclassWrapper::new(label_window, subclass) }
        }
        fn update_widget(subclass: _, hints: WidgetHints) {
            subclass.set_text(subclass.data.text.as_ref());
            subclass.data.widget_data.set(WidgetData {
                widget_hints: hints,
                ..subclass.data.widget_data.get()
            });
        }
    }
}

impl<I> NodeDataWrapper<I> for WidgetGroupNodeData<I>
        where for<'a> I: Parent<()>
{
    fn from_node_data(node_data: I) -> Self {
        let wrapper_window = WindowBuilder::default().show_window(false).build_blank();
        let subclass = WidgetGroupSubclass::new(node_data);

        WidgetGroupNodeData {
            subclass: unsafe{ UnsafeSubclassWrapper::new(wrapper_window, subclass) },
            needs_update: true
        }
    }

    fn inner(&self) -> &I {
        &self.subclass.data.mutable_data.get().node_data.0
    }

    fn inner_mut(&mut self) -> &mut I {
        self.needs_update = true;
        &mut self.subclass.data.mutable_data.get_mut().node_data.0
    }

    fn unwrap(self) -> I {
        self.subclass.data.mutable_data.into_inner().node_data.0
    }
}

impl<I> ParentDataWrapper for WidgetGroupNodeData<I>
        where for<'a> I: Parent<()> + Parent<ConstraintSolverTraverser<'a>>
{
    type Adder = WidgetGroupAdder;
    fn get_adder(&self) -> WidgetGroupAdder {
        WidgetGroupAdder(self.subclass.parent_ref())
    }
}

pub struct WidgetGroupAdder(ParentRef);

impl ParentChildAdder for WidgetGroupAdder {
    fn add_child_node<N>(&mut self, child: &N)
            where N: Node,
                  N::Wrapper: NativeDataWrapper
    {
        self.0.add_child_window(child.wrapper().window_ref());
    }
}


pub struct ToplevelWindow( UnsafeSubclassWrapper<ToplevelWindowBase, ToplevelSubclass> );

impl ToplevelWindow {
    pub fn new<N>(window: ToplevelWindowBase, node: &N) -> ToplevelWindow
            where N: Node,
                  N::Wrapper: NativeDataWrapper
    {
        ToplevelWindow(unsafe{ UnsafeSubclassWrapper::new(window, ToplevelSubclass(node.wrapper().unsafe_subclass_ref())) })
    }

    pub fn bound_to_size_bounds(&self) {
        self.0.bound_to_size_bounds()
    }

    pub fn update_subclass_ptr(&self) {
        self.0.update_subclass_ptr()
    }
}

impl ParentChildAdder for ToplevelWindow {
    fn add_child_node<N>(&mut self, child: &N)
            where N: Node,
                  N::Wrapper: NativeDataWrapper
    {
        // Orphan the Toplevel's current child window, and replace it with the new child passed in
        // through the `child` field.
        self.0.data.0.orphan();
        self.0.data.0 = child.wrapper().unsafe_subclass_ref();

        self.0.add_child_window(self.0.data.0);
    }
}

impl Node for ToplevelWindow {
    type Wrapper = Self;
    type Inner = ();
    type Action = Void;

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




enum ButtonState {
    Released,
    Pressed,
    DoublePressed
}

impl Default for ButtonState {
    #[inline]
    fn default() -> ButtonState {
        ButtonState::Released
    }
}

struct TextButtonSubclass<I: AsRef<str> + Control> {
    node_data: I,
    action_fn: Option<SharedFn<I::Action>>,
    mutable_data: RefCell<TBSMut>
}

#[derive(Default)]
struct TBSMut {
    widget_data: WidgetData,
    button_state: ButtonState
}

impl<I: AsRef<str> + Control> TextButtonSubclass<I> {
    #[inline]
    fn new(node_data: I) -> TextButtonSubclass<I> {
        TextButtonSubclass {
            node_data: node_data,
            action_fn: None,
            mutable_data: RefCell::new(TBSMut::default())
        }
    }
}

impl<B, I> Subclass<B> for TextButtonSubclass<I>
        where B: ButtonWindow,
              I: AsRef<str> + Control
{
    type UserMsg = DerinMsg;
    fn subclass_proc(&self, window: &ProcWindowRef<B, Self>, mut msg: Msg<DerinMsg>) -> i64 {
        let ret = window.default_window_proc(&mut msg);

        match msg {
            Msg::Wm(wm) => match wm {
                Wm::MouseDown(_, _) => self.mutable_data.borrow_mut().button_state = ButtonState::Pressed,
                Wm::MouseDoubleDown(_, _) => self.mutable_data.borrow_mut().button_state = ButtonState::DoublePressed,
                Wm::MouseUp(button, _) => {
                    let mut mutable_data = self.mutable_data.borrow_mut();
                    let action_opt = match mutable_data.button_state {
                        ButtonState::Pressed       => self.node_data.on_mouse_event(MouseEvent::Clicked(button)),
                        ButtonState::DoublePressed => self.node_data.on_mouse_event(MouseEvent::DoubleClicked(button)),
                        ButtonState::Released      => None
                    };
                    if let Some(action) = action_opt {
                        unsafe{ self.action_fn.as_ref().expect("No Action Function").borrow_mut().call_fn(action) };
                    }

                    mutable_data.button_state = ButtonState::Released;
                },
                Wm::SetText(_) => self.mutable_data.borrow_mut().widget_data.abs_size_bounds.min = window.get_ideal_size(),
                Wm::GetSizeBounds(size_bounds) => size_bounds.min = window.get_ideal_size(),
                Wm::Size(_) => window.show(true),
                _ => ()
            },
            Msg::User(DerinMsg::SetRectPropagate(rect)) => window.set_rect(rect),
            _ => ()
        }
        ret
    }
}


struct WidgetGroupSubclass<I: Parent<()>> {
    mutable_data: ToggleCell<WGSMut<I>>,
    widget_data: WidgetData
}

struct WGSMut<I: Parent<()>> {
    node_data: ParentContainer<I>,
    layout_engine: LayoutEngine
}

impl<I: Parent<()>> WidgetGroupSubclass<I> {
    #[inline]
    fn new(node_data: I) -> WidgetGroupSubclass<I> {
        WidgetGroupSubclass {
            mutable_data: ToggleCell::new(WGSMut {
                node_data: ParentContainer(node_data),
                layout_engine: LayoutEngine::new()
            }),
            widget_data: WidgetData::new()
        }
    }
}

impl<P, I> Subclass<P> for WidgetGroupSubclass<I>
        where P: ParentWindow,
              I: Parent<()>
{
    type UserMsg = DerinMsg;
    default fn subclass_proc(&self, _: &ProcWindowRef<P, Self>, _: Msg<DerinMsg>) -> i64 {
        panic!("Should never be called; just here to hide ConstraintSolverTraverser type from public exposure")
    }
}

impl<P, I> Subclass<P> for WidgetGroupSubclass<I>
        where P: ParentWindow,
      for<'a> I: Parent<()> + Parent<ConstraintSolverTraverser<'a>>
{
    fn subclass_proc(&self, window: &ProcWindowRef<P, Self>, mut msg: Msg<DerinMsg>) -> i64 {
        match msg {
            Msg::Wm(wm) => match wm {
                Wm::GetSizeBounds(size_bounds) => {
                    *size_bounds = self.mutable_data.borrow_mut().layout_engine.actual_size_bounds();
                    0
                },
                Wm::Size(_) => {window.show(true); 0},
                wm => window.default_window_proc(&mut Msg::Wm(wm))
            },
            Msg::User(DerinMsg::SetRectPropagate(rect)) => {
                let mut mutable_data = self.mutable_data.borrow_mut();
                let WGSMut {
                    ref mut node_data,
                    ref mut layout_engine
                } = *mutable_data;

                layout_engine.desired_size = OriginRect::from(rect);
                layout_engine.update_engine(node_data).ok();
                window.set_rect(rect);
                0
            },
            _ => window.default_window_proc(&mut msg)
        }
    }
}


struct TextLabelSubclass<S: AsRef<str>> {
    text: S,
    widget_data: Cell<WidgetData>
}

impl<S: AsRef<str>> TextLabelSubclass<S> {
    #[inline]
    fn new(text: S) -> TextLabelSubclass<S> {
        TextLabelSubclass {
            text: text,
            widget_data: Cell::default()
        }
    }
}

impl<W, S> Subclass<W> for TextLabelSubclass<S>
        where W: TextLabelWindow,
              S: AsRef<str>
{
    type UserMsg = DerinMsg;
    fn subclass_proc(&self, window: &ProcWindowRef<W, Self>, mut msg: Msg<DerinMsg>) -> i64 {
        let ret = window.default_window_proc(&mut msg);
        match msg {
            Msg::Wm(wm) => match wm {
                Wm::SetText(new_text) => {
                    let mut widget_data = self.widget_data.get();
                    widget_data.abs_size_bounds.min = unsafe{ window.min_unclipped_rect_raw(new_text) };
                    self.widget_data.set(widget_data);
                },
                Wm::GetSizeBounds(size_bounds) => *size_bounds = self.widget_data.get().combined_size_bounds(),
                Wm::Size(_) => window.show(true),
                _ => ()
            },
            Msg::User(DerinMsg::SetRectPropagate(rect)) => window.set_rect(rect),
            _ => ()
        }
        ret
    }
}

/// A top-level window subclass, with a reference to its child.
struct ToplevelSubclass(UnsafeSubclassRef<DerinMsg>);

impl Subclass<ToplevelWindowBase> for ToplevelSubclass {
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<ToplevelWindowBase, Self>, mut msg: Msg<()>) -> i64 {
        match msg {
            Msg::Wm(Wm::GetSizeBounds(size_bounds)) => {*size_bounds = self.0.size_bounds(); 0},
            Msg::Wm(Wm::Size(rect)) => unsafe{
                self.0.post_user_msg(DerinMsg::SetRectPropagate(OffsetRect::from(rect)));
                0
            },
            _ => window.default_window_proc(&mut msg)
        }
    }
}

pub trait NativeDataWrapper {
    fn size_bounds(&self) -> SizeBounds;
    fn set_rect(&self, OffsetRect);
    fn get_widget_data(&self) -> WidgetData;
    fn window_ref(&self) -> WindowRef;
    fn unsafe_subclass_ref(&self) -> UnsafeSubclassRef<DerinMsg>;
    fn needs_update(&self) -> bool;
}

pub trait ParentDataWrapper {
    type Adder: ParentChildAdder; // ssssSSsssSSsss
    fn get_adder(&self) -> Self::Adder;
}

pub trait ParentChildAdder {
    fn add_child_node<N>(&mut self, &N)
            where N: Node,
                  N::Wrapper: NativeDataWrapper;
}

#[derive(Debug, Clone, Copy, UserMsg)]
pub enum DerinMsg {
    SetRectPropagate(OffsetRect)
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



/// Newtype wrapper around parents to allow them to implement `Container` trait
struct ParentContainer<I>(I);

impl<I> Container for ParentContainer<I>
        where for<'a> I: Parent<ConstraintSolverTraverser<'a>>
{
    fn update_widget_rects(&mut self, solver: WidgetConstraintSolver) {
        let mut traverser = ConstraintSolverTraverser {
            solver: solver
        };
        self.0.children(&mut traverser).ok();
    }
}

pub struct ConstraintSolverTraverser<'a> {
    solver: WidgetConstraintSolver<'a>
}

impl<'s, N> NodeProcessor<N> for ConstraintSolverTraverser<'s>
        where N: Node,
              N::Wrapper: NativeDataWrapper
{
    fn add_child<'a>(&'a mut self, _: ChildId, node: &'a mut N) -> Result<(), ()> {
        let widget_rect_result = self.solver.solve_widget_constraints(node.wrapper().get_widget_data());
        match widget_rect_result {
            Ok(rect) => {node.wrapper().set_rect(rect); Ok(())},
            Err(SolveError::Abort) => Err(()),
            Err(SolveError::WidgetUnsolvable) => Ok(())
        }
    }
}

impl<'a> NodeProcessorAT for ConstraintSolverTraverser<'a> {
    type Error = ();
}
