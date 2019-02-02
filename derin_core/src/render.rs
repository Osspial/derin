// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::cgmath::{Point2};
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox}};
use derin_common_types::cursor::CursorIcon;
use derin_common_types::layout::SizeBounds;

pub trait Renderer {
    type Frame: RenderFrame;
    fn resized(&mut self, new_size: DimsBox<D2, u32>);
    fn dims(&self) -> DimsBox<D2, u32>;
    fn render(
        &mut self,
        theme: &<Self::Frame as RenderFrame>::Theme,
        draw_to_frame: impl FnOnce(&mut Self::Frame)
    );
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

impl Theme for ! {
    type Key = !;
    type ThemeValue = !;
    fn widget_theme(&self, _: &!) -> ! {*self}
}

impl RenderFrame for ! {
    type Theme = !;
    type Primitive = !;

    fn upload_primitives<I>(
        &mut self,
        _: &Self::Theme,
        _: BoundBox<D2, i32>,
        _: BoundBox<D2, i32>,
        _: I
    )
        where I: Iterator<Item=Self::Primitive>
    {}
}
