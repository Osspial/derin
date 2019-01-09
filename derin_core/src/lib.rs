// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![feature(range_contains, nll, specialization, try_blocks)]

use cgmath_geometry::cgmath;
#[macro_use]
extern crate bitflags;
extern crate derin_common_types;

#[cfg(test)]
#[macro_use]
pub mod test_helpers;

pub mod timer;
#[macro_use]
pub mod tree;
pub mod event;
pub mod render;

mod mbseq;
mod offset_widget;
mod event_translator;
mod update_state;
mod widget_traverser;

use crate::cgmath::{Point2, Vector2, Bounded};
use cgmath_geometry::{D2, rect::{DimsBox, GeoBox}};

use crate::{
    event::WidgetEvent,
    event_translator::EventTranslator,
    timer::TimerList,
    tree::*,
    render::{Renderer, RenderFrame, RenderFrameClipped},
    mbseq::MouseButtonSequenceTrackPos,
    offset_widget::OffsetWidgetTrait,
    update_state::{UpdateState, UpdateStateCell},
    widget_traverser::{Relation, WidgetPath, WidgetTraverser, WidgetTraverserBase},
};
use derin_common_types::{
    buttons::{MouseButton, Key, ModifierKeys},
};
use std::{
    rc::Rc,
    time::Instant,
};

const MAX_FRAME_UPDATE_ITERATIONS: usize = 256;

fn find_index<T: PartialEq>(s: &[T], element: &T) -> Option<usize> {
    s.iter().enumerate().find(|&(_, e)| e == element).map(|(i, _)| i)
}

#[must_use]
fn vec_remove_element<T: PartialEq>(v: &mut Vec<T>, element: &T) -> Option<T> {
    find_index(v, element).map(|i| v.remove(i))
}

pub struct Root<A, N, F>
    where N: Widget<A, F> + 'static,
          A: 'static,
          F: RenderFrame + 'static
{
    // Event handing and dispatch
    event_translator: EventTranslator<A>,

    // Input State
    input_state: InputState,

    widget_traverser_base: WidgetTraverserBase<A, F>,

    timer_list: TimerList,
    update_state: Rc<UpdateStateCell>,

    // User data
    pub root_widget: N,
    pub theme: F::Theme,
}

struct InputState {
    mouse_pos: Option<Point2<i32>>,
    mouse_buttons_down: MouseButtonSequenceTrackPos,
    modifiers: ModifierKeys,
    keys_down: Vec<Key>,
    mouse_hover_widget: Option<WidgetID>,
    focused_widget: Option<WidgetID>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    MouseMove(Point2<i32>),
    MouseEnter,
    MouseExit,
    MouseDown(MouseButton),
    MouseUp(MouseButton),
    MouseScrollLines(Vector2<i32>),
    MouseScrollPx(Vector2<i32>),
    WindowResize(DimsBox<D2, u32>),
    KeyDown(Key),
    KeyUp(Key),
    Char(char),
    Timer,
    Redraw
}

/// Whether to continue or abort a loop.
#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopFlow {
    /// Continue the loop.
    Continue,
    /// Abort the loop.
    Break
}

#[must_use]
pub struct FrameEventProcessor<'a, A, F>
    where A: 'static,
          F: RenderFrame + 'static
{
    timer_list: &'a mut TimerList,
    input_state: &'a mut InputState,
    event_translator: &'a mut EventTranslator<A>,
    update_state: Rc<UpdateStateCell>,
    widget_traverser: WidgetTraverser<'a, A, F>,
}

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLoopResult {
    pub wait_until_call_timer: Option<Instant>,
}

impl InputState {
    fn new() -> InputState {
        InputState {
            mouse_pos: None,
            mouse_buttons_down: MouseButtonSequenceTrackPos::new(),
            modifiers: ModifierKeys::empty(),
            keys_down: Vec::new(),
            mouse_hover_widget: None,
            focused_widget: None
        }
    }
}

