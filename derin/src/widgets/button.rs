// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    LoopFlow,
    event::{EventOps, WidgetEvent, WidgetEventSourced, InputState, MouseHoverChange},
    widget::{MessageTarget, Parent, WidgetTag, WidgetRenderable, WidgetIdent, WidgetInfo, WidgetInfoMut, Widget},
    render::DisplayEngine,
};
use derin_display_engines::{Content, LayoutContent, RenderContent};
use crate::widgets::assistants::ButtonState;
use serde::Serialize;

use cgmath_geometry::{D2, rect::BoundBox};
use derin_common_types::layout::SizeBounds;

/// A simple push-button.
///
/// When pressed, calls the [`on_click`] function in the associated handler passed in by the `new`
/// function.
///
/// [`on_click`]: ./trait.ButtonHandler.html#tymethod.on_click
#[derive(Debug, Clone)]
pub struct Button<L: Widget> {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    state: ButtonState,
    label: L,
    size_bounds: SizeBounds,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ButtonContent {
    pub state: ButtonState,
}

#[derive(Debug, Clone)]
pub struct ButtonClickMessage {_marker: ()}

impl Content for ButtonContent {}

impl<L: Widget> Button<L> {
    /// Creates a new button with the given contents and
    pub fn new(label: L) -> Button<L> {
        Button {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            state: ButtonState::Normal,
            label,
            size_bounds: SizeBounds::default(),
        }
    }

    // TODO: LABEL MODIFICAITON FUNCTIONS
}

impl<L> Parent for Button<L>
    where L: Widget
{
    fn num_children(&self) -> usize {
        1
    }

    fn framed_child<D>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, D>>
        where for<'d> D: DisplayEngine<'d>
    {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.label)),
            _ => None
        }
    }
    fn framed_child_mut<D>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, D>>
        where for<'d> D: DisplayEngine<'d>
    {
        match widget_ident {
            WidgetIdent::Num(0) => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.label)),
            _ => None
        }
    }

    fn framed_children<'a, D, G>(&'a self, mut for_each: G)
        where for<'d> D: DisplayEngine<'d>,
              G: FnMut(WidgetInfo<'a, D>) -> LoopFlow
    {
        let _ = for_each(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.label));
    }

    fn framed_children_mut<'a, D, G>(&'a mut self, mut for_each: G)
        where for<'d> D: DisplayEngine<'d>,
              G: FnMut(WidgetInfoMut<'a, D>) -> LoopFlow
    {
        let _ = for_each(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.label));
    }

    fn framed_child_by_index<D>(&self, index: usize) -> Option<WidgetInfo<'_, D>>
        where for<'d> D: DisplayEngine<'d>,
    {
        match index {
            0 => Some(WidgetInfo::new(WidgetIdent::Num(0), 0, &self.label)),
            _ => None
        }
    }
    fn framed_child_by_index_mut<D>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, D>>
        where for<'d> D: DisplayEngine<'d>,
    {
        match index {
            0 => Some(WidgetInfoMut::new(WidgetIdent::Num(0), 0, &mut self.label)),
            _ => None
        }
    }
}

impl<L> Widget for Button<L>
    where L: Widget
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
                self.widget_tag.send_message_to(
                    ButtonClickMessage{_marker: ()},
                    MessageTarget::Widget(self.widget_id()),
                );
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

impl<D, L> WidgetRenderable<D> for Button<L>
    where for<'d> D: DisplayEngine<'d>,
          for<'d> <D as DisplayEngine<'d>>::Renderer: RenderContent<'d>,
          for<'d> <D as DisplayEngine<'d>>::Layout: LayoutContent<'d>,
          L: Widget,
{
    fn render(&mut self, frame: <D as DisplayEngine<'_>>::Renderer) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, layout: <D as DisplayEngine<'_>>::Layout) {
        let content = ButtonContent {
            state: self.state,
        };

        let result = layout.layout_content(&content);
        self.size_bounds = result.size_bounds;
        self.size_bounds.min.dims += self.label.size_bounds().min.dims;
        *self.label.rect_mut() = result.content_rect;
    }
}
