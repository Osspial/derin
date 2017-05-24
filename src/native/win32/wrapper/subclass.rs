use ui::{Parent, Node, ChildId, NodeProcessorInit, NodeProcessorGrid, NodeProcessorGridMut, NodeProcessor, EventActionMap};
use ui::widgets::{MouseEvent, RangeEvent};
use ui::widgets::content::{SliderStatus, ProgbarStatus, LabelGroupContents};

use dww::window::*;
use dww::window::refs::{ProcWindowRef, UnsafeSubclassRef};
use dww::gdi::text::Font;
use dww::msg::Msg;
use dww::msg::notify::{Notification, NotifyType, ThumbReason};
use dle::{GridContainer, GridEngine, GridConstraintSolver, SolveError};
use ui::hints::{WidgetHints, GridSize, TrackHints, SizeBounds, Margins};
use ui::geometry::{OriginRect, OffsetRect};

use super::{DerinMsg, SharedFn, ToplevelBaseWindow, NativeDataWrapper};

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

pub struct TextButtonSubclass<B: EventActionMap<MouseEvent>, S: AsRef<str>> {
    pub button_action_map: B,
    pub text: S,
    pub action_fn: Option<SharedFn<B::Action>>,
    pub abs_size_bounds: SizeBounds,
    button_state: ButtonState
}

impl<B: EventActionMap<MouseEvent>, S: AsRef<str>> TextButtonSubclass<B, S> {
    #[inline]
    pub fn new(button_action_map: B, text: S) -> TextButtonSubclass<B, S> {
        TextButtonSubclass {
            button_action_map,
            text,
            action_fn: None,
            abs_size_bounds: SizeBounds::default(),
            button_state: ButtonState::default()
        }
    }
}

impl<W, B, S> Subclass<W> for TextButtonSubclass<B, S>
        where W: ButtonWindow,
              B: EventActionMap<MouseEvent>,
              S: AsRef<str>
{
    type UserMsg = DerinMsg;
    fn subclass_proc(mut window: ProcWindowRef<W, Self>) -> i64 {
        let ret = window.default_window_proc();
        if let Some((window, msg)) = window.msg() {
            match msg {
                Msg::MouseDown(_, _) => window.subclass_data().button_state = ButtonState::Pressed,
                Msg::MouseDoubleDown(_, _) => window.subclass_data().button_state = ButtonState::DoublePressed,
                Msg::MouseUp(button, _) => {
                    let action_opt = match window.subclass_data().button_state {
                        ButtonState::Pressed       => window.subclass_data().button_action_map.on_event(MouseEvent::Clicked(button)),
                        ButtonState::DoublePressed => window.subclass_data().button_action_map.on_event(MouseEvent::DoubleClicked(button)),
                        ButtonState::Released      => None
                    };
                    if let Some(action) = action_opt {
                        unsafe{ window.subclass_data().action_fn.as_ref().expect("No Action Function").borrow_mut().call_fn(action) };
                    }

                    window.subclass_data().button_state = ButtonState::Released;
                },
                Msg::SetText(_) => window.subclass_data().abs_size_bounds.min = window.get_ideal_size(),
                Msg::GetSizeBounds(size_bounds) => size_bounds.min = window.get_ideal_size(),
                Msg::User(DerinMsg::SetRect(rect)) => window.set_rect(rect),
                _ => ()
            }
        }
        ret
    }
}


pub struct GroupSubclass<I: Parent<!>> {
    pub content_data: I,
    pub layout_engine: GridEngine
}

impl<I: Parent<!>> GroupSubclass<I> {
    #[inline]
    pub fn new(content_data: I) -> GroupSubclass<I> {
        GroupSubclass {
            content_data: content_data,
            layout_engine: GridEngine::new()
        }
    }
}

impl<W, I> Subclass<W> for GroupSubclass<I>
        where W: ParentWindow + MutWindow,
              I: Parent<!>
{
    type UserMsg = DerinMsg;
    default fn subclass_proc(_: ProcWindowRef<W, Self>) -> i64 {
        panic!("Should never be called; just here to hide GridWidgetProcessor type from public exposure")
    }
}

