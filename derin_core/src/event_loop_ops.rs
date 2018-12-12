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


use crate::{WindowEvent, Root};
use crate::tree::*;
use crate::popup::{PopupSummary, PopupID};
use crate::event::WidgetEvent;
use crate::render::{Renderer, RenderFrame};

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
        _event_popup_id: Option<PopupID>,
        _event: WindowEvent,
        _bubble_fallthrough: &mut FnMut(WidgetEvent, &[WidgetIdent]) -> Option<A>
    ) -> EventLoopResult {
        // match event {
        //     WindowEvent::MouseEvent =>
        // }

        unimplemented!()
    }

    pub fn redraw<R>(&mut self, _with_renderer: impl FnMut(Option<PopupID>, &mut FnMut(&mut R)))
        where R: Renderer<Frame=F>
    {
        unimplemented!()
        // let Root {
        //     id: root_id,
        //     needs_redraw: ref mut root_needs_redraw,
        //     ref mut root_widget,
        //     ref mut theme,
        //     ref mut cursor_icon,
        //     ref mut popup_widgets,
        //     ref mut set_cursor_icon,
        //     ref mut set_cursor_pos,
        //     ref mut timer_list,
        //     ..
        // } = *self;

        // let mut redraw = |redraw_widget: &mut Widget<A, F>, renderer: &mut R, needs_redraw: &mut bool| {
        //     let mut root_update = redraw_widget.widget_tag().needs_update(root_id);
        //     let mark_active_widgets_redraw = root_update.needs_redraw();

        //     if let Some(cursor_pos) = *set_cursor_pos {
        //         renderer.set_cursor_pos(cursor_pos);
        //     }
        //     if let Some(set_icon) = *set_cursor_icon {
        //         if set_icon != *cursor_icon {
        //             renderer.set_cursor_icon(set_icon);
        //             *cursor_icon = set_icon;
        //         }
        //     }
        //     *set_cursor_pos = None;
        //     *set_cursor_icon = None;

        //     let new_dims = redraw_widget.rect().dims().cast::<u32>().unwrap_or(DimsBox::new2(0, 0));
        //     if new_dims != renderer.dims() {
        //         renderer.resized(new_dims);
        //     }

        //     // Draw the widget tree.
        //     if mark_active_widgets_redraw || *needs_redraw {
        //         let force_full_redraw = *needs_redraw || renderer.force_full_redraw();

        //         root_update.render_self |= force_full_redraw;
        //         root_update.update_child |= force_full_redraw;

        //         if root_update.render_self || root_update.update_child {
        //             macro_rules! render {
        //                 ($prerender_pass:expr) => {{
        //                     let (frame, base_transform) = renderer.make_frame(!$prerender_pass);
        //                     let mut frame_rect_stack = FrameRectStack::new(frame, base_transform, theme);

        //                     if root_update.render_self {
        //                         redraw_widget.render(&mut frame_rect_stack);
        //                     }
        //                     if root_update.update_child {
        //                         if let Some(root_as_parent) = redraw_widget.as_parent_mut() {
        //                             WidgetRenderer {
        //                                 root_id: root_id,
        //                                 frame: frame_rect_stack,
        //                                 force_full_redraw: force_full_redraw && !$prerender_pass,
        //                                 prerender_pass: $prerender_pass,
        //                                 theme
        //                             }.render_widget_children(root_as_parent)
        //                         }
        //                     }
        //                 }}
        //             }

        //             // Before we do rendering proper, a "prerendering pass" is performed. This
        //             // is done because some elements, like text, compute their minimum size
        //             // when rendering, which factors into layout calculations.
        //             render!(true);
        //             if let Some(root_as_parent) = redraw_widget.as_parent_mut() {
        //                 update_widget_layout(root_id, force_full_redraw, timer_list, root_as_parent);
        //                 root_as_parent.widget_tag().unrequest_relayout();
        //             }

        //             // Do the proper rendering pass.
        //             render!(false);

        //             renderer.finish_frame(theme);
        //             redraw_widget.widget_tag().mark_updated(root_id);
        //         }
        //     }

        //     renderer.set_size_bounds(redraw_widget.size_bounds());
        //     *needs_redraw = false;
        // };

        // with_renderer(None, &mut |renderer| redraw(root_widget as &mut Widget<A, F>, renderer, root_needs_redraw));
        // for (id, popup) in popup_widgets.popups_mut() {
        //     with_renderer(Some(id), &mut |renderer| redraw(&mut *popup.widget, renderer, &mut popup.needs_redraw));
        // }
    }
}

// fn update_widget_layout<A: 'static, F: RenderFrame>(root_id: RootID, force_full_redraw: bool, timer_list: &mut TimerList, widget: &mut ParentDyn<A, F>) -> bool {
//     // Loop to re-solve widget layout, if children break their size bounds. Is 0..4 so that
//     // it doesn't enter an infinite loop if children can never be properly solved.
//     for _ in 0..4 {
//         let Update {
//             update_child,
//             update_layout,
//             update_layout_post,
//             ..
//         } = widget.widget_tag().needs_update(root_id);

//         if update_layout || force_full_redraw {
//             widget.update_child_layout();
//         }

//         let mut children_break_bounds = false;
//         if update_child || force_full_redraw {
//             widget.children_mut(&mut |children_summaries| {
//                 for mut summary in children_summaries {
//                     let WidgetSummary {
//                         widget: ref mut child_widget,
//                         ..
//                     } = summary;

//                     let mut update_successful = true;
//                     if let Some(child_widget_as_parent) = child_widget.as_parent_mut() {
//                         update_successful = !update_widget_layout(root_id, force_full_redraw, timer_list, child_widget_as_parent);
//                         children_break_bounds |= !update_successful;
//                     }

//                     if update_successful {
//                         child_widget.widget_tag().unrequest_relayout();
//                     }
//                 }

//                 LoopFlow::Continue
//             });
//         }

//         if update_layout_post {
//             widget.update_child_layout();
//         }

//         if !children_break_bounds {
//             break;
//         }
//     }

//     let widget_rect = widget.rect().dims();
//     widget.size_bounds().bound_rect(widget_rect) != widget_rect
// }
