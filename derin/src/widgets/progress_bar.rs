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

use crate::event::{EventOps, WidgetEvent, InputState};
use crate::core::tree::{WidgetIdent, WidgetTag, Widget};
use crate::core::render::FrameRectStack;
use crate::core::popup::ChildPopupsMut;

use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::BoundBox};

use crate::gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim};

use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
pub struct ProgressBar {
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,

    value: f32,
    min: f32,
    max: f32,
}

impl ProgressBar {
    /// Creates a new progress bar with the given `value`, `step`, `min`, `max`, and action handler.
    pub fn new(value: f32, min: f32, max: f32) -> ProgressBar {
        ProgressBar {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            value,
            min,
            max
        }
    }

    /// Retrieves the value stored in the progress bar.
    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Retrieves the range of possible values the progress bar can contain.
    #[inline]
    pub fn range(&self) -> (f32, f32) {
        (self.min, self.max)
    }

    /// Retrieves the value stored in the progress bar, for mutation.
    ///
    /// Calling this function forces the progress bar to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn value_mut(&mut self) -> &mut f32 {
        self.widget_tag.mark_render_self();
        &mut self.value
    }

    /// Retrieves the range of possible values the progress bar can contain, for mutation.
    ///
    /// Calling this function forces the progress bar to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    #[inline]
    pub fn range_mut(&mut self) -> (&mut f32, &mut f32) {
        self.widget_tag.mark_render_self();
        (&mut self.min, &mut self.max)
    }
}

impl<A, F> Widget<A, F> for ProgressBar
    where F: PrimFrame
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

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        self.value = self.value.min(self.max).max(self.min);
        frame.upload_primitives(ArrayVec::from([
            ThemedPrim {
                theme_path: "ProgressBar::Background",
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
                theme_path: "ProgressBar::Fill",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new(-1.0 + (self.value / (self.max-self.min)) * 2.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image,
                rect_px_out: None
            }
        ]).into_iter());
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, _: &[WidgetIdent]) -> EventOps<A, F> {
        EventOps {
            action: None,
            focus: None,
            bubble: event.default_bubble(),
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }
}
