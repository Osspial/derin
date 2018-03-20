#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;
extern crate num_traits;
use cgmath_geometry::cgmath;
extern crate cgmath_geometry;
extern crate dct;

mod grid;

use dct::Px;
use cgmath::{Point2, Vector2, EuclideanSpace};
use cgmath_geometry::{DimsBox, BoundBox, GeoBox};
use dct::layout::{Fr, Tr, Align2, Align, GridSize, WidgetPos, TrackHints, SizeBounds, Margins};
use grid::{TrackVec, SizeResult};

use std::cmp;

#[derive(Default)]
pub struct UpdateHeapCache {
    frac_tracks: TrackVec<Tr>,
    potential_frac_tracks: TrackVec<Tr>,
    rigid_tracks_widget: Vec<Tr>,
    frac_tracks_widget: Vec<Tr>,
    solvable_widgets: Vec<Solvable>
}

#[derive(Debug, Clone)]
pub struct GridEngine {
    grid: TrackVec,
    /// The pixel size of the layout engine, as requested by the programmer.
    pub desired_size: DimsBox<Point2<Px>>,
    /// The pixel size of the layout engine, accounting for the size bounds of the widgets and the size
    /// bounds of the engine.
    actual_size: DimsBox<Point2<Px>>,
    /// The size bounds of the engine, as requested by the programmer.
    pub desired_size_bounds: SizeBounds,
    /// The size bounds of the engine, accounting for the size bounds of the widgets.
    actual_size_bounds: SizeBounds,
    /// The margins that appear around the outside of the widget grid
    pub grid_margins: Margins<Px>
}

impl UpdateHeapCache {
    #[inline]
    pub fn new() -> UpdateHeapCache {
        UpdateHeapCache::default()
    }
}

impl GridEngine {
    pub fn new() -> GridEngine {
        GridEngine {
            grid: TrackVec::new(),
            desired_size: DimsBox::new2(0, 0),
            actual_size: DimsBox::new2(0, 0),
            desired_size_bounds: SizeBounds::default(),
            actual_size_bounds: SizeBounds::default(),
            grid_margins: Margins::default()
        }
    }

    pub fn grid_size(&self) -> GridSize {
        self.grid.grid_size()
    }

    pub fn set_grid_size(&mut self, size: GridSize) {
        self.grid.set_grid_size(size)
    }

    pub fn row_hints(&self, row: Tr) -> TrackHints {
        self.grid.get_row(row).expect(&format!("Row {} out of range", row)).hints()
    }

    pub fn set_row_hints(&mut self, row: Tr, hints: TrackHints) {
        self.grid.get_row_mut(row).expect(&format!("Row {} out of range", row)).set_hints(hints).ok();
    }

    pub fn col_hints(&self, col: Tr) -> TrackHints {
        self.grid.get_col(col).expect(&format!("Col {} out of range", col)).hints()
    }

    pub fn set_col_hints(&mut self, col: Tr, hints: TrackHints) {
        self.grid.get_col_mut(col).expect(&format!("Col {} out of range", col)).set_hints(hints).ok();
    }

    pub fn actual_size(&self) -> DimsBox<Point2<Px>> {
        self.actual_size
    }

    pub fn actual_size_bounds(&self) -> SizeBounds {
        self.actual_size_bounds
    }

