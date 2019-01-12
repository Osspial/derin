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

use crate::cgmath::{Point2};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox}};
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
    fn make_frame(&mut self) -> (&mut Self::Frame, BoundBox<D2, i32>);
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

pub struct RenderFrameClipped<'a, F: 'a + RenderFrame> {
    pub(crate) frame: &'a mut F,
    pub(crate) transform: BoundBox<D2, i32>,
    pub(crate) clip: BoundBox<D2, i32>,

    pub(crate) theme: &'a F::Theme,
}

impl<'a, F: RenderFrame> RenderFrameClipped<'a, F> {
    #[inline(always)]
    pub fn theme(&self) -> &F::Theme {
        self.theme
    }

    #[inline]
    pub fn upload_primitives<I>(&mut self, prim_iter: I)
        where I: IntoIterator<Item=F::Primitive>
    {
        self.frame.upload_primitives(self.theme, self.transform, self.clip, prim_iter.into_iter())
    }
}
