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

use crate::cgmath::{Point2, Array, Bounded};
use cgmath_geometry::{rect::{GeoBox, DimsBox}, line::Segment};

use std::cmp::Ordering;

use crate::{WindowEvent, LoopFlow, Root};
use crate::tree::*;
use crate::tree::dynamic::*;
use crate::timer::{Timer, TimerList};
use crate::popup::{PopupSummary, PopupID};
use crate::event::{WidgetEvent, FocusChange};
use crate::render::{Renderer, RenderFrame, FrameRectStack};
use crate::widget_stack::{WidgetPath, WidgetStack};
use crate::meta_tracker::{MetaDrain, MetaEvent, MetaEventVariant};
use crate::offset_widget::*;

use std::time::Duration;

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLoopResult {
    pub wait_until_call_timer: Option<Duration>,
    pub popup_deltas: Vec<PopupDelta>
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupDelta {
    Create(PopupSummary),
    Remove(PopupID)
}

impl<A, N, F> Root<A, N, F>
    where N: Widget<A, F>,
          F: RenderFrame,
{
    pub fn process_event(
        &mut self,
        event: WindowEvent,
        mut bubble_fallthrough: impl FnMut(WidgetEvent, &[WidgetIdent]) -> Option<A>
    ) -> EventLoopResult {
        self.process_event_inner(None, event, &mut bubble_fallthrough)
    }
    pub fn process_popup_event(
        &mut self,
        popup_id: PopupID,
        event: WindowEvent,
        mut bubble_fallthrough: impl FnMut(WidgetEvent, &[WidgetIdent]) -> Option<A>
    ) -> EventLoopResult {
        self.process_event_inner(Some(popup_id), event, &mut bubble_fallthrough)
    }
    pub fn remove_popup(&mut self, popup_id: PopupID) {
        self.popup_widgets.remove(popup_id);
    }

    fn process_event_inner(
        &mut self,
        event_popup_id: Option<PopupID>,
        event: WindowEvent,
        bubble_fallthrough: &mut FnMut(WidgetEvent, &[WidgetIdent]) -> Option<A>
    ) -> EventLoopResult {
        unimplemented!()
    }

    pub fn redraw<R>(&mut self, mut with_renderer: impl FnMut(Option<PopupID>, &mut FnMut(&mut R)))
        where R: Renderer<Frame=F>
    {
        let Root {
            id: root_id,
            needs_redraw: ref mut root_needs_redraw,
            ref mut widget_ident_stack,
            ref mut root_widget,
            ref mut theme,
            ref mut cursor_icon,
            ref mut popup_widgets,
            ref mut set_cursor_icon,
            ref mut set_cursor_pos,
            ref mut timer_list,
            ..
        } = *self;

        let mut redraw = |redraw_widget: &mut Widget<A, F>, renderer: &mut R, needs_redraw: &mut bool| {
            let mut root_update = redraw_widget.widget_tag().needs_update(root_id);
            let mark_active_widgets_redraw = root_update.needs_redraw();

            if let Some(cursor_pos) = *set_cursor_pos {
                renderer.set_cursor_pos(cursor_pos);
            }
            if let Some(set_icon) = *set_cursor_icon {
                if set_icon != *cursor_icon {
                    renderer.set_cursor_icon(set_icon);
                    *cursor_icon = set_icon;
                }
            }
            *set_cursor_pos = None;
            *set_cursor_icon = None;

            let new_dims = redraw_widget.rect().dims().cast::<u32>().unwrap_or(DimsBox::new2(0, 0));
            if new_dims != renderer.dims() {
                renderer.resized(new_dims);
            }

            // Draw the widget tree.
            if mark_active_widgets_redraw || *needs_redraw {
                let force_full_redraw = *needs_redraw || renderer.force_full_redraw();

                root_update.render_self |= force_full_redraw;
                root_update.update_child |= force_full_redraw;

                if root_update.render_self || root_update.update_child {
                    macro_rules! render {
                        ($prerender_pass:expr) => {{
                            let (frame, base_transform) = renderer.make_frame(!$prerender_pass);
                            let mut frame_rect_stack = FrameRectStack::new(frame, base_transform, theme, widget_ident_stack);

                            if root_update.render_self {
                                redraw_widget.render(&mut frame_rect_stack);
                            }
                            if root_update.update_child {
                                if let Some(root_as_parent) = redraw_widget.as_parent_mut() {
                                    WidgetRenderer {
                                        root_id: root_id,
                                        frame: frame_rect_stack,
                                        force_full_redraw: force_full_redraw && !$prerender_pass,
                                        prerender_pass: $prerender_pass,
                                        theme
                                    }.render_widget_children(root_as_parent)
                                }
                            }
                        }}
                    }

                    // Before we do rendering proper, a "prerendering pass" is performed. This
                    // is done because some elements, like text, compute their minimum size
                    // when rendering, which factors into layout calculations.
                    render!(true);
                    if let Some(root_as_parent) = redraw_widget.as_parent_mut() {
                        update_widget_layout(root_id, force_full_redraw, timer_list, root_as_parent);
                        root_as_parent.widget_tag().unmark_update_layout();
                    }

                    // Do the proper rendering pass.
                    render!(false);

                    renderer.finish_frame(theme);
                    redraw_widget.widget_tag().mark_updated(root_id);
                }
            }

            renderer.set_size_bounds(redraw_widget.size_bounds());
            *needs_redraw = false;
        };

        with_renderer(None, &mut |renderer| redraw(root_widget as &mut Widget<A, F>, renderer, root_needs_redraw));
        for (id, popup) in popup_widgets.popups_mut() {
            with_renderer(Some(id), &mut |renderer| redraw(&mut *popup.widget, renderer, &mut popup.needs_redraw));
        }
    }
}

fn update_widget_timers<A: 'static, F: RenderFrame>(root_id: RootID, timer_list: &mut TimerList, widget: &mut Widget<A, F>) {
    let widget_tag = widget.widget_tag();
    let Update {
        update_child,
        update_timer,
        ..
    } = widget_tag.needs_update(root_id);

    if update_timer {
        widget_tag.unmark_update_timer();
        let mut register = timer_list.new_timer_register(widget_tag.widget_id);
        widget.register_timers(&mut register);
    }

    if update_child {
        if let Some(widget_as_parent) = widget.as_parent_mut() {
            widget_as_parent.children_mut(&mut |children_summaries| {
                for summary in children_summaries {
                    update_widget_timers(root_id, timer_list, summary.widget);
                }

                LoopFlow::Continue
            });
        }
    }

    // TODO: PROPERLY UNSET update_child IF ALL CHILDREN HAVE RECIEVED NECESSARY UPDATES
}

