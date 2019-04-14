use cgmath_geometry::{
    D2,
    rect::BoundBox,
};
use derin_common_types::layout::SizeBounds;
use serde::{Deserialize, Serialize};
use std::ops::Range;

pub trait RenderContent<'a> {
    fn render_laid_out_content(self);
}

/// Lay out content to render with `RenderContent`.
pub trait LayoutContent<'a> {
    fn layout_content<C: Content>(self, content: &C) -> LayoutResult;
}

pub trait LayoutString<'a>: LayoutContent<'a> {
    /// Given a [`Content`] struct and a reference to a string inside that [`Content`] struct, lay
    /// out each of the string's [`GraphemeCluster`]s and append them to to the `grapheme_clusters`
    /// buffer.
    ///
    /// This is useful for text edit boxes, which must know the exact pixel location of each
    /// grapheme cluster on the screen in order to perform certain editing operations.
    ///
    /// # Panics
    /// This function will panic if the `string` parameter does not *exactly* reference a string
    /// inside the `content` struct.
    fn layout_string<C: Content>(
        &mut self,
        content: &C,
        string: &str,
        grapheme_clusters: &mut Vec<GraphemeCluster>
    );
}

/// A grapheme cluster in the the source string, and its location on the screen.
///
/// For detailed information as to what exactly a grapheme cluster *is*, see this document:
/// <https://unicode.org/reports/tr29/>
///
/// TL;DR: "Grapheme Cluster" is the technical name for any character on the screen that an average
/// user would consider a character. This may include multiple `char`s, as certain grapheme clusters
/// are made out of multiple individual `char`s, such as the 🤷🏽‍♀️ emoji: it's made out of separate
/// `'🤷'`, `'🏽'`, `'‍'` (zero width joiner), and `'♀️'` unicode characters, which get combined
/// together by the font into a single grapheme cluster that gets displayed to the user.
pub struct GraphemeCluster {
    /// The byte range in the source string used to construct this grapheme cluster.
    pub range: Range<usize>,
    /// The rectangle used for selecting text. This need not, and indeed usually doesn't, exactly line
    /// up with the grapheme cluster's rendered rectangle.
    pub selection_rect: BoundBox<D2, i32>,
}

pub trait Content: Serialize {}
impl Content for () {}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditString {
    pub string: String,
    pub draw_cursor: bool,
    pub cursor_pos: usize,
    pub highlight_range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutResult {
    pub size_bounds: SizeBounds,
    /// The rectangle child content widgets should be put in.
    pub content_rect: BoundBox<D2, i32>,
}

impl Default for EditString {
    fn default() -> EditString {
        EditString {
            string: String::new(),
            draw_cursor: false,
            cursor_pos: 0,
            highlight_range: 0..0,
        }
    }
}
