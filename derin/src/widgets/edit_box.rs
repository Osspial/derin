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

use core::event::{EventOps, WidgetEvent, InputState, FocusChange};
use core::tree::{WidgetIdent, UpdateTag, Widget};
use core::render::{FrameRectStack, Theme};
use core::popup::ChildPopupsMut;
use core::timer::TimerRegister;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox, Segment};
use dct::layout::SizeBounds;
use dct::cursor::CursorIcon;
use dct::buttons::{Key, ModifierKeys};

use gl_render::{ThemedPrim, PrimFrame, RenderString, EditString, RelPoint, Prim};

use std::time::Duration;

use clipboard::{ClipboardContext, ClipboardProvider};
use arrayvec::ArrayVec;

/// Multi-line editable text widget.
#[derive(Debug, Clone)]
pub struct EditBox {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    string: EditString,
    size_bounds: SizeBounds
}

impl EditBox {
    /// Create a new `EditBox`, containing the included `String` by default.
    pub fn new(string: String) -> EditBox {
        EditBox {
            update_tag: UpdateTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            string: EditString::new(RenderString::new(string)),
            size_bounds: SizeBounds::default()
        }
    }

    /// Retrieves a reference to the string stored within the `EditBox`.
    pub fn string(&self) -> &str {
        self.string.render_string.string()
    }

    /// Retrieves the `String` stored in the `EditBox`, for mutation.
    ///
    /// Calling this function forces the box to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn string_mut(&mut self) -> &mut String {
        self.update_tag.mark_render_self();
        self.string.render_string.string_mut()
    }
}

impl<A, F> Widget<A, F> for EditBox
    where F: PrimFrame
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
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
        self.size_bounds
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives(ArrayVec::from([
            ThemedPrim {
                theme_path: "EditBox",
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
                theme_path: "EditBox",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::EditString(&mut self.string),
                rect_px_out: None
            }
        ]).into_iter());

        self.size_bounds.min = frame.theme().widget_theme("EditBox").image.map(|i| i.min_size()).unwrap_or(DimsBox::new2(0, 0));
        let render_string_min = self.string.render_string.min_size();
        self.size_bounds.min.dims.y += render_string_min.height();
    }

    fn on_widget_event(&mut self, event: WidgetEvent, input_state: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        use self::WidgetEvent::*;
        use dct::buttons::MouseButton;

        let allow_char = |c| match c {
            '\t' |
            '\r' |
            '\n' => true,
            _ => !c.is_control()
        };
        let mut focus = None;
        let mut cursor_icon = None;
        let mut allow_bubble = true;
        match event {
            KeyDown(key, modifiers) => loop {
                allow_bubble = false;
                let jump_to_word_boundaries = modifiers.contains(ModifierKeys::CTRL);
                match (key, modifiers) {
                    (Key::LArrow, _) => self.string.move_cursor_horizontal(
                        -1,
                        jump_to_word_boundaries,
                        modifiers.contains(ModifierKeys::SHIFT)
                    ),
                    (Key::RArrow, _) => self.string.move_cursor_horizontal(
                        1,
                        jump_to_word_boundaries,
                        modifiers.contains(ModifierKeys::SHIFT)
                    ),
                    (Key::UArrow, _) => self.string.move_cursor_vertical(-1, modifiers.contains(ModifierKeys::SHIFT)),
                    (Key::DArrow, _) => self.string.move_cursor_vertical(1, modifiers.contains(ModifierKeys::SHIFT)),
                    (Key::A, ModifierKeys::CTRL) => self.string.select_all(),
                    (Key::C, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let select_range = self.string.highlight_range();
                            clipboard.set_contents(self.string.render_string.string()[select_range].to_string()).ok();
                        }
                    },
                    (Key::V, ModifierKeys::CTRL) => {
                        if let Ok(clipboard_conents) = ClipboardContext::new().and_then(|mut c| c.get_contents()) {
                            self.string.insert_str(&clipboard_conents);
                        }
                    },
                    (Key::X, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let highlight_range = self.string.highlight_range();
                            clipboard.set_contents(self.string.render_string.string()[highlight_range.clone()].to_string()).ok();
                            if highlight_range.len() > 0 {
                                self.string.delete_chars(1, false);
                            }
                        }
                    },
                    (Key::Back, _) => self.string.delete_chars(-1, jump_to_word_boundaries),
                    (Key::Delete, _) => self.string.delete_chars(1, jump_to_word_boundaries),
                    _ => break
                }
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
                break;
            },
            KeyUp(..) => allow_bubble = false,
            Char(c) if allow_char(c) => {
                allow_bubble = false;
                self.string.insert_char(c);
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
            }
            MouseDown{in_widget: true, button, pos} => {
                focus = Some(FocusChange::Take);
                if button == MouseButton::Left {
                    self.string.select_on_line(Segment::new(pos, pos));
                    self.update_tag
                        .mark_render_self()
                        .mark_update_timer();
                }
            },
            MouseUp{button: MouseButton::Left, ..} => {
                self.update_tag.mark_render_self();
            }
            MouseDown{in_widget: false, ..} => {
                focus = Some(FocusChange::Remove);
                self.string.draw_cursor = false;
                self.update_tag
                    .mark_render_self()
                    .mark_update_timer();
            },
            MouseMove{new_pos, ..} => {
                if let Some(down) = input_state.mouse_buttons_down_in_widget.iter().find(|d| d.button == MouseButton::Left) {
                    self.string.select_on_line(Segment::new(down.down_pos, new_pos));
                    self.update_tag.mark_render_self();
                }
            },
            MouseEnter{..} => cursor_icon = Some(CursorIcon::Text),
            MouseExit{..} => cursor_icon = Some(CursorIcon::default()),
            GainFocus  |
            LoseFocus => {
                self.string.deselect_all();
                self.update_tag.mark_update_timer();
            },
            Timer{name: "cursor_flash", times_triggered, ..} => {
                self.string.draw_cursor = times_triggered % 2 == 0;
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
