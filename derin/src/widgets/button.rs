// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    event::{EventOps, WidgetEvent, WidgetEventSourced, InputState, MouseHoverChange},
    widget::{WidgetTag, WidgetRenderable, Widget},
    render::{DisplayEngine, RendererLayout, SubFrame},
};
use crate::widgets::{
    Content,
    assistants::ButtonState,
};

use cgmath_geometry::{D2, rect::BoundBox};
use derin_common_types::layout::SizeBounds;

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
    pub handler: H,
    contents: Content,
    size_bounds: SizeBounds
}

/// Determines which action, if any, should be taken in response to a button press.
pub trait ButtonHandler: 'static {
    fn on_click(&mut self);
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonTheme {
    pub state: ButtonState,
}

impl<H> Button<H> {
    /// Creates a new button with the given contents and
    pub fn new(contents: Content, handler: H) -> Button<H> {
        Button {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            state: ButtonState::Normal,
            handler,
            contents,
            size_bounds: SizeBounds::default()
        }
    }

    pub fn contents(&self) -> &Content {
        &self.contents
    }

    pub fn contents_mut(&mut self) -> &mut Content {
        self.widget_tag
            .request_redraw()
            .request_relayout();
        &mut self.contents
    }
}

impl<H> Widget for Button<H>
    where H: ButtonHandler
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

    fn on_widget_event(&mut self, event: WidgetEventSourced, _: InputState) -> EventOps {
        use self::WidgetEvent::*;
        let event = event.unwrap();

        let new_state = match event {
            MouseMove{hover_change: Some(ref change), ..} => match change {
                MouseHoverChange::Enter => ButtonState::Hover,
                MouseHoverChange::Exit => ButtonState::Normal,
                _ => self.state
            },
            MouseDown{..} => ButtonState::Pressed,
            MouseUp{in_widget: true, pressed_in_widget: true, ..} => {
                self.handler.on_click();
                ButtonState::Hover
            },
            MouseUp{in_widget: false, ..} => ButtonState::Normal,
            GainFocus(_, _) => ButtonState::Hover,
            LoseFocus => ButtonState::Normal,
            _ => self.state
        };

        if new_state != self.state {
            self.widget_tag.request_redraw();
            self.state = new_state;
        }


        EventOps {
            focus: None,
            bubble: event.default_bubble(),
        }
    }
}

impl<R, H> WidgetRenderable<R> for Button<H>
    where R: Renderer,
          H: ButtonHandler
{
    type Theme = ButtonTheme;

    fn theme(&self) -> ButtonTheme {
        ButtonTheme {
            state: self.state,
        }
    }

    fn render(&mut self, frame: &mut R::SubFrame) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, layout: &mut R::Layout) {
        match self.contents {
            Content::Text(ref s) => layout.prepare_string(s),
            Content::Icon(ref i) => layout.prepare_icon(i),
        }

        let result = layout.finish();
        self.size_bounds = result.size_bounds;
    }
}

impl WidgetTheme for ButtonTheme {
    type Fallback = !;
    fn fallback(self) -> Option<!> {
        None
    }
}