impl<W, I> Subclass<W> for GroupSubclass<I>
        where W: ParentWindow + MutWindow,
      for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>>
{
    fn subclass_proc(mut window: ProcWindowRef<W, Self>) -> i64 {
        if let Some((window, msg)) = window.msg() {
            match msg {
                Msg::GetSizeBounds(size_bounds) => {
                    *size_bounds = window.subclass_data().layout_engine.actual_size_bounds();
                    return 0;
                },
                Msg::User(DerinMsg::SetRect(rect)) => {
                    {
                        let GroupSubclass {
                            ref mut content_data,
                            ref mut layout_engine,
                            ..
                        } = *window.subclass_data();

                        layout_engine.desired_size = OriginRect::from(rect);
                        layout_engine.update_engine(&mut ParentContainer(content_data)).ok();
                    }
                    window.set_rect(rect);
                    return 0;
                },
                _ => ()
            }
        }

        window.default_window_proc()
    }
}


pub struct LabelGroupSubclass<S: AsRef<str>, I: Parent<!>> {
    pub contents: LabelGroupContents<S, I>,
    pub groupbox_window: GroupBoxBase<&'static Font>,
    pub layout_engine: GridEngine
}

impl<S: AsRef<str>, I: Parent<!>> LabelGroupSubclass<S, I> {
    pub fn new<W: ParentWindow>(contents: LabelGroupContents<S, I>, container_window: &W) -> LabelGroupSubclass<S, I> {
        container_window.clip_children(false);
        let mut layout_engine = GridEngine::new();
        layout_engine.grid_margins = Margins::new(9, 16, 9, 11);
        LabelGroupSubclass {
            contents,
            groupbox_window: WindowBuilder::default().build_group_box_with_font(container_window, &*super::CAPTION_FONT),
            layout_engine
        }
    }
}

impl<W, S, I> Subclass<W> for LabelGroupSubclass<S, I>
        where W: ParentWindow + MutWindow,
              S: AsRef<str>,
              I: Parent<!>
{
    type UserMsg = DerinMsg;
    default fn subclass_proc(_: ProcWindowRef<W, Self>) -> i64 {
        panic!("Should never be called; just here to hide GridWidgetProcessor type from public exposure")
    }
}

