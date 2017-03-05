use ui::{Control, Parent, Node, ChildId, NodeProcessor, NodeProcessorAT, NodeDataWrapper};
use ui::intrinsics::TextButton;
use dww::*;
use dle::{Widget, Container, LayoutEngine, WidgetData, WidgetConstraintSolver, SolveError, LayoutUpdater};
use dle::hints::WidgetHints;
use dct::events::{MouseEvent, MouseButton};
use dct::geometry::{Rect, OffsetRect, Point};

use std::cell::{Cell, RefCell};

type ToplevelWindowBase = IconWrapper<WindowIcon, OverlapWrapper<BlankBase>>;

macro_rules! node_data {
    (
        pub struct $name:ident<$inner_ty:ident>( $field_ty:ty )
                $(where $($where_ty:ty: $($constraint:path)|+),+)*;

        impl {
            expr widget_data($widget_data_in:ident) = $widget_data:expr;
            expr node_data($node_data_in:ident) = $node_data:expr;

            fn from_node_data($fnd_in:ident: _) -> Self $from_node_data:block
        }

        $($rest:tt)*
    ) => {
        pub struct $name<$inner_ty>( $field_ty )
                $(where $($where_ty: $($constraint +)+),+)*;

        impl<$inner_ty> $name<$inner_ty>
                $(where $($where_ty: $($constraint +)+),+)*
        {
            #[doc(hidden)]
            #[inline]
            pub unsafe fn update_subclass_ptr(&self) {
                self.0.update_subclass_ptr();
            }
        }

        impl<$inner_ty> WidgetDataContainer for $name<$inner_ty>
                $(where $($where_ty: $($constraint +)+),+)*
        {
            #[inline]
            fn get_widget_data(&self) -> WidgetData {
                let $widget_data_in = self;
                $widget_data
            }
        }

        impl<$inner_ty> NodeDataWrapper<$inner_ty> for $name<$inner_ty>
                $(where $($where_ty: $($constraint +)+),+)*
        {
            fn from_node_data($fnd_in: $inner_ty) -> $name<$inner_ty> $from_node_data

            fn inner(&self) -> &$inner_ty {let $node_data_in = self; &$node_data}
            fn inner_mut(&mut self) -> &mut $inner_ty {let $node_data_in = self; &mut $node_data}
            fn unwrap(self) -> $inner_ty {let $node_data_in = self; $node_data}
        }

        node_data!{$($rest)*}
    };

    () => ();
}

node_data!{
    pub struct TextButtonNodeData<I>( UnsafeSubclassWrapper<PushButtonBase, TextButtonSubclass<I>> )
            where I: AsRef<str> | Control;

    impl {
        expr widget_data(this) = this.0.subclass_data.mutable_data.borrow().widget_data;
        expr node_data(this) = this.0.subclass_data.node_data;

        fn from_node_data(node_data: _) -> Self {
            let button_window = WindowBuilder::default().build_push_button();
            let subclass = TextButtonSubclass::new(node_data);

            let wrapper = unsafe{ UnsafeSubclassWrapper::new(button_window, subclass) };
            TextButtonNodeData(wrapper)
        }
    }

    pub struct WidgetGroupNodeData<I>( UnsafeSubclassWrapper<BlankBase, WidgetGroupSubclass<I>> )
            where I: Parent<()>;

    impl {
        expr widget_data(this) = this.0.subclass_data.widget_data;
        expr node_data(this) = this.0.subclass_data.node_data.0;

        fn from_node_data(node_data: _) -> Self {
            let blank_window = WindowBuilder::default().build_blank();
            let subclass = WidgetGroupSubclass::new(node_data);

            let wrapper = unsafe{ UnsafeSubclassWrapper::new(blank_window, subclass) };
            WidgetGroupNodeData(wrapper)
        }
    }

    pub struct TextLabelNodeData<S>( UnsafeSubclassWrapper<TextLabelBase, TextLabelSubclass<S>> )
            where S: AsRef<str>;

    impl {
        expr widget_data(this) = this.0.subclass_data.widget_data.get();
        expr node_data(this) = this.0.subclass_data.text;

        fn from_node_data(text: _) -> Self {
            let label_window = WindowBuilder::default().build_text_label();
            let subclass = TextLabelSubclass::new(text);

            let wrapper = unsafe{ UnsafeSubclassWrapper::new(label_window, subclass) };
            TextLabelNodeData(wrapper)
        }
    }
}

impl<I> WidgetGroupNodeData<I>
        where for<'a> I: Parent<()> + Parent<ConstraintSolverTraverser<'a>>
{
    pub fn layout_engine(&self) -> &LayoutEngine {
        &self.0.subclass_data.layout_engine
    }

    pub fn layout_engine_mut(&mut self) -> &mut LayoutEngine {
        &mut self.0.subclass_data.layout_engine
    }

    pub fn update_engine(&mut self, updater: &mut LayoutUpdater) {
        updater.update_engine(&mut self.0.subclass_data.node_data, &mut self.0.subclass_data.layout_engine);
    }
}

