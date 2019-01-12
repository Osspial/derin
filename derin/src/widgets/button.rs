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
use crate::widgets::{Contents, ContentsInner};
use crate::core::event::{EventOps, WidgetEvent, WidgetEventSourced, InputState, MouseHoverChange};
use crate::core::tree::{WidgetIdent, WidgetTag, Widget};
use crate::core::render::{RenderFrameClipped, Theme};

use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};
use derin_common_types::layout::SizeBounds;

use crate::gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

use arrayvec::ArrayVec;

/// Determines which action, if any, should be taken in response to a button press.
pub trait ButtonHandler<A: 'static>: 'static {
    /// Called when the button is pressed. If `Some` is returned, the given action is pumped into
    /// the action queue and passed to [`run_forever`'s `on_action`][on_action].
    ///
    /// [on_action]: ../struct.Window.html#method.run_forever
    fn on_click(&mut self) -> Option<A>;
}

impl<A: 'static + Clone> ButtonHandler<A> for Option<A> {
    #[inline]
    fn on_click(&mut self) -> Option<A> {
        self.clone()
    }
}

impl<A: 'static> ButtonHandler<A> for () {
    #[inline]
    fn on_click(&mut self) -> Option<A> {
        None
    }
}

/// A simple push-button.
///
/// When pressed, calls the [`on_click`] function in the associated handler passed in by the `new`
/// function.
///
/// [`on_click`]: ./trait.ButtonHandler.html#tymethod.on_click
#[derive(Debug, Clone)]
pub struct Button<H> {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    state: ButtonState,
    handler: H,
    contents: ContentsInner,
    size_bounds: SizeBounds
}

impl<H> Button<H> {
    /// Creates a new button with the given contents and
    pub fn new(contents: Contents<String>, handler: H) -> Button<H> {
        Button {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            state: ButtonState::Normal,
            handler,
            contents: contents.to_inner(),
            size_bounds: SizeBounds::default()
        }
    }

    /// Retrieves the contents of the button.
    pub fn contents(&self) -> Contents<&str> {
        self.contents.borrow()
    }

    /// Retrieves the contents of the button, for mutation.
    ///
    /// Calling this function forces the button to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> Contents<&mut String> {
        self.widget_tag.request_redraw();
        self.contents.borrow_mut()
    }
}

impl<A, F, H> Widget<A, F> for Button<H>
    where A: 'static,
          F: PrimFrame,
          H: ButtonHandler<A>
{
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

    fn size_bounds(&self) -> SizeBounds {
        self.size_bounds
    }

    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        let image_str = match self.state {
            ButtonState::Normal    => "Button::Normal",
            ButtonState::Hover     => "Button::Hover",
            ButtonState::Pressed   => "Button::Pressed",
            // ButtonState::Disabled  => "Button::Disabled",
            // ButtonState::Defaulted => "Button::Defaulted"
        };

        frame.upload_primitives(ArrayVec::from([
            ThemedPrim {
                theme_path: image_str,
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
            self.contents.to_prim(image_str, None)
        ]).into_iter());

        self.size_bounds.min = frame.theme().widget_theme(image_str).image.map(|i| i.min_size()).unwrap_or(DimsBox::new2(0, 0));
        let render_string_min = self.contents.min_size(frame.theme());
        self.size_bounds.min.dims.x += render_string_min.width();
        self.size_bounds.min.dims.y += render_string_min.height();
    }

    fn on_widget_event(&mut self, event: WidgetEventSourced, input_state: InputState) -> EventOps<A> {
        use self::WidgetEvent::*;
        let event = event.unwrap();

        let mut action = None;

        let new_state = match event {
            MouseMove{hover_change: Some(ref change), ..} => match change {
                MouseHoverChange::Enter => ButtonState::Hover,
                MouseHoverChange::Exit => ButtonState::Normal,
                _ => self.state
            },
            MouseDown{..} => ButtonState::Pressed,
            MouseUp{in_widget: true, pressed_in_widget: true, ..} => {
                action = self.handler.on_click();
                ButtonState::Hover
            },
            MouseUp{in_widget: false, ..} => ButtonState::Normal,
            GainFocus(_) => ButtonState::Hover,
            LoseFocus => ButtonState::Normal,
            _ => self.state
        };

        if new_state != self.state {
            self.widget_tag.request_redraw();
            self.state = new_state;
        }


        EventOps {
            action,
            focus: None,
            bubble: event.default_bubble(),
        }
    }
}