fn update_widget_layout<A: 'static, F: RenderFrame>(root_id: RootID, force_full_redraw: bool, timer_list: &mut TimerList, widget: &mut ParentDyn<A, F>) -> bool {
    // Loop to re-solve widget layout, if children break their size bounds. Is 0..4 so that
    // it doesn't enter an infinite loop if children can never be properly solved.
    for _ in 0..4 {
        let Update {
            update_child,
            update_layout,
            update_layout_post,
            ..
        } = widget.widget_tag().needs_update(root_id);

        if update_layout || force_full_redraw {
            widget.update_child_layout();
        }

        let mut children_break_bounds = false;
        if update_child || force_full_redraw {
            widget.children_mut(&mut |children_summaries| {
                for mut summary in children_summaries {
                    let WidgetSummary {
                        widget: ref mut child_widget,
                        ..
                    } = summary;

                    let mut update_successful = true;
                    if let Some(child_widget_as_parent) = child_widget.as_parent_mut() {
                        update_successful = !update_widget_layout(root_id, force_full_redraw, timer_list, child_widget_as_parent);
                        children_break_bounds |= !update_successful;
                    }

                    if update_successful {
                        child_widget.widget_tag().unmark_update_layout();
                    }
                }

                LoopFlow::Continue
            });
        }

        if update_layout_post {
            widget.update_child_layout();
        }

        if !children_break_bounds {
            break;
        }
    }

    let widget_rect = widget.rect().dims();
    widget.size_bounds().bound_rect(widget_rect) != widget_rect
}

struct WidgetRenderer<'a, F>
    where F: 'a + RenderFrame
{
    root_id: RootID,
    frame: FrameRectStack<'a, F>,
    force_full_redraw: bool,
    prerender_pass: bool,
    theme: &'a F::Theme
}

impl<'a, F> WidgetRenderer<'a, F>
    where F: 'a + RenderFrame
{
    fn render_widget_children<A: 'static>(&mut self, parent: &mut ParentDyn<A, F>) {
        parent.children_mut(&mut |children_summaries| {
            for mut summary in children_summaries {
                let WidgetSummary {
                    widget: ref mut child_widget,
                    ref ident,
                    ..
                } = summary;
                let child_rect = child_widget.rect();

                let mut root_update = child_widget.widget_tag().needs_update(self.root_id);
                root_update.render_self |= self.force_full_redraw;
                root_update.update_child |= self.force_full_redraw;
                let Update {
                    render_self,
                    update_child,
                    update_layout: _,
                    update_timer: _,
                    update_layout_post: _
                } = root_update;

                match child_widget.as_parent_mut() {
                    Some(child_widget_as_parent) => {
                        if let Some(mut child_frame) = self.frame.enter_child_widget(ident.clone()).enter_child_rect(child_rect) {
                            if render_self {
                                child_widget_as_parent.render(&mut child_frame);
                            }
                            if update_child {
                                WidgetRenderer {
                                    root_id: self.root_id,
                                    frame: child_frame,
                                    force_full_redraw: self.force_full_redraw,
                                    prerender_pass: self.prerender_pass,
                                    theme: self.theme
                                }.render_widget_children(child_widget_as_parent);
                            }
                        }
                    },
                    None => {
                        if render_self {
                            if let Some(mut child_frame) = self.frame.enter_child_widget(ident.clone()).enter_child_rect(child_rect) {
                                child_widget.render(&mut child_frame);
                            }
                        }
                    }
                }

                if !self.prerender_pass {
                    child_widget.widget_tag().mark_updated(self.root_id);
                }
            }

            LoopFlow::Continue
        });
    }
}