    /// This method is the core of the derin layout engine, and is easily the most complex
    /// method it has. This takes a layout engine, iterates over all of the updates performed on that
    /// engine, and performs constraint solving to ensure that all* of the constraints within the engine
    /// are solved.
    ///
    /// <sup>\* The only situation where some constraints may end up violated would be when the maximum
    /// size is less than the minimum size. In that case, minimum size overrides maximum size, as doing
    /// otherwise could cause rendering issues. </sup>
    pub fn update_engine(
        &mut self,
        hints: &[WidgetPos],
        rects: &mut [Result<BoundBox<Point2<Px>>, SolveError>],
        heap_cache: &mut UpdateHeapCache
    ) {
        assert_eq!(hints.len(), rects.len());

        // We start out by setting the free space to its maximum possible value.
        let mut free_width = sub_px_bound_zero(self.desired_size.width(), self.grid_margins.width());
        let mut fr_total_width = 0.0;
        let mut free_height = sub_px_bound_zero(self.desired_size.height(), self.grid_margins.height());
        let mut fr_total_height = 0.0;

        // Reset the actual size bounds to zero.
        self.actual_size_bounds = SizeBounds {
            min: DimsBox::new2(0, 0),
            max: DimsBox::new2(0, 0)
        };

        let mut frac_min_size = DimsBox::new2(0, 0);

        let mut rigid_min_size = DimsBox::new2(0, 0);

        // Next, we perform an iteration over the tracks, subtracting from the free space if the track is
        // rigid.
        macro_rules! first_track_pass {
            ($axis:ident, $push_track:ident, $track_range_mut:ident, $free_size:expr, $fr_total:expr) => {
                if 0 == self.grid.$track_range_mut(..).unwrap().len() {
                    self.actual_size_bounds.max.dims.$axis = Px::max_value();
                } else {
                    for (index, track) in self.grid.$track_range_mut(..).unwrap().iter_mut().enumerate() {
                        let track_fr_size = track.hints().fr_size;
                        if track_fr_size <= 0.0 {
                            track.reset_shrink();
                            rigid_min_size.dims.$axis += track.min_size();
                            // To make sure that the maximum size isn't below the minimum needed for this track,
                            // increase the engine maximum size by the rigid track minimum size.
                            self.actual_size_bounds.max.dims.$axis =
                                self.actual_size_bounds.max.dims.$axis.saturating_add(track.min_size());
                            $free_size = sub_px_bound_zero($free_size, track.size());
                        } else {
                            // The engine maximum size isn't expanded in a rigid track because the track won't
                            // expand when the rectangle of the engine is expanded.
                            self.actual_size_bounds.max.dims.$axis =
                                self.actual_size_bounds.max.dims.$axis.saturating_add(track.max_size());
                            track.reset_expand();
                            frac_min_size.dims.$axis += track.min_size();
                            $fr_total += track_fr_size;
                            heap_cache.frac_tracks.$push_track(index as Tr);
                        }
                    }
                }
            }
        }

        first_track_pass!(x, push_col, col_range_mut, free_width, fr_total_width);
        first_track_pass!(y, push_row, row_range_mut, free_height, fr_total_height);


        self.actual_size_bounds.max =
            self.desired_size_bounds.bound_rect(self.actual_size_bounds.max);

        self.actual_size_bounds.min = DimsBox::new2(
            frac_min_size.width() + rigid_min_size.width() + self.grid_margins.width(),
            frac_min_size.height() + rigid_min_size.height() + self.grid_margins.height()
        );
        self.actual_size_bounds.min =
            self.desired_size_bounds.bound_rect(self.actual_size_bounds.min);

        self.actual_size = self.actual_size_bounds.bound_rect(self.desired_size);

        'update: loop {
            /// Macro for solving the track constraints independent of axis. Because each axis is
            /// independent from the other but the computations required for both are basically the same,
            /// they're placed in a macro to allow DRY.
            macro_rules! track_constraints {
                ($get_track:ident, $get_track_mut:ident, $push_track:ident, $num_tracks_method:ident,
                 $remove_track:ident, $free_size:expr, $fr_total:expr) => {(|| {
                    //                                                      ^^
                    // Why is this a closure? Consecutive loops in the same function get a warning
                    // for label shadowing if they have the same label, and this supresses that.

                    let mut pft_index = 0;
                    while let Some(track_index) = heap_cache.potential_frac_tracks.$get_track(pft_index).cloned() {
                        let track = self.grid.$get_track(track_index).unwrap();
                        let track_fr_size = track.hints().fr_size;
                        let mut track_copy = track.clone();

                        // While this isn't an *exact* calculation of the new size of the track (due to remainders and whatnot
                        // as implemented in `FrDivider`), it's a good enough estimate.
                        let new_size = (($free_size + track.size()) as Fr * track_fr_size / ($fr_total + track_fr_size)) as Px;

                        match track_copy.change_size(new_size) {
                            // If the track can be freely rescaled, add it back to `frac_tracks` and remove it from
                            // `potential_frac_tracks`.
                            SizeResult::SizeUpscale    |
                            SizeResult::SizeDownscale => {
                                $free_size += track.size();
                                $fr_total += track_fr_size;
                                heap_cache.frac_tracks.$push_track(track_index);
                                heap_cache.potential_frac_tracks.$remove_track(pft_index);
                            }

                            // If the track has been downscaled but clamped, it still isn't a free track. However, it does free
                            // up some space that can be used by other tracks so add that to the total.
                            SizeResult::SizeDownscaleClamp => {
                                $free_size += track.size() - track_copy.size();
                                pft_index += 1;
                            },

                            // If the track has been *upscaled* but clamped, it still isn't a free track but it does take up some
                            // hitherto unoccupied free space.
                            SizeResult::SizeUpscaleClamp => {
                                $free_size -= track.size() - track_copy.size();
                                pft_index += 1;
                            },

                            // If there's no effect, keep the track on the list and increment `pft_index`.
                            SizeResult::NoEffectUp    |
                            SizeResult::NoEffectEq    |
                            SizeResult::NoEffectDown => pft_index += 1
                        }
                    }

                    'frac: loop {
                        let mut frac_index = 0;
                        let mut fr_divider = FrDivider::new(heap_cache.frac_tracks.$num_tracks_method(), $free_size, $fr_total);
                        while let Some(track_index) = heap_cache.frac_tracks.$get_track(frac_index).map(|t| *t as Tr) {
                            let track = self.grid.$get_track_mut(track_index).unwrap();
                            let track_fr_size = track.hints().fr_size;

                            let new_size = fr_divider.divvy(track_fr_size);

                            match track.change_size(new_size) {
                                // If the resize occured without issues, increment frac_index and go on to the next track.
                                SizeResult::SizeUpscale    |
                                SizeResult::SizeDownscale  |
                                SizeResult::NoEffectEq    => frac_index += 1,

                                // If changing the track size resulted in the track reaching its minimum size, that track can be
                                // considered rigid because it cannot shrink any further. Mark it for removal from the fractional
                                // tracks list, remove it from the fractional totals, then begin the fractional expansion again.
                                SizeResult::SizeDownscaleClamp |
                                SizeResult::NoEffectUp         |
                                SizeResult::SizeUpscaleClamp   |
                                SizeResult::NoEffectDown      => {
                                    $free_size -= track.size();
                                    $fr_total -= track_fr_size;
                                    heap_cache.frac_tracks.$remove_track(frac_index);
                                    heap_cache.potential_frac_tracks.$push_track(track_index as u32);
                                    continue 'frac;
                                }
                            }
                        }

                        break 'frac;
                    }
                })()}
            }

