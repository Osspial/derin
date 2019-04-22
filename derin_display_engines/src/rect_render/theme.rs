use cgmath_geometry::{D2, rect::DimsBox};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageMeta {
    pub id: ImageId,
    pub dims: DimsBox<D2, u32>,
    pub rescale: RescaleRules,
    pub size_bounds: SizeBounds,
}

/// The algorithm used to rescale an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RescaleRules {
    /// Rescale the image by uniformly stretching it out, from its edges.
    Stretch,
    /// Perform nine-slicing on the provided image, stretching out the center of the image while
    /// keeping the borders of the image a constant size.
    Slice(Margins<u16>),
    Align(Align2),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeText {
    pub face: FontFaceId,
    /// The color to draw text.
    pub color: Color,
    /// The color of the highlight when highlighting text.
    pub highlight_bg_color: Color,
    /// The color of highlighted text.
    pub highlight_text_color: Color,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeWidget {
    pub text: ThemeText,
    pub image: Option<ImageMeta>,
    pub content_margins: Margins<u16>,
}
