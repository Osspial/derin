// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::{
    core::{
        event::{EventOps, WidgetEventSourced, InputState},
        widget::{WidgetRender, WidgetTag, Widget},
        render::RenderFrameClipped,
    },
    gl_render::PrimFrame,
    widgets::{Contents, ContentsInner},
};

use cgmath_geometry::{D2, rect::{BoundBox, DimsBox}};
use derin_common_types::layout::SizeBounds;


/// A simple, non-interactive label.
///
/// Can display text or an image, depending on what's in `contents`.
#[derive(Debug, Clone)]
pub struct Label {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    contents: ContentsInner,
    min_size: DimsBox<D2, i32>
}

impl Label {
    /// Create a new label with the given contents.
    pub fn new(contents: Contents<String>) -> Label {
        Label {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            contents: contents.to_inner(),
            min_size: DimsBox::new2(0, 0)
        }
    }

    /// Retrieves the contents of the label.
    pub fn contents(&self) -> Contents<&str> {
        self.contents.borrow()
    }

    /// Retrieves the contents of the label, for mutation.
    ///
    /// Calling this function forces the label to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn contents_mut(&mut self) -> Contents<&mut String> {
        self.widget_tag.request_redraw();
        self.contents.borrow_mut()
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
        SizeBounds::new_min(self.min_size)
    }

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl<F> WidgetRender<F> for Label
    where F: PrimFrame
{
    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        frame.upload_primitives(Some(self.contents.to_prim("Label", None)).into_iter());
        self.min_size = self.contents.min_size(frame.theme());
    }
}
