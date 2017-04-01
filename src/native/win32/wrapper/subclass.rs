use ui::{Parent, Node, ChildId, NodeProcessorInit, NodeProcessorGrid, NodeProcessorGridMut, NodeProcessor};
use ui::widgets::{Button, ProgBarStatus};

use dww::*;
use dle::{GridContainer, GridEngine, GridConstraintSolver, SolveError};
use dle::hints::{WidgetHints, GridSize, TrackHints};
use dct::events::MouseEvent;
use dct::geometry::{OriginRect, OffsetRect, SizeBounds};

use std::borrow::Borrow;

use super::{DerinMsg, SharedFn, ToplevelWindowBase, NativeDataWrapper};

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

pub struct TextButtonSubclass<I: Borrow<str> + Button> {
    pub node_data: I,
    pub action_fn: Option<SharedFn<I::Action>>,
    pub abs_size_bounds: SizeBounds,
    button_state: ButtonState
}

impl<I: Borrow<str> + Button> TextButtonSubclass<I> {
    #[inline]
    pub fn new(node_data: I) -> TextButtonSubclass<I> {
        TextButtonSubclass {
            node_data: node_data,
            action_fn: None,
            abs_size_bounds: SizeBounds::default(),
            button_state: ButtonState::default()
        }
    }
}

impl<B, I> Subclass<B> for TextButtonSubclass<I>
        where B: ButtonWindow,
              I: Borrow<str> + Button
{
    type UserMsg = DerinMsg;
    fn subclass_proc(window: &mut ProcWindowRef<B, Self>, mut msg: Msg<DerinMsg>) -> i64 {
        let ret = window.default_window_proc(&mut msg);

        match msg {
            Msg::Wm(wm) => match wm {
                Wm::MouseDown(_, _) => window.subclass_data().button_state = ButtonState::Pressed,
                Wm::MouseDoubleDown(_, _) => window.subclass_data().button_state = ButtonState::DoublePressed,
                Wm::MouseUp(button, _) => {
                    let action_opt = match window.subclass_data().button_state {
                        ButtonState::Pressed       => window.subclass_data().node_data.on_mouse_event(MouseEvent::Clicked(button)),
                        ButtonState::DoublePressed => window.subclass_data().node_data.on_mouse_event(MouseEvent::DoubleClicked(button)),
                        ButtonState::Released      => None
                    };
                    if let Some(action) = action_opt {
                        unsafe{ window.subclass_data().action_fn.as_ref().expect("No Action Function").borrow_mut().call_fn(action) };
                    }

                    window.subclass_data().button_state = ButtonState::Released;
                },
                Wm::SetText(_) => window.subclass_data().abs_size_bounds.min = window.get_ideal_size(),
                Wm::GetSizeBounds(size_bounds) => size_bounds.min = window.get_ideal_size(),
                Wm::Size(_) => window.show(true),
                _ => ()
            },
            Msg::User(DerinMsg::SetRectPropagate(rect)) |
            Msg::User(DerinMsg::SetRect(rect))         => window.set_rect(rect),
            _ => ()
        }
        ret
    }
}


pub struct WidgetGroupSubclass<I: Parent<!>> {
    pub node_data: I,
    pub layout_engine: GridEngine
}

impl<I: Parent<!>> WidgetGroupSubclass<I> {
    #[inline]
    pub fn new(node_data: I) -> WidgetGroupSubclass<I> {
        WidgetGroupSubclass {
            node_data: node_data,
            layout_engine: GridEngine::new()
        }
    }
}

impl<P, I> Subclass<P> for WidgetGroupSubclass<I>
        where P: ParentWindow + WindowMut,
              I: Parent<!>
{
    type UserMsg = DerinMsg;
    default fn subclass_proc(_: &mut ProcWindowRef<P, Self>, _: Msg<DerinMsg>) -> i64 {
        panic!("Should never be called; just here to hide GridWidgetProcessor type from public exposure")
    }
}

impl<P, I> Subclass<P> for WidgetGroupSubclass<I>
        where P: ParentWindow + WindowMut,
      for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>>
{
    fn subclass_proc(window: &mut ProcWindowRef<P, Self>, mut msg: Msg<DerinMsg>) -> i64 {
        match msg {
            Msg::Wm(wm) => match wm {
                Wm::GetSizeBounds(size_bounds) => {
                    *size_bounds = window.subclass_data().layout_engine.actual_size_bounds();
                    0
                },
                Wm::Size(_) => {window.show(true); 0},
                wm => window.default_window_proc(&mut Msg::Wm(wm))
            },
            Msg::User(DerinMsg::SetRect(rect)) => {window.set_rect(rect); 0},
            Msg::User(DerinMsg::SetRectPropagate(rect)) => {
                {
                    let WidgetGroupSubclass {
                        ref mut node_data,
                        ref mut layout_engine,
                        ..
                    } = *window.subclass_data();

                    layout_engine.desired_size = OriginRect::from(rect);
                    layout_engine.update_engine(&mut ParentContainer(node_data)).ok();
                }
                window.set_rect(rect);
                0
            },
            _ => window.default_window_proc(&mut msg)
        }
    }
}


pub struct TextLabelSubclass<S: AsRef<str>> {
    pub text: S,
    pub abs_size_bounds: SizeBounds
}

