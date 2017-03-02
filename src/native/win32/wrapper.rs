use ui::{Control, Parent, Node, ChildId, NodeProcessor, NodeProcessorAT, NodeDataWrapper};
use ui::intrinsics::TextButton;
use dww::*;
use dle::{Widget, Container, LayoutEngine, WidgetData, WidgetConstraintSolver, SolveError};
use dle::hints::WidgetHints;
use dct::events::{MouseEvent, MouseButton};
use dct::geometry::OffsetRect;

use std::marker::PhantomData;

use super::NodeTraverser;

pub struct TextButtonWindow<I>(UnsafeSubclassWrapper<PushButtonBase, TextButtonSubclass<I>>)
        where I: 'static + AsRef<str> + Control;

enum ButtonState {
    Released,
    Pressed,
    DoublePressed
}

pub struct TextButtonSubclass<I: 'static + AsRef<str> + Control> {
    widget: I,
    widget_data: WidgetData,
    button_state: ButtonState
}

impl<B, I> Subclass<B> for TextButtonSubclass<I>
        where B: ButtonWindow,
              I: 'static + AsRef<str> + Control
{
    type UserMsg = ();
    fn subclass_proc(&mut self, window: &ProcWindowRef<B>, msg: Msg<()>) -> i64 {
        let ret = window.default_window_proc();

        match msg {
            Msg::Wm(wm) => match wm {
                Wm::MouseDown(_, _) => self.button_state = ButtonState::Pressed,
                Wm::MouseDoubleDown(_, _) => self.button_state = ButtonState::DoublePressed,
                Wm::MouseUp(button, point) => {
                    let action = match self.button_state {
                        ButtonState::Pressed       => self.widget.on_mouse_event(MouseEvent::Clicked(button)),
                        ButtonState::DoublePressed => self.widget.on_mouse_event(MouseEvent::DoubleClicked(button)),
                        ButtonState::Released      => None
                    };

                    self.button_state = ButtonState::Released;
                },
                Wm::SetText(_) => self.widget_data.abs_size_bounds.min = window.get_ideal_size(),
                _ => ()
            },
            _ => ()
        }
        ret
    }
}

pub type ParentSubclassWindow<I> = UnsafeSubclassWrapper<BlankBase, ParentSubclass<I>>;

pub struct ParentSubclass<I: 'static + Parent<()>> {
    widget: I,
    widget_data: WidgetData,
    layout_engine: LayoutEngine
}

impl<P, I> Subclass<P> for ParentSubclass<I>
        where P: ParentWindow,
              I: 'static + Parent<()>
{
    type UserMsg = ();
    fn subclass_proc(&mut self, window: &ProcWindowRef<P>, msg: Msg<()>) -> i64 {
        if let Msg::Wm(Wm::GetSizeBounds(size_bounds)) = msg {
            *size_bounds = self.layout_engine.actual_size_bounds();
            0
        } else {
            window.default_window_proc()
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

struct ConstraintSolverTraverser<'a> {
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

trait WidgetDataContainer {
    fn get_widget_data(&self) -> WidgetData;
}
