#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;
extern crate dct;

pub mod hints;
mod grid;

use dct::geometry::{Px, Rect, OriginRect, OffsetRect, SizeBounds, Margins};
use hints::{PlaceInCell, Place, GridSize, WidgetHints, TrackHints};
use grid::{TrackVec, SizeResult};

use std::cmp;

pub type Tr = u32;
pub type Fr = f32;

#[derive(Default, Debug, Clone, Copy)]
pub struct WidgetData {
    pub widget_hints: WidgetHints,
    /// The "absolute" size bounds, defined by the widget and not the user, beyond which there may be
    /// rendering errors with the widget.
    pub abs_size_bounds: SizeBounds
}

impl WidgetData {
    pub fn new() -> WidgetData {
        WidgetData::default()
    }
}

pub trait Widget {
    fn set_rect(&mut self, rect: OffsetRect);
}

pub trait Container {
    type Widget: Widget + ?Sized;

    fn update_widget_rects(&mut self, WidgetConstraintSolver);
}


#[derive(Default)]
pub struct LayoutUpdater {
    frac_tracks: TrackVec<Tr>,
    potential_frac_tracks: TrackVec<Tr>,
    rigid_tracks_widget: Vec<Tr>,
    frac_tracks_widget: Vec<Tr>,
    solvable_widgets: Vec<Solvable>
}

impl LayoutUpdater {
    pub fn new() -> LayoutUpdater {
        LayoutUpdater::default()
    }

