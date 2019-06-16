// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::Px;
use num_traits::Bounded;

use cgmath_geometry::{D2, rect::{DimsBox, GeoBox}};
use std::ops::{Add, Range, RangeFrom, RangeFull, RangeTo};

pub type Tr = u32;
pub type Fr = f32;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TrRange {
    pub start: Option<Tr>,
    pub end: Option<Tr>
}

impl TrRange {
    /// Get the size of the TrRange, using `start_opt` if `self.start` is `None` and `end_opt` if
    /// `self.end` is `None`.
    pub fn size(self, start_opt: Tr, end_opt: Tr) -> Tr {
        self.end.unwrap_or(end_opt) - self.start.unwrap_or(start_opt)
    }
}

impl From<Tr> for TrRange {
    fn from(n: Tr) -> TrRange {
        TrRange::from(n..n + 1)
    }
}

impl From<Range<Tr>> for TrRange {
    fn from(r: Range<Tr>) -> TrRange {
        TrRange {
            start: Some(r.start),
            end: Some(r.end)
        }
    }
}

impl From<RangeFrom<Tr>> for TrRange {
    fn from(r: RangeFrom<Tr>) -> TrRange {
        TrRange {
            start: Some(r.start),
            end: None
        }
    }
}

impl From<RangeFull> for TrRange {
    fn from(_: RangeFull) -> TrRange {
        TrRange {
            start: None,
            end: None
        }
    }
}

impl From<RangeTo<Tr>> for TrRange {
    fn from(r: RangeTo<Tr>) -> TrRange {
        TrRange {
            start: None,
            end: Some(r.end)
        }
    }
}

two_axis_type!{
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GridSize(Tr);

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct WidgetSpan(Into<TrRange>);

    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Align2(Align);
}

impl Default for WidgetSpan {
    fn default() -> WidgetSpan {
        WidgetSpan::new(0..0, 0..0)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Align {
    Stretch,
    Start,
    End,
    Center
}

impl Default for Align {
    fn default() -> Align {
        Align::Stretch
    }
}



#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone, Copy)]
pub struct WidgetPos {
    pub size_bounds: SizeBounds,
    pub widget_span: WidgetSpan,
    pub place_in_cell: Align2,
    pub margins: Margins<Px>
}

impl WidgetPos {
    pub fn new(size_bounds: SizeBounds, widget_span: WidgetSpan, place_in_cell: Align2, margins: Margins<Px>) -> WidgetPos {
        WidgetPos {
            size_bounds: size_bounds,
            widget_span: widget_span,
            place_in_cell: place_in_cell,
            margins: margins
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackHints {
    /// Track-level minimum size. If the child minimum size is less than this, this is used instead.
    pub min_size: Px,
    /// Track-level maximum size. If this is less than the minimum size, minimum size takes priority
    /// and overrides this.
    pub max_size: Px,
    /// The proportion of free space this track takes up. This value represents a portion of the total
    /// "fractional space" available in the column or row - the layout engine attempts to set the pixel
    /// value to `total_free_space * fr_size / total_fr_size`.
    pub fr_size: Fr
}

impl Default for TrackHints {
    fn default() -> TrackHints {
        TrackHints {
            min_size: 0,
            max_size: Px::max_value(),
            fr_size: 1.0
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeBounds {
    pub min: DimsBox<D2, Px>,
    pub max: DimsBox<D2, Px>
}

impl SizeBounds {
    #[inline]
    pub fn new(min: DimsBox<D2, Px>, max: DimsBox<D2, Px>) -> SizeBounds {
        SizeBounds{ min, max }
    }

    #[inline]
    pub fn new_min(min: DimsBox<D2, Px>) -> SizeBounds {
        SizeBounds{ min, ..SizeBounds::default() }
    }

    #[inline]
    pub fn new_max(max: DimsBox<D2, Px>) -> SizeBounds {
        SizeBounds{ max, ..SizeBounds::default() }
    }

    /// Bound a rectangle to be within the size bounds.
    pub fn bound_rect(self, mut desired_size: DimsBox<D2, Px>) -> DimsBox<D2, Px> {
        if desired_size.width() < self.min.width() {
            desired_size.dims.x = self.min.width();
        } else if desired_size.width() > self.max.width() {
            desired_size.dims.x = self.max.width();
        }

        if desired_size.height() < self.min.height() {
            desired_size.dims.y = self.min.height();
        } else if desired_size.height() > self.max.height() {
            desired_size.dims.y = self.max.height();
        }

        desired_size
    }
}

impl Default for SizeBounds {
    fn default() -> SizeBounds {
        SizeBounds {
            min: DimsBox::new2(0, 0),
            max: DimsBox::max_value()
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Margins<T> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T
}

impl<T> Margins<T> {
    pub fn new(left: T, top: T, right: T, bottom: T) -> Margins<T> {
        Margins {
            left: left,
            top: top,
            right: right,
            bottom: bottom
        }
    }
}

impl<T> Margins<T>
    where T: Add<Output=T>
{
    #[inline(always)]
    pub fn width(self) -> T {
        self.left + self.right
    }

    #[inline(always)]
    pub fn height(self) -> T {
        self.top + self.bottom
    }
}
