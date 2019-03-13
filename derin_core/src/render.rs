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

pub trait Renderer: 'static {
    type SubFrame: SubFrame;
    type Theme;
    type Layout: RendererLayout;

    fn resized(&mut self, new_size: DimsBox<D2, u32>);
    fn dims(&self) -> DimsBox<D2, u32>;
    fn widget_removed(&mut self, widget_id: WidgetId);
    fn layout(
        &mut self,
        widget_id: WidgetId,
        layout: impl FnOnce(&mut Self::Layout)
    );
    fn start_frame(&mut self, theme: &Self::Theme);
    fn finish_frame(&mut self, theme: &Self::Theme);
}

pub trait WidgetRenderer<T: WidgetTheme>: Renderer {
    fn render_widget(
        &mut self,
        widget_id: WidgetId,
        theme: &Self::Theme,
        transform: BoundBox<D2, i32>,
        clip: BoundBox<D2, i32>,
        widget_theme: T,
        render_widget: impl FnOnce(&mut Self::SubFrame),
    );
}

pub trait SubFrame {
    fn render_laid_out_content(&mut self);
}

#[derive(Debug, Clone)]
pub struct CursorData {
    pub draw_cursor: bool,
    pub cursor_pos: usize,
    pub highlight_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorOp {
    MoveVertical {
        delta: isize,
        expand_selection: bool,
    },
    MoveHorizontal {
        delta: isize,
        expand_selection: bool,
        jump_to_word_boundaries: bool,
    },
    SelectOnSegment(Segment<D2, i32>),
    SelectAll,
    UnselectAll,
    InsertChar(char),
    InsertString(String),
    DeleteChars {
        dist: isize,
        jump_to_word_boundaries: bool,
    },
    DeleteSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutResult {
    pub size_bounds: SizeBounds,
    /// The rectangle child content widgets should be put in.
    pub content_rect: BoundBox<D2, i32>,
}

pub trait RendererLayout {
    fn prepare_string(&mut self, string: &str);
    /// Layout the render string and perform any queued cursor operations.
    fn prepare_edit_string(
        &mut self,
        string: &mut String,
        cursor_data: &mut CursorData,
        cursor_ops: impl Iterator<Item=CursorOp>,
    );
    fn prepare_icon(&mut self, icon_name: &str);
    /// Finish laying stuff out and retrieve widget-level layout parameters. Calling this more than
    /// once should panic.
    fn finish(&mut self) -> LayoutResult;
}

pub trait WidgetTheme {
    type Fallback: WidgetTheme;

    fn fallback(self) -> Option<Self::Fallback>;
}

impl WidgetTheme for ! {
    type Fallback = !;

    fn fallback(self) -> Option<!> {
        self
    }
}

impl Renderer for ! {
    type SubFrame = !;
    type Theme = !;
    type Layout = !;

    fn resized(&mut self, _: DimsBox<D2, u32>) {unreachable!()}
    fn dims(&self) -> DimsBox<D2, u32> {unreachable!()}
    fn layout(
        &mut self,
        _: WidgetId,
        _: impl FnOnce(&mut Self::Layout)
    ) {unreachable!()}
    fn widget_removed(&mut self, _: WidgetId) {unreachable!()}
    fn start_frame(&mut self, _: &Self::Theme) {unreachable!()}
    fn finish_frame(&mut self, _: &Self::Theme) {unreachable!()}
}

impl RendererLayout for ! {
    fn prepare_string(&mut self, _string: &str) {}
    fn prepare_edit_string(
        &mut self,
        _: &mut String,
        _: &mut CursorData,
        _: impl Iterator<Item=CursorOp>,
    ) {}
    fn prepare_icon(&mut self, _: &str) {}
    fn finish(&mut self) -> LayoutResult {*self}
}

impl SubFrame for ! {
    fn render_laid_out_content(&mut self) {unreachable!()}
}

impl Default for CursorData {
    fn default() -> CursorData {
        CursorData {
            draw_cursor: false,
            cursor_pos: 0,
            highlight_range: 0..0,
        }
    }
}