impl<S: AsRef<str>> TextLabelSubclass<S> {
    #[inline]
    pub fn new(text: S) -> TextLabelSubclass<S> {
        TextLabelSubclass {
            text: text,
            abs_size_bounds: SizeBounds::default()
        }
    }
}

impl<W, S> Subclass<W> for TextLabelSubclass<S>
        where W: TextLabelWindow + WindowMut,
              S: AsRef<str>
{
    type UserMsg = DerinMsg;
    fn subclass_proc(window: &mut ProcWindowRef<W, Self>, mut msg: Msg<DerinMsg>) -> i64 {
        let ret = window.default_window_proc(&mut msg);
        match msg {
            Msg::Wm(wm) => match wm {
                Wm::SetText(new_text) =>
                    window.subclass_data().abs_size_bounds.min = unsafe{ window.min_unclipped_rect_raw(new_text) },
                Wm::GetSizeBounds(size_bounds) => *size_bounds = window.subclass_data().abs_size_bounds,
                Wm::Size(_) => window.show(true),
                _ => ()
            },
            Msg::User(DerinMsg::SetRectPropagate(rect)) |
            Msg::User(DerinMsg::SetRect(rect))         => window.set_rect(rect),
            _ => ()
        }
        ret
    }
}

pub struct ProgressBarSubclass {
    pub status: ProgBarStatus
}

impl ProgressBarSubclass {
    #[inline]
    pub fn new(status: ProgBarStatus) -> ProgressBarSubclass {
        ProgressBarSubclass {
            status: status
        }
    }
}

impl<W> Subclass<W> for ProgressBarSubclass
        where W: ProgressBarWindow + WindowMut
{
    type UserMsg = DerinMsg;
    fn subclass_proc(window: &mut ProcWindowRef<W, Self>, msg: Msg<DerinMsg>) -> i64 {
        match msg {
            Msg::User(DerinMsg::SetRectPropagate(rect)) |
            Msg::User(DerinMsg::SetRect(rect))         => {window.set_rect(rect); 0},
            Msg::Wm(Wm::Size(rect)) => {
                window.show(true);
                window.default_window_proc(&mut Msg::Wm(Wm::Size(rect)))
            },
            mut msg => window.default_window_proc(&mut msg)
        }
    }
}

/// A top-level window subclass, with a reference to its child.
pub struct ToplevelSubclass(pub UnsafeChildSubclassRef<'static, DerinMsg>);

impl Subclass<ToplevelWindowBase> for ToplevelSubclass {
    type UserMsg = ();
    fn subclass_proc(window: &mut ProcWindowRef<ToplevelWindowBase, Self>, mut msg: Msg<()>) -> i64 {
        match msg {
            Msg::Wm(Wm::GetSizeBounds(size_bounds)) => {*size_bounds = window.subclass_data().0.size_bounds(); 0},
            Msg::Wm(Wm::Size(rect)) => {
                window.subclass_data().0.post_user_msg(DerinMsg::SetRectPropagate(OffsetRect::from(rect)));
                0
            },
            _ => window.default_window_proc(&mut msg)
        }
    }
}


/// Newtype wrapper around parents to allow them to implement `Container` trait
pub struct ParentContainer<'a, I: 'a>( pub &'a mut I );

impl<'a, I> GridContainer for ParentContainer<'a, I>
        where for<'b> I: Parent<GridWidgetProcessor<'b>>
{
    fn update_widget_rects(&mut self, solver: GridConstraintSolver) {
        let traverser = GridWidgetProcessor {
            solver: solver
        };
        self.0.children(traverser).ok();
    }
}

pub struct GridWidgetProcessor<'a> {
    solver: GridConstraintSolver<'a>
}

impl<'a> NodeProcessorInit for GridWidgetProcessor<'a> {
    type Error = ();
    type GridProcessor = GridWidgetProcessor<'a>;
    fn init_grid<C, R>(self, _: GridSize, _: C, _: R) -> GridWidgetProcessor<'a>
            where C: Iterator<Item = TrackHints>,
                  R: Iterator<Item = TrackHints>
    {self}
}

impl<'s, N> NodeProcessorGridMut<N> for GridWidgetProcessor<'s>
        where N: Node,
              N::Wrapper: NativeDataWrapper
{
    fn add_child_mut<'a>(&'a mut self, id: ChildId, widget_hints: WidgetHints, node: &'a mut N) -> Result<(), ()> {
        self.add_child(id, widget_hints, node)
    }
}

impl<'s, N> NodeProcessorGrid<N> for GridWidgetProcessor<'s>
        where N: Node,
              N::Wrapper: NativeDataWrapper
{
    fn add_child<'a>(&'a mut self, _: ChildId, widget_hints: WidgetHints, node: &'a N) -> Result<(), ()> {
        let widget_rect_result = self.solver.solve_widget_constraints(widget_hints, node.wrapper().abs_size_bounds());
        match widget_rect_result {
            Ok(rect) => {
                node.wrapper().post_user_msg(DerinMsg::SetRect(rect));
                Ok(())
            },
            Err(SolveError::Abort) => Err(()),
            Err(SolveError::WidgetUnsolvable) |
            Err(SolveError::CellOutOfBounds) => Ok(())
        }
    }
}

impl<'a> NodeProcessor for GridWidgetProcessor<'a> {
    type Error = ();
}