pub type ToplevelWindow = UnsafeSubclassWrapper<ToplevelWindowBase, ToplevelSubclass>;



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
            mutable_data: RefCell::new(TBSMut::default())
        }
    }
}

impl<B, I> Subclass<B> for TextButtonSubclass<I>
        where B: ButtonWindow,
              I: AsRef<str> + Control
{
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<B>, msg: Msg<()>) -> i64 {
        let ret = window.default_window_proc();
        let mut mutable_data = self.mutable_data.borrow_mut();

        match msg {
            Msg::Wm(wm) => match wm {
                Wm::MouseDown(_, _) => mutable_data.button_state = ButtonState::Pressed,
                Wm::MouseDoubleDown(_, _) => mutable_data.button_state = ButtonState::DoublePressed,
                Wm::MouseUp(button, _) => {
                    let action = match mutable_data.button_state {
                        ButtonState::Pressed       => self.node_data.on_mouse_event(MouseEvent::Clicked(button)),
                        ButtonState::DoublePressed => self.node_data.on_mouse_event(MouseEvent::DoubleClicked(button)),
                        ButtonState::Released      => None
                    };

                    mutable_data.button_state = ButtonState::Released;
                },
                Wm::SetText(_) => mutable_data.widget_data.abs_size_bounds.min = window.get_ideal_size(),
                _ => ()
            },
            _ => ()
        }
        ret
    }
}


struct WidgetGroupSubclass<I: Parent<()>> {
    node_data: ParentContainer<I>,
    widget_data: WidgetData,
    layout_engine: LayoutEngine
}

impl<I: Parent<()>> WidgetGroupSubclass<I> {
    #[inline]
    fn new(node_data: I) -> WidgetGroupSubclass<I> {
        WidgetGroupSubclass {
            node_data: ParentContainer(node_data),
            widget_data: WidgetData::default(),
            layout_engine: LayoutEngine::new()
        }
    }
}

impl<P, I> Subclass<P> for WidgetGroupSubclass<I>
        where P: ParentWindow,
              I: Parent<()>
{
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<P>, msg: Msg<()>) -> i64 {
        if let Msg::Wm(Wm::GetSizeBounds(size_bounds)) = msg {
            *size_bounds = self.layout_engine.actual_size_bounds();
            0
        } else {
            window.default_window_proc()
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
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<W>, msg: Msg<()>) -> i64 {
        if let Msg::Wm(Wm::SetText(new_text)) = msg {
            let mut widget_data = self.widget_data.get();
            widget_data.abs_size_bounds.min = unsafe{ window.min_unclipped_rect_raw(new_text) };
            self.widget_data.set(widget_data);
        }
        window.default_window_proc()
    }
}

/// A top-level window subclass, with a reference to its child.
struct ToplevelSubclass(WindowRef);

impl Subclass<ToplevelWindowBase> for ToplevelSubclass {
    type UserMsg = ();
    fn subclass_proc(&self, window: &ProcWindowRef<ToplevelWindowBase>, msg: Msg<()>) -> i64 {
        match msg {
            Msg::Wm(Wm::GetSizeBounds(size_bounds)) => {*size_bounds = self.0.size_bounds(); 0},
            Msg::Wm(Wm::Size(rect)) => {self.0.set_rect(OffsetRect::from(rect)); 0},
            _ => window.default_window_proc()
        }
    }
}



/// Newtype wrapper around parents to allow them to implement `Container` trait
struct ParentContainer<I>(I);

impl<I, NP> Parent<NP> for ParentContainer<I>
        where I: Parent<NP>,
              NP: NodeProcessorAT
{
    type ChildAction = I::ChildAction;
    type ChildLayout = I::ChildLayout;

    fn children(&mut self, np: NP) -> Result<(), NP::Error> {
        self.0.children(np)
    }
    fn child_layout(&self) -> I::ChildLayout {
        self.0.child_layout()
    }
}

impl<I> Container for ParentContainer<I>
        where for<'a> I: Parent<ConstraintSolverTraverser<'a>>
{
    fn update_widget_rects(&mut self, solver: WidgetConstraintSolver) {
        let traverser = ConstraintSolverTraverser {
            solver: solver
        };
        self.children(traverser).ok();
    }
}

pub struct ConstraintSolverTraverser<'a> {
    solver: WidgetConstraintSolver<'a>
}

impl<'s, W, N> NodeProcessor<W, N> for ConstraintSolverTraverser<'s>
        where W: NodeDataWrapper<N::Inner> + WidgetDataContainer + Window,
              N: Node<W>
{
    fn add_child<'a>(&'a mut self, _: ChildId, node: &'a mut N) -> Result<(), ()> {
        match self.solver.solve_widget_constraints(node.data().get_widget_data()) {
            Ok(rect) => {node.data().set_rect(rect); Ok(())},
            Err(SolveError::Abort) => Err(()),
            Err(SolveError::WidgetUnsolvable) => Ok(())
        }
    }
}

impl<'a> NodeProcessorAT for ConstraintSolverTraverser<'a> {
    type Error = ();
}

pub trait WidgetDataContainer {
    fn get_widget_data(&self) -> WidgetData;
}