            track_constraints!(get_col, get_col_mut, push_col, num_cols, remove_col, free_width, fr_total_width);
            track_constraints!(get_row, get_row_mut, push_row, num_rows, remove_row, free_height, fr_total_height);

            let mut solvable_index = 0;
            for (hint, widget_rect) in hints.iter().zip(rects.iter_mut()) {
                if 0 < hint.widget_span.x.size(0, 1) &&
                   0 < hint.widget_span.y.size(0, 1)
                {
                    let &mut UpdateHeapCache {
                        ref mut rigid_tracks_widget,
                        ref mut frac_tracks_widget,
                        ref mut solvable_widgets,
                        ..
                    } = heap_cache;

                    let &mut GridEngine {
                        ref mut grid,
                        ref mut actual_size,
                        ref mut actual_size_bounds,
                        ..
                    } = self;

                    let solvable = {
                        if solvable_widgets.get(solvable_index).is_some() {
                            &mut solvable_widgets[solvable_index]
                        } else {
                            solvable_widgets.push(Solvable::default());
                            solvable_widgets.last_mut().unwrap()
                        }
                    };

                    // The widget size bounds, including the margin
                    let widget_size_bounds = {
                        let mut wsb = hint.size_bounds;
                        let margins_x = hint.margins.left + hint.margins.right;
                        let margins_y = hint.margins.top + hint.margins.bottom;

                        wsb.min.dims.x += margins_x;
                        wsb.max.dims.x = wsb.max.dims.x.saturating_add(margins_x);
                        wsb.min.dims.y += margins_y;
                        wsb.max.dims.y = wsb.max.dims.y.saturating_add(margins_y);
                        wsb
                    };

                    macro_rules! widget_scale {
                        ($axis:ident, $size:ident, $track_range:ident, $track_range_mut:ident, $free_size:expr, $fr_axis:expr) => {{
                            // The total fractional size of the tracks in the widget
                            let mut fr_widget = 0.0;
                            let mut fr_expand: Px = 0;
                            // The total pixel size of the tracks in the widget
                            let mut px_widget = 0;
                            let mut min_size_debt = widget_size_bounds.min.$size();

                            if let Some(track_slice) = grid.$track_range(hint.widget_span.$axis) {
                                for (index, track) in track_slice.iter().enumerate() {
                                    let track_fr_size = track.hints().fr_size;
                                    px_widget += track.size();
                                    min_size_debt = sub_px_bound_zero(min_size_debt, track.min_size());

                                    if track_fr_size == 0.0 {
                                        rigid_tracks_widget.push(index as Tr);
                                    } else {
                                        fr_widget += track_fr_size;
                                        fr_expand = fr_expand.saturating_add(track.max_size() - track.min_size());
                                        frac_tracks_widget.push(index as Tr);
                                    }
                                }
                            }

                            if solvable.$axis == SolveAxis::Solvable {
                                let mut grid_changed = false;

                                while 0 < rigid_tracks_widget.len() {
                                    let rigid_expand = min_size_debt / rigid_tracks_widget.len() as Px;
                                    let mut expand_rem = min_size_debt % rigid_tracks_widget.len() as Px;

                                    let mut rigid_index = 0;
                                    while let Some(track_index) = rigid_tracks_widget.get(rigid_index).cloned() {
                                        let track = &mut grid.$track_range_mut(hint.widget_span.$axis).unwrap()[track_index as usize];
                                        let expansion = rigid_expand + (expand_rem != 0) as Px;

                                        if track.min_size() + expansion <= track.max_size() {
                                            min_size_debt = sub_px_bound_zero(min_size_debt, expansion);
                                            let new_size = track.min_size() + expansion;

                                            if let Err(expanded) = track.expand_widget_min_size(new_size) {
                                                actual_size_bounds.max.dims.$axis =
                                                    actual_size_bounds.max.$size().saturating_add(expanded);
                                                actual_size.dims.$axis += expanded;

                                                $free_size = sub_px_bound_zero($free_size, expanded);
                                                rigid_min_size.dims.$axis += expanded;

                                                grid_changed = true;
                                            }
                                            rigid_index += 1;

                                        } else {
                                            rigid_tracks_widget.remove(rigid_index);
                                            min_size_debt = sub_px_bound_zero(min_size_debt, track.max_size() - track.min_size());

                                            let track_max_size = track.max_size();
                                            if let Err(expanded) = track.expand_widget_min_size(track_max_size) {
                                                actual_size_bounds.max.dims.$axis =
                                                    actual_size_bounds.max.$size().saturating_add(expanded);
                                                actual_size.dims.$axis += expanded;

                                                $free_size = sub_px_bound_zero($free_size, expanded);
                                                rigid_min_size.dims.$axis += track.max_size() - track.min_size();

                                                grid_changed = true;
                                            }

                                            // we don't continue because TODO PUT WHY
                                        }

                                        expand_rem = sub_px_bound_zero(expand_rem, 1);
                                    }

                                    if 0 == min_size_debt {break}
                                }

                                frac_min_size.dims.$axis = cmp::max(
                                    (widget_size_bounds.min.$size() as Fr * $fr_axis / fr_widget).ceil() as Px,
                                    frac_min_size.$size()
                                );

                                min_size_debt = sub_px_bound_zero(min_size_debt, fr_expand);

                                if 0 < min_size_debt {
                                    solvable.$axis = SolveAxis::Unsolvable;
                                }

                                actual_size_bounds.min.dims.$axis = frac_min_size.$size() + rigid_min_size.$size() + self.grid_margins.$size();
                                if actual_size.$size() < actual_size_bounds.min.$size() {
                                    grid_changed = true;
                                    actual_size.dims.$axis = actual_size_bounds.min.$size();
                                }

                                rigid_tracks_widget.clear();
                                frac_tracks_widget.clear();

                                if grid_changed {
                                    continue 'update;
                                }
                            }

                            px_widget
                        }}
                    }

                    // The widget_scale macro isn't guaranteed to return, but if it does it returns the axis size
                    // if it does. If it doesn't, the rest of this body is skipped and we go back to the beginning
                    // of the `update` loop.
                    let size_x = widget_scale!(x, width, col_range, col_range_mut, free_width, fr_total_width);
                    let size_y = widget_scale!(y, height, row_range, row_range_mut, free_height, fr_total_height);

                    // Perform cell hinting and set
                    let widget_origin_rect = DimsBox::new2(size_x, size_y);

                    if let Some(offset) = grid.get_cell_offset(
                        hint.widget_span.x.start.unwrap_or(0),
                        hint.widget_span.y.start.unwrap_or(0)
                    ) {
                        let outer_rect = BoundBox::from(widget_origin_rect) + offset.to_vec();
                        let cell_hinter = CellHinter::new(outer_rect, hint.place_in_cell);

                        solvable_index += 1;
                        let grid_margin_offset = Vector2::new(self.grid_margins.left, self.grid_margins.top);
                        *widget_rect = cell_hinter.hint(hint.size_bounds, hint.margins)
                            .map(|rect| BoundBox::from(rect) + grid_margin_offset)
                            .map_err(|_| SolveError::WidgetUnsolvable)
                    } else {
                        *widget_rect = Err(SolveError::CellOutOfBounds)
                    }
                } else {
                    *widget_rect = Err(SolveError::WidgetUnsolvable)
                }
            }