    /// This method is the heart and soul of the derin layout engine, and is easily the most complex
    /// method it has. This takes a layout engine, iterates over all of the updates performed on that
    /// engine, and performs constraint solving to ensure that all* of the constraints within the engine
    /// are solved.
    ///
    /// Returns `Ok(())` if the size of the engine was not changed, and `Err(new_rect)` if the size WAS
    /// changed.
    ///
    /// <sup>\* The only situation where some constraints may end up violated would be when the maximum
    /// size is less than the minimum size. In that case, minimum size overrides maximum size, as doing
    /// otherwise could cause rendering issues. </sup>
    pub fn update_engine<C>(&mut self, container: &mut C, engine: &mut LayoutEngine) -> Result<(), OriginRect>
            where C: Container
    {
        // We start out by setting the free space to its maximum possible value.
        let mut free_width = engine.desired_size.width();
        let mut fr_total_width = 0.0;
        let mut free_height = engine.desired_size.height();
        let mut fr_total_height = 0.0;

        let old_engine_size = engine.actual_size;

        // Reset the actual size bounds to zero.
        engine.actual_size_bounds = SizeBounds {
            min: OriginRect::min(),
            max: OriginRect::min()
        };

        let mut frac_min_size = OriginRect::min();

        let mut rigid_min_size = OriginRect::min();

        // Next, we perform an iteration over the tracks, subtracting from the free space if the track is
        // rigid.
        macro_rules! first_track_pass {
            ($rect_size:ident, $push_track:ident, $track_range_mut:ident, $free_size:expr, $fr_total:expr) => {
                for (index, track) in engine.grid.$track_range_mut(..).unwrap().iter_mut().enumerate() {
                    let track_fr_size = track.hints().fr_size;
                    if track_fr_size <= 0.0 {
                        track.reset_shrink();
                        rigid_min_size.$rect_size += track.min_size();
                        // To make sure that the maximum size isn't below the minimum needed for this track,
                        // increase the engine maximum size by the rigid track minimum size.
                        engine.actual_size_bounds.max.$rect_size =
                            engine.actual_size_bounds.max.$rect_size.saturating_add(track.min_size());
                        $free_size = $free_size.saturating_sub(track.size());
                    } else {
                        // The engine maximum size isn't expanded in a rigid track because the track won't
                        // expand when the rectangle of the engine is expanded.
                        engine.actual_size_bounds.max.$rect_size =
                            engine.actual_size_bounds.max.$rect_size.saturating_add(track.max_size());
                        track.reset_expand();
                        frac_min_size.$rect_size += track.min_size();
                        $fr_total += track_fr_size;
                        self.frac_tracks.$push_track(index as Tr);
                    }
                }
            }
        }

        first_track_pass!(width, push_col, col_range_mut, free_width, fr_total_width);
        first_track_pass!(height, push_row, row_range_mut, free_height, fr_total_height);


        engine.actual_size_bounds.max =
            engine.desired_size_bounds.bound_rect(engine.actual_size_bounds.max).converge();

        engine.actual_size_bounds.min = OriginRect::new(
            frac_min_size.width() + rigid_min_size.width(),
            frac_min_size.height() + rigid_min_size.height()
        );
        engine.actual_size_bounds.min =
            engine.desired_size_bounds.bound_rect(engine.actual_size_bounds.min).converge();

        engine.actual_size = engine.actual_size_bounds.bound_rect(engine.desired_size).converge();

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
                    while let Some(track_index) = self.potential_frac_tracks.$get_track(pft_index).cloned() {
                        let track = engine.grid.$get_track(track_index).unwrap();
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
                                self.frac_tracks.$push_track(track_index);
                                self.potential_frac_tracks.$remove_track(pft_index);
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
                        let mut fr_divider = FrDivider::new(self.frac_tracks.$num_tracks_method(), $free_size, $fr_total);
                        while let Some(track_index) = self.frac_tracks.$get_track(frac_index).map(|t| *t as Tr) {
                            let track = engine.grid.$get_track_mut(track_index).unwrap();
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
                                    self.frac_tracks.$remove_track(frac_index);
                                    self.potential_frac_tracks.$push_track(track_index as u32);
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

            container.update_widget_rects(WidgetConstraintSolver {
                solvable_index: 0,
                engine: engine,
                updater: self,
                free_width: &mut free_width,
                free_height: &mut free_height,
                fr_total_width: &mut fr_total_width,
                fr_total_height: &mut fr_total_height,
                rigid_min_size: &mut rigid_min_size,
                frac_min_size: &mut frac_min_size
            });

            break 'update;
        }

        self.frac_tracks.clear();
        self.potential_frac_tracks.clear();
        self.rigid_tracks_widget.clear();
        self.frac_tracks_widget.clear();
        self.solvable_widgets.clear();

        if engine.actual_size != old_engine_size {
            Err(engine.actual_size)
        } else {
            Ok(())
        }
    }
}

pub struct LayoutEngine {
    grid: TrackVec,
    /// The pixel size of the layout engine, as requested by the programmer.
    pub desired_size: OriginRect,
    /// The pixel size of the layout engine, accounting for the size bounds of the widgets and the size
    /// bounds of the engine.
    actual_size: OriginRect,
    /// The size bounds of the engine, as requested by the programmer.
    pub desired_size_bounds: SizeBounds,
    /// The size bounds of the engine, accounting for the size bounds of the widgets.
    actual_size_bounds: SizeBounds
}

impl LayoutEngine {
    pub fn new() -> LayoutEngine {
        LayoutEngine {
            grid: TrackVec::new(),
            desired_size: OriginRect::min(),
            actual_size: OriginRect::min(),
            desired_size_bounds: SizeBounds::default(),
            actual_size_bounds: SizeBounds::default()
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

    pub fn actual_size(&self) -> OriginRect {
        self.actual_size
    }

    pub fn actual_size_bounds(&self) -> SizeBounds {
        self.actual_size_bounds
    }
}

pub enum SolveError {
    WidgetUnsolvable,
    Abort
}

pub struct WidgetConstraintSolver<'a> {
    solvable_index: usize,
    engine: &'a mut LayoutEngine,
    updater: &'a mut LayoutUpdater,

    free_width: &'a mut Px,
    free_height: &'a mut Px,
    fr_total_width: &'a mut Fr,
    fr_total_height: &'a mut Fr,
    rigid_min_size: &'a mut OriginRect,
    frac_min_size: &'a mut OriginRect
}

impl<'a> WidgetConstraintSolver<'a> {
    pub fn solve_widget_constraints(&mut self, WidgetData{widget_hints, abs_size_bounds}: WidgetData) -> Result<OffsetRect, SolveError> {
        if 0 < widget_hints.node_span.x.size(0, 1) &&
           0 < widget_hints.node_span.y.size(0, 1)
        {
            let &mut LayoutUpdater {
                ref mut rigid_tracks_widget,
                ref mut frac_tracks_widget,
                ref mut solvable_widgets,
                ..
            } = self.updater;

            let &mut LayoutEngine {
                ref mut grid,
                ref mut actual_size,
                ref mut actual_size_bounds,
                ..
            } = self.engine;

            let solvable = {
                if solvable_widgets.get(self.solvable_index).is_some() {
                    &mut solvable_widgets[self.solvable_index]
                } else {
                    solvable_widgets.push(Solvable::default());
                    solvable_widgets.last_mut().unwrap()
                }
            };

            // The widget size bounds without the margin
            let widget_size_bounds_nomargin = SizeBounds {
                min: abs_size_bounds.bound_rect(widget_hints.size_bounds.min).converge(),
                max: abs_size_bounds.bound_rect(widget_hints.size_bounds.max).converge()
            };
            // The widget size bounds, including the margin
            let widget_size_bounds = {
                let mut wsb = widget_size_bounds_nomargin;
                let margins_x = widget_hints.margins.left + widget_hints.margins.right;
                let margins_y = widget_hints.margins.top + widget_hints.margins.bottom;

                wsb.min.width += margins_x;
                wsb.max.width = wsb.max.width.saturating_add(margins_x);
                wsb.min.height += margins_y;
                wsb.max.height = wsb.max.height.saturating_add(margins_y);
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

                    if let Some(track_slice) = grid.$track_range(widget_hints.node_span.$axis) {
                        for (index, track) in track_slice.iter().enumerate() {
                            let track_fr_size = track.hints().fr_size;
                            px_widget += track.size();
                            min_size_debt = min_size_debt.saturating_sub(track.min_size());

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
                                let track = &mut grid.$track_range_mut(widget_hints.node_span.$axis).unwrap()[track_index as usize];
                                let expansion = rigid_expand + (expand_rem != 0) as Px;

                                if track.min_size() + expansion <= track.max_size() {
                                    min_size_debt = min_size_debt.saturating_sub(expansion);
                                    let new_size = track.min_size() + expansion;

                                    if let Err(expanded) = track.expand_widget_min_size(new_size) {
                                        actual_size_bounds.max.$size =
                                            actual_size_bounds.max.$size().saturating_add(expanded);
                                        actual_size.$size += expanded;

                                        $free_size = $free_size.saturating_sub(expanded);
                                        self.rigid_min_size.$size += expanded;

                                        grid_changed = true;
                                    }
                                    rigid_index += 1;

                                } else {
                                    rigid_tracks_widget.remove(rigid_index);
                                    min_size_debt = min_size_debt.saturating_sub(track.max_size() - track.min_size());

                                    let track_max_size = track.max_size();
                                    if let Err(expanded) = track.expand_widget_min_size(track_max_size) {
                                        actual_size_bounds.max.$size =
                                            actual_size_bounds.max.$size().saturating_add(expanded);
                                        actual_size.$size += expanded;

                                        $free_size = $free_size.saturating_sub(expanded);
                                        self.rigid_min_size.$size += track.max_size() - track.min_size();

                                        grid_changed = true;
                                    }

                                    // we don't continue because TODO PUT WHY
                                }

                                expand_rem = expand_rem.saturating_sub(1);
                            }

                            if 0 == min_size_debt {break}
                        }

                        self.frac_min_size.$size = cmp::max(
                            (widget_size_bounds.min.$size() as Fr * $fr_axis / fr_widget).ceil() as Px,
                            self.frac_min_size.$size()
                        );

                        min_size_debt = min_size_debt.saturating_sub(fr_expand);

                        if 0 < min_size_debt {
                            solvable.$axis = SolveAxis::Unsolvable;
                        }

                        actual_size_bounds.min.$size = self.frac_min_size.$size() + self.rigid_min_size.$size();
                        if actual_size.$size() < actual_size_bounds.min.$size() {
                            grid_changed = true;
                            actual_size.$size = actual_size_bounds.min.$size();
                        }

                        rigid_tracks_widget.clear();
                        frac_tracks_widget.clear();

                        if grid_changed {return Err(SolveError::Abort)}
                    }

                    px_widget
                }}
            }

