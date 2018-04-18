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

use widgets::assistants::text_edit::{TextEditAssist, TextEditOps, LineCharFilter};
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, WidgetTag, Widget};
use core::render::{FrameRectStack, Theme};
use core::popup::ChildPopupsMut;
use core::timer::TimerRegister;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};
use derin_common_types::layout::SizeBounds;

use gl_render::{ThemedPrim, PrimFrame, RenderString, EditString, RelPoint, Prim};

use std::time::Duration;

use arrayvec::ArrayVec;

/// Multi-line editable text widget.
#[derive(Debug, Clone)]
pub struct EditBox {
    update_tag: WidgetTag,
    bounds: BoundBox<Point2<i32>>,
    edit: TextEditAssist,
    min_size: DimsBox<Point2<i32>>
}

/// Single-line editable text widget.
#[derive(Debug, Clone)]
pub struct LineBox {
    update_tag: WidgetTag,
    bounds: BoundBox<Point2<i32>>,
    edit: TextEditAssist<LineCharFilter>,
    min_size: DimsBox<Point2<i32>>
}

impl EditBox {
    /// Create a new `EditBox`, containing the included `String` by default.
    pub fn new(string: String) -> EditBox {
        EditBox {
            update_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            edit: TextEditAssist {
                string: EditString::new(RenderString::new(string)),
                ..TextEditAssist::default()
            },
            min_size: DimsBox::new2(0, 0)
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
        self.update_tag.mark_render_self();
        self.edit.string.render_string.string_mut()
    }
}

impl LineBox {
    /// Create a new `LineBox`, containing the included `String` by default.
    pub fn new(string: String) -> LineBox {
        LineBox {
            update_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            edit: TextEditAssist {
                string: EditString::new(RenderString::new(string)),
                ..TextEditAssist::default()
            },
            min_size: DimsBox::new2(0, 0)
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
        self.update_tag.mark_render_self();
        self.edit.string.render_string.string_mut()
    }
}

macro_rules! render_and_event {
    ($ty:ty) => {
            fn render(&mut self, frame: &mut FrameRectStack<F>) {
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

            fn on_widget_event(&mut self, event: WidgetEvent, input_state: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
                use self::WidgetEvent::*;

                let TextEditOps {
                    allow_bubble,
                    redraw,
                    cursor_flash,
                    cursor_icon,
                    focus,
                } = self.edit.adapt_event(&event, input_state);
                if cursor_flash.is_some() {
                    self.update_tag.mark_update_timer();
                }
                if redraw {
                    self.update_tag.mark_render_self();
                }

                match event {
                    Timer{name: "cursor_flash", times_triggered, ..} => {
                        self.edit.string.draw_cursor = times_triggered % 2 == 0;
                        self.update_tag.mark_render_self();
                    },
                    _ => ()
                };
                EventOps {
                    action: None,
                    focus,
                    bubble: allow_bubble && event.default_bubble(),
                    cursor_pos: None,
                    cursor_icon,
                    popup: None
                }
            }

            fn register_timers(&self, register: &mut TimerRegister) {
                if self.update_tag.has_keyboard_focus() {
                    register.add_timer("cursor_flash", Duration::new(1, 0)/2, true);
                }
            }
    }
}

impl<A, F> Widget<A, F> for EditBox
    where F: PrimFrame
{
    #[inline]
    fn update_tag(&self) -> &WidgetTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::new_min(self.min_size)
    }

    render_and_event!(EditBox);
}

impl<A, F> Widget<A, F> for LineBox
    where F: PrimFrame
{
    #[inline]
    fn update_tag(&self) -> &WidgetTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        &mut self.bounds
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        SizeBounds {
            min: self.min_size,
            max: DimsBox::new2(i32::max_value(), self.min_size.height())
        }
    }

    render_and_event!(LineBox);
}