            break 'update;
        }

        heap_cache.frac_tracks.clear();
        heap_cache.potential_frac_tracks.clear();
        heap_cache.rigid_tracks_widget.clear();
        heap_cache.frac_tracks_widget.clear();
        heap_cache.solvable_widgets.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveError {
    /// The widget's constraints lead to it being unsolvable.
    WidgetUnsolvable,
    CellOutOfBounds
}

struct FrDivider {
    num_tracks: Tr,
    desired_size: Px,
    fr_total: Fr,
    /// An accumulation of the fractional parts of the `new_size_float` variable computed in the
    /// `divvy` function.
    remainder: f32
}

impl FrDivider {
    fn new(num_tracks: Tr, desired_size: Px, fr_total: Fr) -> FrDivider {
        FrDivider {
            num_tracks: num_tracks,
            desired_size: desired_size,
            fr_total: fr_total,
            remainder: 0.0
        }
    }

    /// Given a fractional track size, divvy up a part of the desired pixel size and return it.
    fn divvy(&mut self, track_fr_size: Fr) -> Px {
        // Compute the size of the track as a floating-point number. We can't just return this value, as
        // tracks are alligned to the pixel and floats mess with that.
        let new_size_float = self.desired_size as Fr * track_fr_size / self.fr_total;

        // Add the fractional part of `new_size_float` to the remainder accumulator, and if that accumulator
        // is greater than one add it to the `new_size` variable. Then, make sure the remainder accumulator
        // is less than one.
        self.remainder += new_size_float.fract();
        let new_size = new_size_float as Px + self.remainder as Px;
        self.remainder -= self.remainder.trunc();

        self.fr_total -= track_fr_size;
        self.desired_size -= new_size;
        self.num_tracks -= 1;

        if self.num_tracks == 0 && self.desired_size > 0 {
            new_size + self.desired_size
        } else {
            new_size
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Solvable {
    x: SolveAxis,
    y: SolveAxis
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SolveAxis {
    Solvable,
    Unsolvable
}

impl Default for SolveAxis {
    fn default() -> SolveAxis {
        SolveAxis::Solvable
    }
}

#[derive(Debug, Clone, Copy)]
struct CellHinter {
    outer_rect: BoundBox<Point2<Px>>,
    place_in_or: Align2
}

impl CellHinter {
    pub fn new(outer_rect: BoundBox<Point2<Px>>, place_in_or: Align2) -> CellHinter {
        CellHinter {
            outer_rect: outer_rect,
            place_in_or: place_in_or,
        }
    }

    pub fn hint(&self, bounds: SizeBounds, margins: Margins<Px>) -> Result<BoundBox<Point2<Px>>, HintError> {
        let margins_x = margins.left + margins.right;
        let margins_y = margins.top + margins.bottom;

        if bounds.min.width() + margins_x > self.outer_rect.width() ||
           bounds.min.height() + margins_y > self.outer_rect.height()
        {
            return Err(HintError::ORTooSmall)
        }

        let mut inner_rect = BoundBox::new2(0, 0, 0, 0);

        macro_rules! place_on_axis {
            ($axis:ident, $size:ident, $front_margin:expr, $back_margin:expr) => {
                match self.place_in_or.$axis {
                    Align::Stretch => {
                        inner_rect.min.$axis = self.outer_rect.min.$axis + $front_margin;
                        inner_rect.max.$axis = self.outer_rect.max.$axis - $back_margin;

                        if inner_rect.$size() > bounds.max.$size() {
                            let size_diff = inner_rect.$size() - bounds.max.$size();

                            inner_rect.min.$axis += size_diff / 2 + size_diff % 2;
                            inner_rect.max.$axis -= size_diff / 2;
                        }
                    },
                    Align::Start => {
                        inner_rect.min.$axis = self.outer_rect.min.$axis + $front_margin + $front_margin;
                        inner_rect.max.$axis = self.outer_rect.min.$axis + bounds.min.$size() + $front_margin;
                    },
                    Align::End => {
                        inner_rect.max.$axis = self.outer_rect.max.$axis - $back_margin;
                        inner_rect.min.$axis = self.outer_rect.max.$axis - bounds.min.$size() - $back_margin;
                    },
                    Align::Center => {
                        let center = (self.outer_rect.min.$axis + self.outer_rect.max.$axis) / 2;
                        inner_rect.min.$axis = center - bounds.min.$size() / 2;
                        inner_rect.max.$axis = center + bounds.min.$size() / 2;

                        if inner_rect.$size() > bounds.max.$size() {
                            let size_diff = inner_rect.$size() - bounds.max.$size();

                            inner_rect.min.$axis += size_diff / 2 + size_diff % 2;
                            inner_rect.max.$axis -= size_diff / 2;
                        }

                        let front_margin_shift = sub_px_bound_zero($front_margin, inner_rect.min.$axis - self.outer_rect.min.$axis);
                        let back_margin_shift = sub_px_bound_zero($back_margin, self.outer_rect.max.$axis - inner_rect.max.$axis);
                        inner_rect.min.$axis += front_margin_shift;
                        inner_rect.max.$axis += front_margin_shift;
                        inner_rect.min.$axis -= back_margin_shift;
                        inner_rect.max.$axis -= back_margin_shift;
                    }
                }
            }
        }

        place_on_axis!(x, width, margins.left, margins.right);
        place_on_axis!(y, height, margins.top, margins.bottom);

        Ok(inner_rect)
    }
}

#[inline]
fn sub_px_bound_zero(lhs: Px, rhs: Px) -> Px {
    let result = lhs.saturating_sub(rhs);
    if 0 <= result {
        result
    } else {
        0
    }
}

enum HintError {
    /// The outer rect is smaller than the minimum size bound, making constraint unsolvable
    ORTooSmall
}


#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use geometry::*;
    use std::mem;

    quickcheck!{
        fn test_px_divvy(desired_size: Px, frac_sizes: Vec<Fr>) -> bool {
            let mut frac_sizes = frac_sizes;

            // Make sure that none of the frac_sizes are negative, because `FrDivider` does not
            // support that.
            for track_fr_size in &mut frac_sizes {
                *track_fr_size = track_fr_size.abs();
            }

            if frac_sizes.len() == 0 {
                return true;
            }

            let num_fracts = frac_sizes.len() as Tr;
            let fr_total: Fr = frac_sizes.iter().cloned().sum();

            let mut expander = FrDivider::new(num_fracts, desired_size, fr_total);

            let mut actual_size = 0;
            for track_fr_size in frac_sizes {
                actual_size += expander.divvy(track_fr_size);
            }

            actual_size == desired_size
        }
    }

    impl Arbitrary for BoundRect {
        fn arbitrary<G: Gen>(g: &mut G) -> BoundRect {
            let mut topleft = Point2::arbitrary(g);
            let mut lowright = Point2::arbitrary(g);

            // Make sure that topleft is above and to the left of lowright.
            if lowright.x < topleft.x {
                mem::swap(&mut lowright.x, &mut topleft.x);
            }
            if lowright.y < topleft.y {
                mem::swap(&mut lowright.y, &mut topleft.y);
            }

            BoundRect {
                topleft: topleft,
                lowright: lowright
            }
        }
    }

    impl Arbitrary for Point2 {
        fn arbitrary<G: Gen>(g: &mut G) -> Point2 {
            Point2::new(g.next_u32(), g.next_u32())
        }
    }
}
