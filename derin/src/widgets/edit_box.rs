// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    event::{EventOps, WidgetEvent, WidgetEventSourced, InputState},
    timer::{Timer, TimerId},
    widget::{WidgetTag, WidgetRender, Widget},
    render::{Renderer, RendererLayout, SubFrame, WidgetTheme},
};
use crate::widgets::assistants::text_edit::{TextEditAssist, TextEditOps, CursorFlashOp, LineCharFilter};
use cgmath_geometry::{D2, rect::BoundBox};
use derin_common_types::layout::SizeBounds;
use std::time::Duration;

/// Multi-line editable text widget.
#[derive(Debug, Clone)]
pub struct EditBox {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    edit: TextEditAssist,
    size_bounds: SizeBounds,
    flash_timer: Option<TimerId>,
}

/// Single-line editable text widget.
#[derive(Debug, Clone)]
pub struct LineBox {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    edit: TextEditAssist<LineCharFilter>,
    size_bounds: SizeBounds,
    flash_timer: Option<TimerId>,
}

impl EditBox {
    /// Create a new `EditBox`, containing the included `String` by default.
    pub fn new(string: String) -> EditBox {
        EditBox {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            edit: TextEditAssist {
                string,
                ..TextEditAssist::default()
            },
            size_bounds: SizeBounds::default(),
            flash_timer: None,
        }
    }

    /// Retrieves a reference to the string stored within the `EditBox`.
    pub fn string(&self) -> &str {
        &self.edit.string
    }

    /// Retrieves the `String` stored in the `EditBox`, for mutation.
    ///
    /// Calling this function forces the box to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn string_mut(&mut self) -> &mut String {
        self.widget_tag.request_redraw().request_relayout();
        &mut self.edit.string
    }
}

impl LineBox {
    /// Create a new `LineBox`, containing the included `String` by default.
    pub fn new(string: String) -> LineBox {
        LineBox {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            edit: TextEditAssist {
                string,
                ..TextEditAssist::default()
            },
            size_bounds: SizeBounds::default(),
            flash_timer: None,
        }
    }

    /// Retrieves a reference to the string stored within the `LineBox`.
    pub fn string(&self) -> &str {
        &self.edit.string
    }

    /// Retrieves the `String` stored in the `LineBox`, for mutation.
    ///
    /// Calling this function forces the box to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn string_mut(&mut self) -> &mut String {
        self.widget_tag.request_redraw().request_relayout();
        &mut self.edit.string
    }
}

macro_rules! render {
    ($ty:ty) => {
        impl<R: Renderer> WidgetRender<R> for $ty {
            fn render(&mut self, frame: &mut R::SubFrame) {
                frame.render_laid_out_content();
            }

            fn theme_list(&self) -> &[WidgetTheme] {
                const LABEL: &[WidgetTheme] = &[WidgetTheme::new(stringify!($ty))];
                LABEL
            }

            fn update_layout(&mut self, layout: &mut R::Layout) {
                layout.prepare_edit_string(
                    &mut self.edit.string,
                    &mut self.edit.cursor_data,
                    self.edit.cursor_ops.drain(..),
                );

                let result = layout.finish();
                self.size_bounds = result.size_bounds;
            }
        }
    }
}

macro_rules! event {
    () => {
        fn on_widget_event(&mut self, event: WidgetEventSourced, input_state: InputState) -> EventOps {
            let event = event.unwrap();

            let TextEditOps {
                allow_bubble,
                redraw,
                cursor_flash,
                cursor_icon,
                focus,
            } = self.edit.adapt_event(&event, input_state);

            match (cursor_flash, self.flash_timer) {
                (Some(CursorFlashOp::Start), None) => {
                    let timer_id = TimerId::new();
                    self.widget_tag.timers_mut().insert(timer_id, Timer::new(Duration::new(1, 0)/2));
                    self.flash_timer = Some(timer_id);
                },
                (Some(CursorFlashOp::End), Some(timer_id)) => {
                    self.widget_tag.timers_mut().remove(&timer_id);
                    self.flash_timer = None;
                },
                _ => ()
            }

            if redraw {
                self.widget_tag.request_redraw();
            }

            match event {
                WidgetEvent::Timer{timer_id, times_triggered, ..} if Some(timer_id) == self.flash_timer => {
                    self.edit.cursor_data.draw_cursor = times_triggered % 2 == 0;
                    self.widget_tag.request_redraw();
                },
                _ => ()
            };

            if let Some(cursor_icon) = cursor_icon {
                self.widget_tag.set_cursor_icon(cursor_icon).ok();
            }

            EventOps {
                focus,
                bubble: allow_bubble && event.default_bubble(),
            }
        }
    }
}

impl Widget for EditBox {
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        &mut self.bounds
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    event!();
}

impl Widget for LineBox {
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        &mut self.bounds
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    event!();
}

render!(EditBox);
render!(LineBox);
