// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    core::{
        event::{EventOps, WidgetEvent, WidgetEventSourced, InputState},
        timer::{Timer, TimerID},
        widget::{WidgetTag, WidgetRender, Widget},
        render::{RenderFrameClipped, Theme},
    },
    gl_render::{ThemedPrim, PrimFrame, RenderString, EditString, RelPoint, Prim},
    widgets::assistants::text_edit::{TextEditAssist, TextEditOps, CursorFlashOp, LineCharFilter},
};
use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};
use derin_common_types::layout::SizeBounds;
use std::time::Duration;
use arrayvec::ArrayVec;

/// Multi-line editable text widget.
#[derive(Debug, Clone)]
pub struct EditBox {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    edit: TextEditAssist,
    min_size: DimsBox<D2, i32>,
    flash_timer: Option<TimerID>,
}

/// Single-line editable text widget.
#[derive(Debug, Clone)]
pub struct LineBox {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    edit: TextEditAssist<LineCharFilter>,
    min_size: DimsBox<D2, i32>,
    flash_timer: Option<TimerID>,
}

impl EditBox {
    /// Create a new `EditBox`, containing the included `String` by default.
    pub fn new(string: String) -> EditBox {
        EditBox {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            edit: TextEditAssist {
                string: EditString::new(RenderString::new(string)),
                ..TextEditAssist::default()
            },
            min_size: DimsBox::new2(0, 0),
            flash_timer: None,
        }
    }

    /// Retrieves a reference to the string stored within the `EditBox`.
    pub fn string(&self) -> &str {
        self.edit.string.render_string.string()
    }

    /// Retrieves the `String` stored in the `EditBox`, for mutation.
    ///
    /// Calling this function forces the box to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn string_mut(&mut self) -> &mut String {
        self.widget_tag.request_redraw();
        self.edit.string.render_string.string_mut()
    }
}

impl LineBox {
    /// Create a new `LineBox`, containing the included `String` by default.
    pub fn new(string: String) -> LineBox {
        LineBox {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            edit: TextEditAssist {
                string: EditString::new(RenderString::new(string)),
                ..TextEditAssist::default()
            },
            min_size: DimsBox::new2(0, 0),
            flash_timer: None,
        }
    }

    /// Retrieves a reference to the string stored within the `LineBox`.
    pub fn string(&self) -> &str {
        self.edit.string.render_string.string()
    }

    /// Retrieves the `String` stored in the `LineBox`, for mutation.
    ///
    /// Calling this function forces the box to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn string_mut(&mut self) -> &mut String {
        self.widget_tag.request_redraw();
        self.edit.string.render_string.string_mut()
    }
}

macro_rules! render {
    ($ty:ty) => {
        impl<F: PrimFrame> WidgetRender<F> for $ty {
            fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
                frame.upload_primitives(ArrayVec::from([
                    ThemedPrim {
                        theme_path: stringify!($ty),
                        min: Point2::new(
                            RelPoint::new(-1.0, 0),
                            RelPoint::new(-1.0, 0),
                        ),
                        max: Point2::new(
                            RelPoint::new( 1.0, 0),
                            RelPoint::new( 1.0, 0)
                        ),
                        prim: Prim::Image,
                        rect_px_out: None
                    },
                    ThemedPrim {
                        theme_path: stringify!($ty),
                        min: Point2::new(
                            RelPoint::new(-1.0, 0),
                            RelPoint::new(-1.0, 0),
                        ),
                        max: Point2::new(
                            RelPoint::new( 1.0, 0),
                            RelPoint::new( 1.0, 0)
                        ),
                        prim: Prim::EditString(&mut self.edit.string),
                        rect_px_out: None
                    }
                ]).into_iter());

                self.min_size = frame.theme().widget_theme(stringify!($ty)).image.map(|i| i.min_size()).unwrap_or(DimsBox::new2(0, 0));
                let render_string_min = self.edit.string.render_string.min_size();
                self.min_size.dims.y += render_string_min.height();
            }
        }
    }
}

macro_rules! event {
    ($ty:ty) => {
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
                    let timer_id = TimerID::new();
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
                    self.edit.string.draw_cursor = times_triggered % 2 == 0;
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
        SizeBounds::new_min(self.min_size)
    }

    event!(EditBox);
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
        SizeBounds {
            min: self.min_size,
            max: DimsBox::new2(i32::max_value(), self.min_size.height())
        }
    }

    event!(LineBox);
}

render!(EditBox);
render!(LineBox);
