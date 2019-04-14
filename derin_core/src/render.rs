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

pub trait DisplayEngine<'a>: 'static {
    type Renderer: 'a;
    type Layout: 'a;

    fn resized(&mut self, new_size: DimsBox<D2, u32>);
    fn dims(&self) -> DimsBox<D2, u32>;
    fn widget_removed(&mut self, widget_id: WidgetId);

    fn layout(&'a mut self, widget_id: WidgetId) -> Self::Layout;
    fn start_frame(&mut self);
    fn render(&'a mut self, widget_id: WidgetId, transform: BoundBox<D2, i32>, clip: BoundBox<D2, i32>) -> Self::Renderer;
    fn finish_frame(&mut self);
}

impl<'a> DisplayEngine<'a> for ! {
    type Renderer = !;
    type Layout = !;

    fn resized(&mut self, _: DimsBox<D2, u32>) {unreachable!()}
    fn dims(&self) -> DimsBox<D2, u32> {unreachable!()}
    fn widget_removed(&mut self, _: WidgetId) {unreachable!()}

    fn layout(&'a mut self, _: WidgetId) -> ! {*self}
    fn start_frame(&mut self) {unreachable!()}
    fn render(&'a mut self, _: WidgetId, _: BoundBox<D2, i32>, _: BoundBox<D2, i32>) -> ! {*self}
    fn finish_frame(&mut self) {unreachable!()}
}
