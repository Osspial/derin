// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    event::{Key, ModifierKeys, WidgetEvent, FocusChange, InputState, MouseHoverChange},
    theme::CursorIcon,
};
use clipboard::{ClipboardContext, ClipboardProvider};
use cgmath_geometry::line::Segment;
use derin_core::render::{CursorData, CursorOp};

pub trait CharFilter {
    fn char_allowed(&mut self, c: char) -> bool;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefaultCharFilter;
impl CharFilter for DefaultCharFilter {
    #[inline(always)]
    fn char_allowed(&mut self, c: char) -> bool {
        match c {
            '\t' |
            '\r' |
            '\n' => true,
            _ => !c.is_control()
        }
    }
}


#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineCharFilter;
impl CharFilter for LineCharFilter {
    #[inline(always)]
    fn char_allowed(&mut self, c: char) -> bool {
        match c {
            '\r' |
            '\n' => false,
            _ => DefaultCharFilter.char_allowed(c)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorFlashOp {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextEditOps {
    pub allow_bubble: bool,
    pub redraw: bool,
    pub focus: Option<FocusChange>,
    pub cursor_flash: Option<CursorFlashOp>,
    pub cursor_icon: Option<CursorIcon>,
}

#[derive(Default, Debug, Clone)]
pub struct TextEditAssist<C = DefaultCharFilter>
    where C: CharFilter
{
    pub string: String,
    pub cursor_data: CursorData,
    pub cursor_ops: Vec<CursorOp>,
    pub filter: C,
}

impl<C> TextEditAssist<C>
    where C: CharFilter
{
    pub fn adapt_event(&mut self, event: &WidgetEvent, input_state: InputState) -> TextEditOps {
        use self::WidgetEvent::*;
        use derin_common_types::buttons::MouseButton;

        let mut focus = None;
        let mut cursor_icon = None;
        let mut allow_bubble = true;
        let mut redraw = false;
        let mut cursor_flash = None;

        match *event {
            KeyDown(key, modifiers) => loop {
                allow_bubble = false;
                let jump_to_word_boundaries = modifiers.contains(ModifierKeys::CTRL);
                match (key, modifiers) {
                    (Key::LArrow, _) => self.cursor_ops.push(CursorOp::MoveHorizontal {
                        delta: -1,
                        expand_selection: modifiers.contains(ModifierKeys::SHIFT),
                        jump_to_word_boundaries,
                    }),
                    (Key::RArrow, _) => self.cursor_ops.push(CursorOp::MoveHorizontal {
                        delta: 1,
                        expand_selection: modifiers.contains(ModifierKeys::SHIFT),
                        jump_to_word_boundaries,
                    }),
                    (Key::UArrow, _) => self.cursor_ops.push(CursorOp::MoveVertical {
                        delta: -1,
                        expand_selection: modifiers.contains(ModifierKeys::SHIFT),
                    }),
                    (Key::DArrow, _) => self.cursor_ops.push(CursorOp::MoveVertical {
                        delta: 1,
                        expand_selection: modifiers.contains(ModifierKeys::SHIFT),
                    }),
                    (Key::A, ModifierKeys::CTRL) => self.cursor_ops.push(CursorOp::SelectAll),

                    // This implementation has a bug - if any `CursorOp`s has been submitted earlier in
                    // the same frame that produced these cut/copy/paste events, the ops will be ignored
                    // when performing the clipboard operation. However, as far as I can tell the only
                    // way to fix that is to add `Cut`/`Copy`/`Paste` events to `CursorOp`, which I'm
                    // presently against.
                    (Key::C, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let new_contents = self.string[self.cursor_data.highlight_range.clone()].to_string();
                            clipboard.set_contents(new_contents).ok();
                        }
                    },
                    (Key::V, ModifierKeys::CTRL) => {
                        if let Ok(clipboard_contents) = ClipboardContext::new().and_then(|mut c| c.get_contents()) {
                            self.cursor_ops.push(CursorOp::InsertString(clipboard_contents));
                        }
                    },
                    (Key::X, ModifierKeys::CTRL) => {
                        if let Ok(mut clipboard) = ClipboardContext::new() {
                            let new_contents = self.string[self.cursor_data.highlight_range.clone()].to_string();
                            clipboard.set_contents(new_contents).ok();
                            self.cursor_ops.push(CursorOp::DeleteSelection);
                        }
                    },
                    (Key::Back, _) => self.cursor_ops.push(CursorOp::DeleteChars {
                        dist: -1,
                        jump_to_word_boundaries,
                    }),
                    (Key::Delete, _) => self.cursor_ops.push(CursorOp::DeleteChars {
                        dist: 1,
                        jump_to_word_boundaries,
                    }),
                    _ => break
                }
                redraw = true;
                cursor_flash = Some(CursorFlashOp::Start);
                break;
            },
            KeyUp(..) => allow_bubble = false,
            Char(c) => if self.filter.char_allowed(c) {
                allow_bubble = false;
                self.cursor_ops.push(CursorOp::InsertChar(c));
                redraw = true;
                cursor_flash = Some(CursorFlashOp::Start);
            }
            MouseDown{in_widget: true, button, pos} => {
                focus = Some(FocusChange::Take);
                if button == MouseButton::Left {
                    self.cursor_ops.push(CursorOp::SelectOnSegment(Segment::new(pos, pos)));
                    redraw = true;
                    cursor_flash = Some(CursorFlashOp::Start);
                }
            },
            MouseDown{in_widget: false, ..} => {
                focus = Some(FocusChange::Remove);
                self.cursor_data.draw_cursor = false;
                redraw = true;
                cursor_flash = Some(CursorFlashOp::Start);
            },
            MouseMove{new_pos, ref hover_change, ..} => {
                match hover_change {
                    Some(MouseHoverChange::Enter) => cursor_icon = Some(CursorIcon::Text),
                    Some(MouseHoverChange::Exit) => cursor_icon = Some(CursorIcon::default()),
                    _ => ()
                }
                if let Some(down) = input_state.mouse_buttons_down_in_widget.iter().find(|d| d.button == MouseButton::Left) {
                    self.cursor_ops.push(CursorOp::SelectOnSegment(Segment::new(down.down_pos, new_pos)));
                    redraw = true;
                }
            },
            GainFocus(_, _) => {
                redraw = true;
                cursor_flash = Some(CursorFlashOp::Start);
            }
            LoseFocus => {
                self.cursor_ops.push(CursorOp::UnselectAll);
                redraw = true;
                cursor_flash = Some(CursorFlashOp::End);
            },
            _ => ()
        };
        TextEditOps {
            allow_bubble,
            redraw,
            cursor_flash,
            cursor_icon,
            focus
        }
    }
}