            // The widget_scale macro isn't guaranteed to return, but if it does it returns the axis size
            // if it does. If it doesn't, the rest of this body is skipped and we go back to the beginning
            // of the `update` loop.
            let size_x = widget_scale!(x, width, col_range, col_range_mut, *self.free_width, *self.fr_total_width);
            let size_y = widget_scale!(y, height, row_range, row_range_mut, *self.free_height, *self.fr_total_height);

            // Perform cell hinting and set
            let widget_origin_rect = OriginRect::new(size_x, size_y);

            let offset = grid.get_cell_offset(
                widget_hints.node_span.x.start.unwrap_or(0),
                widget_hints.node_span.y.start.unwrap_or(0)
            ).unwrap();

            let outer_rect = widget_origin_rect.offset(offset);
            let cell_hinter = CellHinter::new(outer_rect, widget_hints.place_in_cell);

            self.solvable_index += 1;
            cell_hinter.hint(widget_size_bounds_nomargin, widget_hints.margins).map_err(|_| SolveError::WidgetUnsolvable)
        } else {
            Err(SolveError::WidgetUnsolvable)
        }
    }
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
    outer_rect: OffsetRect,
    place_in_or: PlaceInCell
}

impl CellHinter {
    pub fn new(outer_rect: OffsetRect, place_in_or: PlaceInCell) -> CellHinter {
        CellHinter {
            outer_rect: outer_rect,
            place_in_or: place_in_or,
        }
    }

