// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use derin_common_types::{
    id,
    layout::{Align2, Margins, SizeBounds}
};


id!(pub ImageId);
id!(pub FontFaceId);

// TODO: Unify with Gullery color. Perhaps split out Gullery's image_format
// module into a separate crate?
/// SRGB 32-bit RGBA unsigned color format.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// The algorithm used to determine where line breaks occur in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineWrap {
    /// Disallow all line breaks, including explicit ones (such as from `'\n'`).
    None,
    /// Only allow explicit line breaks.
    Explicit,
    /// Allow line breaks at break points, as defined by [UAX #14](https://unicode.org/reports/tr14/).
    Normal,
}

/// Collection of information used to determine how to render text in a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextStyle {
    pub margins: Margins<i32>,
    pub render: TextRenderStyle,
    pub layout: TextLayoutStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRenderStyle {
    /// The color to draw text.
    pub color: Color,
    /// The color of the highlight when highlighting text.
    pub highlight_bg_color: Color,
    /// The color of highlighted text.
    pub highlight_text_color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextLayoutStyle {
    pub face: FontFaceId,
    /// The size of the text being drawn, in 64ths of a [point].
    ///
    /// [point]: https://en.wikipedia.org/wiki/Point_(typography)
    pub face_size: u32,
    /// The number of spaces contained within a tab stop.
    pub tab_size: u32,
    /// The horizontal and vertical justification of the text.
    pub justify: Align2,
    /// The line wrapping algorithm.
    pub line_wrap: LineWrap,
}

/// The text style and image used to draw a widget with a given style.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WidgetStyle {
    pub background: Option<ImageId>,
    pub text: TextStyle,
    pub content_margins: Margins<i32>,
    pub size_bounds: SizeBounds,
}
