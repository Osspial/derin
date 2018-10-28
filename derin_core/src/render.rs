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

use crate::cgmath::{EuclideanSpace, Point2};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};
use derin_common_types::cursor::CursorIcon;
use derin_common_types::layout::SizeBounds;

pub trait Renderer {
    type Frame: RenderFrame;
    #[inline]
    fn force_full_redraw(&self) -> bool {false}
    fn set_cursor_pos(&mut self, pos: Point2<i32>);
    fn set_cursor_icon(&mut self, icon: CursorIcon);
    fn set_size_bounds(&mut self, size_bounds: SizeBounds);
    fn resized(&mut self, new_size: DimsBox<D2, u32>);
    fn dims(&self) -> DimsBox<D2, u32>;
    fn make_frame(&mut self, draw_output: bool) -> (&mut Self::Frame, BoundBox<D2, i32>);
    fn finish_frame(&mut self, theme: &<Self::Frame as RenderFrame>::Theme);
}

pub trait RenderFrame: 'static {
    type Theme: Theme;
    type Primitive;

    fn upload_primitives<I>(
        &mut self,
        theme: &Self::Theme,
        transform: BoundBox<D2, i32>,
        clip: BoundBox<D2, i32>,
        prim_iter: I
    )
        where I: Iterator<Item=Self::Primitive>;
}

pub trait Theme {
    type Key: ?Sized;
    type ThemeValue;
    fn widget_theme(&self, key: &Self::Key) -> Self::ThemeValue;
}

pub struct FrameRectStack<'a, F: 'a + RenderFrame> {
    frame: &'a mut F,
    transform: BoundBox<D2, i32>,
    clip_rect: BoundBox<D2, i32>,

    theme: &'a F::Theme,

    pop_widget_ident: bool,
}

impl<'a, F: RenderFrame> FrameRectStack<'a, F> {
    #[inline]
    pub(crate) fn new(
        frame: &'a mut F,
        base_transform: BoundBox<D2, i32>,
        theme: &'a F::Theme,
    ) -> FrameRectStack<'a, F>
    {
        FrameRectStack {
            frame,
            transform: base_transform,
            clip_rect: base_transform,

            theme,

            pop_widget_ident: false,
        }
    }

    #[inline(always)]
    pub fn theme(&self) -> &F::Theme {
        self.theme
    }

    #[inline]
    pub fn upload_primitives<I>(&mut self, prim_iter: I)
        where I: IntoIterator<Item=F::Primitive>
    {
        self.frame.upload_primitives(self.theme, self.transform, self.clip_rect, prim_iter.into_iter())
    }

    #[inline]
    pub fn enter_child_rect<'b>(&'b mut self, child_rect: BoundBox<D2, i32>) -> Option<FrameRectStack<'b, F>> {
        let child_transform = child_rect + self.transform.min().to_vec();
        Some(FrameRectStack {
            frame: self.frame,
            transform: child_transform,
            clip_rect: self.clip_rect.intersect_rect(child_transform)?,

            theme: self.theme,
            pop_widget_ident: false,
        })
    }
}