    pub fn hint(&self, bounds: SizeBounds, margins: Margins) -> Result<OffsetRect, HintError> {
        let margins_x = margins.left + margins.right;
        let margins_y = margins.top + margins.bottom;

        if bounds.min.width() + margins_x > self.outer_rect.width() ||
           bounds.min.height() + margins_y > self.outer_rect.height()
        {
            return Err(HintError::ORTooSmall)
        }

        let mut inner_rect = OffsetRect::default();

        macro_rules! place_on_axis {
            ($axis:ident, $size:ident, $front_margin:expr, $back_margin:expr) => {
                match self.place_in_or.$axis {
                    Place::Stretch => {
                        inner_rect.topleft.$axis = self.outer_rect.topleft.$axis + $front_margin;
                        inner_rect.lowright.$axis = self.outer_rect.lowright.$axis - $back_margin;

                        if inner_rect.$size() > bounds.max.$size() {
                            let size_diff = inner_rect.$size() - bounds.max.$size();

                            inner_rect.topleft.$axis += size_diff / 2 + size_diff % 2;
                            inner_rect.lowright.$axis -= size_diff / 2;
                        }
                    },
                    Place::Start => {
                        inner_rect.topleft.$axis = self.outer_rect.topleft.$axis + $front_margin + $front_margin;
                        inner_rect.lowright.$axis = self.outer_rect.topleft.$axis + bounds.min.$size() + $front_margin;
                    },
                    Place::End => {
                        inner_rect.lowright.$axis = self.outer_rect.lowright.$axis - $back_margin;
                        inner_rect.topleft.$axis = self.outer_rect.lowright.$axis - bounds.min.$size() - $back_margin;
                    },
                    Place::Center => {
                        let center = (self.outer_rect.topleft.$axis + self.outer_rect.lowright.$axis) / 2;
                        inner_rect.topleft.$axis = center - bounds.min.$size() / 2;
                        inner_rect.lowright.$axis = center + bounds.min.$size() / 2;

                        if inner_rect.$size() > bounds.max.$size() {
                            let size_diff = inner_rect.$size() - bounds.max.$size();

                            inner_rect.topleft.$axis += size_diff / 2 + size_diff % 2;
                            inner_rect.lowright.$axis -= size_diff / 2;
                        }

                        let front_margin_shift = $front_margin.saturating_sub(inner_rect.topleft.$axis - self.outer_rect.topleft.$axis);
                        let back_margin_shift = $back_margin.saturating_sub(self.outer_rect.lowright.$axis - inner_rect.lowright.$axis);
                        inner_rect.topleft.$axis += front_margin_shift;
                        inner_rect.lowright.$axis += front_margin_shift;
                        inner_rect.topleft.$axis -= back_margin_shift;
                        inner_rect.lowright.$axis -= back_margin_shift;
                    }
                }
            }
        }

        place_on_axis!(x, width, margins.left, margins.right);
        place_on_axis!(y, height, margins.top, margins.bottom);

        Ok(inner_rect)
    }
}

enum HintError {
    /// The outer rect is smaller than the minimum size bound, making constraint unsolvable
    ORTooSmall
}

trait Converge<T> {
    /// Given a result where both `Ok` and `Err` contain the same type, "converge" those values
    /// to just one value.
    fn converge(self) -> T;
}

impl<T> Converge<T> for Result<T, T> {
    fn converge(self) -> T {
        match self {
            Ok(t) |
            Err(t) => t
        }
    }
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

    impl Arbitrary for OffsetRect {
        fn arbitrary<G: Gen>(g: &mut G) -> OffsetRect {
            let mut topleft = Point::arbitrary(g);
            let mut lowright = Point::arbitrary(g);

            // Make sure that topleft is above and to the left of lowright.
            if lowright.x < topleft.x {
                mem::swap(&mut lowright.x, &mut topleft.x);
            }
            if lowright.y < topleft.y {
                mem::swap(&mut lowright.y, &mut topleft.y);
            }

            OffsetRect {
                topleft: topleft,
                lowright: lowright
            }
        }
    }

    impl Arbitrary for Point {
        fn arbitrary<G: Gen>(g: &mut G) -> Point {
            Point::new(g.next_u32(), g.next_u32())
        }
    }
}