impl<A, N, F> Root<A, N, F>
    where N: Widget<A, F>,
          F: RenderFrame
{
    #[inline]
    pub fn new(mut root_widget: N, theme: F::Theme, dims: DimsBox<D2, u32>) -> Root<A, N, F> {
        // TODO: DRAW ROOT AND DO INITIAL LAYOUT
        *root_widget.rect_mut() = dims.cast().unwrap_or(DimsBox::max_value()).into();
        Root {
            event_translator: EventTranslator::new(),

            input_state: InputState::new(),

            widget_traverser_base: WidgetTraverserBase::new(root_widget.widget_tag().widget_id),

            timer_list: TimerList::new(None),
            update_state: UpdateState::new(),

            root_widget, theme,
        }
    }

    pub fn drain_actions(&mut self) -> impl '_ + Iterator<Item=A> + ExactSizeIterator + DoubleEndedIterator {
        self.event_translator.drain_actions()
    }

    pub fn start_frame(&mut self) -> FrameEventProcessor<'_, A, F> {
        FrameEventProcessor {
            timer_list: &mut self.timer_list,
            input_state: &mut self.input_state,
            event_translator: &mut self.event_translator,
            update_state: self.update_state.clone(),
            widget_traverser: self.widget_traverser_base.with_root_ref(&mut self.root_widget, self.update_state.clone())
        }
    }

    pub fn relayout(&mut self) {
        let mut widget_traverser = self.widget_traverser_base.with_root_ref(&mut self.root_widget, self.update_state.clone());

        let mut relayout_widgets = Vec::new();

        let global_update = self.update_state.borrow().global_update;
        while global_update || self.update_state.borrow().relayout.len() > 0 {
            match global_update {
                false => relayout_widgets.extend(self.update_state.borrow_mut().relayout.drain()),
                true => {
                    self.update_state.borrow_mut().relayout.clear();
                    relayout_widgets.extend(widget_traverser.all_widgets());
                }
            }

            let valid_len = widget_traverser.sort_widgets_by_depth(&mut relayout_widgets).len();
            relayout_widgets.truncate(valid_len);

            for i in 0..valid_len {
                let widget_id = relayout_widgets[i];

                // Ignore any duplicate Widget IDs.
                if Some(widget_id) == relayout_widgets.get(i.wrapping_sub(1)).cloned() {
                    continue;
                }

                let WidgetPath{mut widget, ..} = match widget_traverser.get_widget(widget_id) {
                    Some(path) => path,
                    None => continue
                };

                let old_widget_rect = widget.rect();
                widget.update_layout(&self.theme);
                let size_bounds = widget.size_bounds();
                let new_widget_rect = widget.rect();
                let widget_dims = new_widget_rect.dims();
                drop(widget);

                // If we're doing a global update, all widgets are in the relayout list so we don't
                // need to queue the part for relayout. Otherwise, queue the parent for relayout if
                // the widget's rect has changed or the widget's dimensions no longer fall in its size
                // bounds.
                let parent_needs_relayout =
                    size_bounds.bound_rect(widget_dims) != widget_dims ||
                    old_widget_rect != new_widget_rect;

                if !global_update && parent_needs_relayout {
                    if let Some(WidgetPath{widget_id: parent_id, ..}) = widget_traverser.get_widget_relation(widget_id, Relation::Parent) {
                        // This can push duplicate relayout requests to the `relayout_widgets` queue
                        // if multiple children aren't in their size bounds. We handle that above.
                        relayout_widgets.push(parent_id);
                    }
                }
            }

            // Remove all re-layed-out widgets from the list.
            relayout_widgets.drain(..valid_len);

            if global_update {
                break;
            }
        }
    }

    pub fn redraw<R>(&mut self, renderer: &mut R)
        where R: Renderer<Frame=F>
    {
        let root_rect = self.root_widget.rect();
        let new_dims = root_rect.dims().cast::<u32>().unwrap_or(DimsBox::new2(0, 0));
        if new_dims != renderer.dims() {
            renderer.resized(new_dims);
        }

        let Root {
            ref update_state,
            ref mut widget_traverser_base,
            ref mut root_widget,
            ref theme,
            ..
        } = *self;

        let mut update_state = update_state.borrow_mut();
        if update_state.global_update || update_state.redraw.len() > 0 {
            // We should probably support incremental redraw at some point but not doing that is
            // soooo much easier.
            update_state.redraw.clear();
            update_state.reset_global_update();
            drop(update_state);

            let (mut frame, window_rect) = renderer.make_frame();

            let mut widget_traverser = widget_traverser_base.with_root_ref(root_widget, self.update_state.clone());
            widget_traverser.crawl_widgets(|mut path| {
                let widget_rect = path.widget.rect();
                let mut render_frame_clipped = RenderFrameClipped {
                    frame: frame,
                    transform: path.widget.rect(),
                    clip: path.widget.clip().unwrap_or(window_rect),
                    theme: theme
                };

                path.widget.render(&mut render_frame_clipped);
            });

            renderer.finish_frame(theme);
        }

    }
}

impl<A, F> FrameEventProcessor<'_, A, F>
    where F: RenderFrame
{
    pub fn process_event(
        &mut self,
        event: WindowEvent,
        mut bubble_fallthrough: impl FnMut(WidgetEvent, &[WidgetIdent]) -> Option<A>
    ) {
        let FrameEventProcessor {
            ref mut timer_list,
            ref mut input_state,
            ref mut event_translator,
            ref update_state,
            ref mut widget_traverser,
        } = *self;

        event_translator
            .with_data(
                timer_list,
                widget_traverser,
                input_state,
                update_state.clone(),
            )
            .translate_window_event(event);
    }

    pub fn set_modifiers(&mut self, modifiers: ModifierKeys) {
        self.input_state.modifiers = modifiers;
    }

    pub fn finish(mut self) -> EventLoopResult {
        let mut update_state = self.update_state.borrow_mut();

        for remove_id in update_state.remove_from_tree.drain() {
            // Because this removes the widget and all child widgets, this could cause some issues
            // if a widget removal causes the removal of child widgets that haven't actually been
            // destroyed.
            //
            // TODO: INVESTIGATE FURTHER FOR POTENTIAL BUGS
            self.widget_traverser.remove_widget(remove_id);
        }

        EventLoopResult {
            wait_until_call_timer: None
        }
    }
}
