// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![feature(try_blocks)]

pub mod rect_layout;
pub mod rect_to_triangles;
pub mod theme;

use cgmath_geometry::cgmath;

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
    /// Given a [`Content`] struct, lay out each of the [`Content`]'s string's [`GraphemeCluster`]s
    /// and append them to to the `grapheme_clusters` buffer.
    ///
    /// This is useful for text edit boxes, which must know the exact pixel location of each
    /// grapheme cluster on the screen in order to perform certain editing operations.
    ///
    /// If the [`Content`]'s string is `None`, this does nothing.
    fn layout_string<C: Content>(
        &mut self,
        content: &C,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphemeCluster {
    /// The byte range in the source string used to construct this grapheme cluster.
    pub range: Range<usize>,
    /// The rectangle used for selecting text. This need not, and indeed usually doesn't, exactly line
    /// up with the grapheme cluster's rendered rectangle.
    pub selection_rect: BoundBox<D2, i32>,
}

pub trait Content: Serialize {
    fn string(&self) -> Option<RenderString<'_>> {
        None
    }
}
impl Content for () {}


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RenderString<'s> {
    pub string: &'s str,
    pub decorations: EditStringDecorations,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditStringDecorations {
    pub cursor_pos: Option<usize>,
    pub highlight_range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutResult {
    pub size_bounds: SizeBounds,
    /// The rectangle child content widgets should be put in.
    pub content_rect: BoundBox<D2, i32>,
}

impl Default for EditStringDecorations {
    fn default() -> EditStringDecorations {
        EditStringDecorations {
            cursor_pos: None,
            highlight_range: 0..0,
        }
    }
}

#[cfg(test)]
mod tests {
    use cgmath_geometry::{
        D2,
        cgmath::{Point2, Vector2},
        rect::{BoundBox, GeoBox},
    };
    use std::collections::hash_map::{Entry, HashMap};

    /// Turns a string representation of rectangles into a `HashMap` of rectangles.
    ///
    /// The user provides a string with several alphanumeric characters. Each rectangle
    /// is created by looking at all alphanumeric characters, finding matching top-left
    /// and lower-right corner characters, then setting the `min` and `max` coordinates
    /// based the locations of those characters.
    ///
    /// All non-alphanumeric characters are ignored, making them useful as guides.
    ///
    /// Any unpaired characters will be represented as 0x0 rectangles. Note that the
    /// following rectangle is 1x1, not 2x2:
    ///
    /// ```text
    /// 0+
    /// +0
    /// ```
    pub fn rects_from_string(s: &str, first_alphanumeric_is_origin: bool) -> HashMap<char, BoundBox<D2, i32>> {
        let mut rects = HashMap::new();
        let mut offset_set = !first_alphanumeric_is_origin;
        let mut offset = Vector2::new(0, 0);

        for (y, line) in s.lines().enumerate() {
            let y = y as i32;

            for (x, c) in line.chars().enumerate().filter(|&(_, c)| c.is_alphanumeric()) {
                let x = x as i32;

                if !offset_set {
                    offset = Vector2::new(-x, -y);
                    offset_set = true;
                }

                match rects.entry(c) {
                    Entry::Vacant(v) => {v.insert(BoundBox::new2(x, y, x, y) + offset);},
                    Entry::Occupied(ref mut o) if o.get().min == o.get().max => {
                        let o = o.get_mut();
                        o.max = Point2::new(x, y) + offset;
                        let min = Point2::new(i32::min(o.min.x, o.max.x), i32::min(o.min.y, o.max.y));
                        let max = Point2::new(i32::max(o.min.x, o.max.x), i32::max(o.min.y, o.max.y));
                        o.min = min;
                        o.max = max;
                    },
                    Entry::Occupied(_) => panic!("Attempted to set lower-right corner of rect {} twice", c),
                }
            }
        }

        rects
    }

    #[test]
    fn test_rects_from_string() {
        // 0x0 square
        let s = "
        0
        ";
        let mut rects = HashMap::new();
        rects.insert('0', BoundBox::new2(8, 1, 8, 1));
        assert_eq!(rects, rects_from_string(s, false));
        assert_eq!((0, 0), (rects[&'0'].width(), rects[&'0'].height()));

        // 1x1 square
        let s = "
        0+
        +0
        ";
        let mut rects = HashMap::new();
        rects.insert('0', BoundBox::new2(8, 1, 9, 2));
        assert_eq!(rects, rects_from_string(s, false));
        assert_eq!((1, 1), (rects[&'0'].width(), rects[&'0'].height()));

        // 1x1 square
        let s = "
        0+
        +0
        ";
        let mut rects = HashMap::new();
        rects.insert('0', BoundBox::new2(0, 0, 1, 1));
        assert_eq!(rects, rects_from_string(s, true));
        assert_eq!((1, 1), (rects[&'0'].width(), rects[&'0'].height()));

        // 1x1 square
        let s = "
        +0
        0+
        ";
        let mut rects = HashMap::new();
        rects.insert('0', BoundBox::new2(8, 1, 9, 2));
        assert_eq!(rects, rects_from_string(s, false));
        assert_eq!((1, 1), (rects[&'0'].width(), rects[&'0'].height()));

        let s = "
            o
                      2---+
             +++    0 + 0 1
             ----     |   |   1
              ||||    +---2
        ";
        let mut rects = HashMap::new();
        rects.insert('o', BoundBox::new2(0, 0, 0, 0));
        rects.insert('0', BoundBox::new2(8, 2, 12, 2));
        rects.insert('1', BoundBox::new2(14, 2, 18, 3));
        rects.insert('2', BoundBox::new2(10, 1, 14, 4));
        assert_eq!(rects, rects_from_string(s, true));
    }
}