impl<W, S, I> Subclass<W> for LabelGroupSubclass<S, I>
        where W: ParentWindow + MutWindow,
              S: AsRef<str>,
      for<'a> I: Parent<!> + Parent<GridWidgetProcessor<'a>>
{
    fn subclass_proc(mut window: ProcWindowRef<W, Self>) -> i64 {
        if let Some((window, msg)) = window.msg() {
            match msg {
                Msg::GetSizeBounds(size_bounds) => {
                    *size_bounds = window.subclass_data().layout_engine.actual_size_bounds();
                    return 0;
                },
                Msg::User(DerinMsg::SetRect(rect)) => {
                    {
                        let LabelGroupSubclass {
                            contents: LabelGroupContents {ref mut children, ..},
                            ref mut groupbox_window,
                            ref mut layout_engine
                        } = *window.subclass_data();

                        layout_engine.desired_size = OriginRect::from(rect);
                        layout_engine.update_engine(&mut ParentContainer(children)).ok();
                        groupbox_window.set_rect(OriginRect::from(rect).into());
                    }
                    window.set_rect(rect);
                    return 0;
                },
                _ => ()
            }
        }

        window.default_window_proc()
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
        where W: TextLabelWindow + MutWindow,
              S: AsRef<str>
{
    type UserMsg = DerinMsg;
    fn subclass_proc(mut window: ProcWindowRef<W, Self>) -> i64 {
        let ret = window.default_window_proc();
        if let Some((window, msg)) = window.msg() {
            match msg {
                Msg::SetText(new_text) =>
                    window.subclass_data().abs_size_bounds.min = unsafe{ window.min_unclipped_rect_ucs2(new_text) },
                Msg::GetSizeBounds(size_bounds) => *size_bounds = window.subclass_data().abs_size_bounds,
                Msg::User(DerinMsg::SetRect(rect)) => window.set_rect(rect),
                _ => ()
            }
        }
        ret
    }
}

pub struct ProgbarSubclass {
    pub status: ProgbarStatus
}

impl ProgbarSubclass {
    #[inline]
    pub fn new(status: ProgbarStatus) -> ProgbarSubclass {
        ProgbarSubclass {
            status: status
        }
    }
}

impl<W> Subclass<W> for ProgbarSubclass
        where W: ProgressBarWindow + MutWindow
{
    type UserMsg = DerinMsg;
    fn subclass_proc(mut window: ProcWindowRef<W, Self>) -> i64 {
        if let Some((window, Msg::User(DerinMsg::SetRect(rect)))) = window.msg() {
            window.set_rect(rect);
            return 0;
        }
        window.default_window_proc()
    }
}

pub struct SliderSubclass<C: EventActionMap<RangeEvent>> {
    pub range_action_map: C,
    pub status: SliderStatus,
    pub action_fn: Option<SharedFn<C::Action>>,
    pub slider_window: TrackbarBase
}

impl<C: EventActionMap<RangeEvent>> SliderSubclass<C> {
    pub fn new<W: ParentWindow>(range_action_map: C, status: SliderStatus, container_window: &W) -> SliderSubclass<C> {
        SliderSubclass {
            range_action_map,
            status,
            action_fn: None,
            slider_window: WindowBuilder::default().build_trackbar(container_window)
        }
    }
}

impl<W, C> Subclass<W> for SliderSubclass<C>
        where W: MutWindow,
              C: EventActionMap<RangeEvent>
{
    type UserMsg = DerinMsg;
    fn subclass_proc(mut window: ProcWindowRef<W, Self>) -> i64 {
        if let Some((window, msg)) = window.msg() {
            match msg {
                Msg::Notify(Notification {
                    notify_type: NotifyType::TrackbarThumbPosChanging(new_pos, reason),
                    ..
                }) => {
                    let event = match reason {
                        ThumbReason::EndTrack       |
                        ThumbReason::ThumbPosition => RangeEvent::Drop(new_pos),
                        _                          => RangeEvent::Move(new_pos)
                    };
                    let action_opt = window.subclass_data().range_action_map.on_event(event);
                    if let Some(action) = action_opt {
                        unsafe{ window.subclass_data().action_fn.as_ref().expect("No Action Function").borrow_mut().call_fn(action) };
                    }
                    return 0;
                },
                Msg::User(DerinMsg::SetRect(rect)) => {
                    window.set_rect(rect);
                    window.subclass_data().slider_window.set_rect(OriginRect::from(rect).into());
                    return 0;
                },
                _ => ()
            }
        }

        window.default_window_proc()
    }
}

/// A top-level window subclass, with a reference to its child.
pub struct ToplevelSubclass(pub UnsafeSubclassRef<'static, DerinMsg>);

impl Subclass<ToplevelBaseWindow> for ToplevelSubclass {
    type UserMsg = ();
    fn subclass_proc(mut window: ProcWindowRef<ToplevelBaseWindow, Self>) -> i64 {
        if let Some((window, msg)) = window.msg() {
            match msg {
                Msg::GetSizeBounds(size_bounds) => {
                    *size_bounds = window.subclass_data().0.size_bounds();
                    size_bounds.min = window.adjust_window_rect(size_bounds.min);
                    size_bounds.max = window.adjust_window_rect(size_bounds.max);
                    return 0;
                },
                Msg::Size(rect) => {
                    window.subclass_data().0.post_user_msg(DerinMsg::SetRect(OffsetRect::from(rect)));
                    return 0;
                },
                _ => ()
            }
        }

        window.default_window_proc()
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
