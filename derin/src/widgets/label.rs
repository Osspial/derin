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

use widgets::{Contents, ContentsInner};
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, Widget};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox};
use derin_common_types::layout::SizeBounds;

use gl_render::PrimFrame;

/// A simple, non-interactive label.
///
/// Can display text or an image, depending on what's in `contents`.
#[derive(Debug, Clone)]
pub struct Label {
    update_tag: UpdateTag,
    bounds: BoundBox<Point2<i32>>,
    contents: ContentsInner,
    min_size: DimsBox<Point2<i32>>
}

impl Label {
    /// Create a new label with the given contents.
    pub fn new(contents: Contents<String>) -> Label {
        Label {
            update_tag: UpdateTag::new(),
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
        self.update_tag.mark_render_self();
        self.contents.borrow_mut()
    }
}

impl<A, F> Widget<A, F> for Label
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

    fn size_bounds(&self) -> SizeBounds {
        SizeBounds::new_min(self.min_size)
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        frame.upload_primitives(Some(self.contents.to_prim("Label", None)).into_iter());
        self.min_size = self.contents.min_size(frame.theme());
    }

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        EventOps {
            action: None,
            focus: None,
            bubble: true,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }
}
