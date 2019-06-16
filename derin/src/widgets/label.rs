// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_core::{
    event::{EventOps, WidgetEventSourced, InputState},
    widget::{WidgetTag, WidgetRenderable, Widget},
    render::DisplayEngine,
};
use derin_display_engines::{Content, LayoutContent, RenderContent};
use serde::Serialize;

use cgmath_geometry::{D2, rect::BoundBox};
use derin_common_types::layout::SizeBounds;


/// A simple, non-interactive label.
#[derive(Debug, Clone)]
pub struct Label {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    string: String,
    size_bounds: SizeBounds,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LabelContent<'a> {
    pub string: &'a str,
}
impl<'a> Content for LabelContent<'a> {}

impl Label {
    /// Create a new label with the given string.
    pub fn new(string: String) -> Label {
        Label {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            string,
            size_bounds: SizeBounds::default(),
        }
    }

    /// Retrieves the string in the label.
    pub fn string(&self) -> &str {
        &self.string
    }

    /// Retrieves the string in the label, for mutation.
    ///
    /// Calling this function forces the label to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn string_mut(&mut self) -> &mut String {
        self.widget_tag
            .request_redraw()
            .request_relayout();

        &mut self.string
    }
}

impl Widget for Label {
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

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl<D> WidgetRenderable<D> for Label
    where D: DisplayEngine,
          for<'d> <D as DisplayEngine<'d>>::Renderer: RenderContent<'d>,
          for<'d> <D as DisplayEngine<'d>>::Layout: LayoutContent<'d>,
{
    fn render(&mut self, frame: <D as DisplayEngine<'_>>::Renderer) {
        frame.render_laid_out_content();
    }

    fn update_layout(&mut self, layout: <D as DisplayEngine<'_>>::Layout) {
        let content = LabelContent {
            string: &self.string,
        };

        let result = layout.layout_content(&content);
        self.size_bounds = result.size_bounds;
    }
}
