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

use crate::widgets::assistants::ButtonState;
use crate::widgets::{Contents, ContentsInner, ToggleHandler};
use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use crate::core::event::{EventOps, InputState, WidgetEvent, WidgetEventSourced, MouseHoverChange};
use crate::core::tree::{WidgetIdent, WidgetTag, Widget};
use crate::core::render::RenderFrameClipped;
use derin_common_types::layout::SizeBounds;

use crate::gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame};

/// A toggleable box that can be either checked or unchecked.
///
/// When toggled, calls the [`change_state`] function in the associated handler passed in through the
/// `new` function.
///
/// [`change_state`]: ./trait.ToggleHandler.html
#[derive(Debug, Clone)]
pub struct CheckBox<H> {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,

    check_rect: BoundBox<D2, i32>,
    contents: ContentsInner,
    checked: bool,
    button_state: ButtonState,
    handler: H
}

impl<H> CheckBox<H> {
    /// Creates a new `CheckBox` with the given checked state, contents, and [toggle handler].
    ///
    /// [toggle handler]: ./trait.ToggleHandler.html
    pub fn new(checked: bool, contents: Contents<String>, handler: H) -> CheckBox<H> {
        CheckBox {
            widget_tag: WidgetTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),

            check_rect: BoundBox::new2(0, 0, 0, 0),
            contents: contents.to_inner(),
            button_state: ButtonState::Normal,
            checked, handler
        }
    }

    /// Retrieves the contents of the checkbox.
    pub fn contents(&self) -> Contents<&str> {
        self.contents.borrow()
    }

    /// Retrieves the contents of the checkbox, for mutation.
    ///
    /// Calling this function forces the checkbox to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> Contents<&mut String> {
        self.widget_tag.request_redraw();
        self.contents.borrow_mut()
    }

    /// Retrieves whether or not the checkbox is checked.
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Retrieves whether or not the checkbox is checked, for mutation.
    ///
    /// Calling this function forces the checkbox to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn checked_mut(&mut self) -> &mut bool {
        self.widget_tag.request_redraw();
        &mut self.checked
    }
}

impl<F, H> Widget<F> for CheckBox<H>
    where F: PrimFrame,
          H: ToggleHandler
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        &mut self.rect
    }

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::new_min(self.check_rect.dims())
    }

    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        let image_str = match (self.checked, self.button_state) {
            (true, ButtonState::Normal) => "CheckBox::Checked",
            (true, ButtonState::Hover) => "CheckBox::Checked::Hover",
            (true, ButtonState::Pressed) => "CheckBox::Checked::Pressed",
            (false, ButtonState::Normal) => "CheckBox::Empty",
            (false, ButtonState::Hover) => "CheckBox::Empty::Hover",
            (false, ButtonState::Pressed) => "CheckBox::Empty::Pressed",
        };

        let mut content_rect = BoundBox::new2(0, 0, 0, 0);
        frame.upload_primitives(Some(self.contents.to_prim("CheckBox", Some(&mut content_rect))));

        frame.upload_primitives(Some(
            match content_rect == BoundBox::new2(0, 0, 0, 0) {
                true => ThemedPrim {
                    min: Point2::new(
                        RelPoint::new(-1.0, 0),
                        RelPoint::new(-1.0, 0),
                    ),
                    max: Point2::new(
                        RelPoint::new( 1.0, 0),
                        RelPoint::new( 1.0, 0)
                    ),
                    prim: Prim::Image,
                    theme_path: image_str,
                    rect_px_out: Some(&mut self.check_rect)
                },
                false => ThemedPrim {
                    min: Point2::new(
                        RelPoint::new(-1.0, 0),
                        RelPoint::new(-1.0, content_rect.min().y),
                    ),
                    max: Point2::new(
                        RelPoint::new( 1.0, 0),
                        RelPoint::new(-1.0, content_rect.max().y),
                    ),
                    prim: Prim::Image,
                    theme_path: image_str,
                    rect_px_out: Some(&mut self.check_rect)
                }
            }
        ));
    }

    fn on_widget_event(&mut self, event: WidgetEventSourced, input_state: InputState) -> EventOps {
        use self::WidgetEvent::*;
        let event = event.unwrap();

        let (mut new_checked, mut new_state) = (self.checked, self.button_state);
        match event {
            MouseMove{hover_change: Some(ref change), ..} => match change {
                MouseHoverChange::Enter => new_state = ButtonState::Hover,
                MouseHoverChange::Exit => new_state = ButtonState::Normal,
                _ => ()
            },
            MouseDown{..} => new_state = ButtonState::Pressed,
            MouseUp{in_widget: true, pressed_in_widget: true, ..} => {
                if !self.checked {
                    self.handler.change_state(!self.checked);
                }
                new_checked = !self.checked;
                new_state = ButtonState::Hover;
            },
            MouseUp{in_widget: false, ..} => new_state = ButtonState::Normal,
            GainFocus(_) => new_state = ButtonState::Hover,
            LoseFocus => new_state = ButtonState::Normal,
            _ => ()
        };

        if new_checked != self.checked || new_state != self.button_state {
            self.widget_tag.request_redraw();
            self.checked = new_checked;
            self.button_state = new_state;
        }


        EventOps {
            focus: None,
            bubble: event.default_bubble(),
        }
    }
}
