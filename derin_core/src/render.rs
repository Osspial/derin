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
///
/// Really, `Render` and `Layout` should use associated type constructors for their lifetimes, not
/// a top-level lifetime, but ATCs aren't even implemented in the compiler so we can't do that. In
/// the meanwhile, use `for<'d> <D as DisplayEngine<'d>>` or `<D as DisplayEngine<'_>>` to refer
/// to `DisplayEngine` in your widget's where clauses.
pub trait DisplayEngine<'d>: 'static {
    type Renderer: 'd;
    type Layout: 'd;

    fn resized(&mut self, new_size: DimsBox<D2, u32>);
    fn dims(&self) -> DimsBox<D2, u32>;
    fn widget_removed(&mut self, widget_id: WidgetId);

    fn layout(&'d mut self, widget_id: WidgetId, dims: DimsBox<D2, i32>) -> Self::Layout;
    fn start_frame(&mut self);
    fn render(&'d mut self, widget_id: WidgetId, transform: BoundBox<D2, i32>, clip: BoundBox<D2, i32>) -> Self::Renderer;
    fn finish_frame(&mut self);
}

impl<'d> DisplayEngine<'d> for ! {
    type Renderer = !;
    type Layout = !;

    fn resized(&mut self, _: DimsBox<D2, u32>) {unreachable!()}
    fn dims(&self) -> DimsBox<D2, u32> {unreachable!()}
    fn widget_removed(&mut self, _: WidgetId) {unreachable!()}

    fn layout(&'d mut self, _: WidgetId, _: DimsBox<D2, i32>) -> ! {*self}
    fn start_frame(&mut self) {unreachable!()}
    fn render(&'d mut self, _: WidgetId, _: BoundBox<D2, i32>, _: BoundBox<D2, i32>) -> ! {*self}
    fn finish_frame(&mut self) {unreachable!()}
}
