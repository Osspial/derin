// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::widget::WidgetId;
use cgmath_geometry::{
    D2,
    line::Segment,
    rect::{BoundBox, DimsBox},
};
use derin_common_types::layout::SizeBounds;
use std::ops::Range;

/// Lays out and renders Derin widgets.
pub trait DisplayEngine: 'static + for<'d> DisplayEngineLayoutRender<'d> {
    fn resized(&mut self, new_size: DimsBox<D2, u32>);
    fn dims(&self) -> DimsBox<D2, u32>;
    fn widget_removed(&mut self, widget_id: WidgetId);

    fn start_frame(&mut self);
    fn finish_frame(&mut self);
}

pub trait DisplayEngineLayoutRender<'d> {
    type Layout: 'd;
    type Renderer: 'd;

    fn layout(&'d mut self, widget_id: WidgetId, dims: DimsBox<D2, i32>) -> Self::Layout;
    fn render(&'d mut self, widget_id: WidgetId, transform: BoundBox<D2, i32>, clip: BoundBox<D2, i32>) -> Self::Renderer;
}

impl<'d> DisplayEngineLayoutRender<'d> for ! {
    type Renderer = !;
    type Layout = !;

    fn layout(&'d mut self, _: WidgetId, _: DimsBox<D2, i32>) -> ! {*self}
    fn render(&'d mut self, _: WidgetId, _: BoundBox<D2, i32>, _: BoundBox<D2, i32>) -> ! {*self}
}

impl DisplayEngine for ! {
    fn resized(&mut self, _: DimsBox<D2, u32>) {unreachable!()}
    fn dims(&self) -> DimsBox<D2, u32> {unreachable!()}
    fn widget_removed(&mut self, _: WidgetId) {unreachable!()}

    fn start_frame(&mut self) {unreachable!()}
    fn finish_frame(&mut self) {unreachable!()}
}
