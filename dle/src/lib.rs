#![feature(const_fn)]

#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

pub mod geometry;
pub mod widget_hints;
mod grid;

use geometry::{Rect, OriginRect, OffsetRect};
use widget_hints::{NodeSpan, PlaceInCell, Place, GridSize};
use grid::{TrackVec, SizeResult};

use std::cmp;
use std::sync::atomic::{AtomicUsize, Ordering};

pub type Tr = u32;
pub type Px = u32;
pub type Fr = f32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeBounds {
    pub min: OriginRect,
    pub max: OriginRect
}

impl SizeBounds {
    /// Bound a rectangle to be within the size bounds. Returns `Ok` if the rect wasn't bounded, and
    /// `Err` if it was bounded.
    fn bound_rect(self, mut desired_size: OriginRect) -> Result<OriginRect, OriginRect> {
        let mut size_bounded = false;

        if desired_size.width() < self.min.width() {
            desired_size.lowright.x = self.min.width();
            size_bounded = true;
        } else if desired_size.width() > self.max.width() {
            desired_size.lowright.x = self.max.width();
            size_bounded = true;
        }

        if desired_size.height() < self.min.height() {
            desired_size.lowright.y = self.min.height();
            size_bounded = true;
        } else if desired_size.height() > self.max.height() {
            desired_size.lowright.y = self.max.height();
            size_bounded = true;
        }

        if !size_bounded {
            Ok(desired_size)
        } else {
            Err(desired_size)
        }
    }
}

impl Default for SizeBounds {
    fn default() -> SizeBounds {
        SizeBounds {
            min: OriginRect::min(),
            max: OriginRect::max()
        }
    }
}

pub struct WidgetData<W: Widget> {
    pub widget: W,
    pub layout_info: WidgetLayoutInfo
}

impl<W: Widget> WidgetData<W> {
    pub fn new(widget: W) -> WidgetData<W> {
        WidgetData {
            widget: widget,
            layout_info: WidgetLayoutInfo::default()
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct WidgetLayoutInfo {
    pub size_bounds: SizeBounds,
    pub node_span: NodeSpan,
    pub placement: PlaceInCell,
    solvable: Solvable
}

pub trait Widget {
    fn set_rect(&mut self, rect: OffsetRect);
}

pub trait Container
        where for<'a> &'a Self: ContainerRef<'a, Widget = Self::Widget> {
    type Widget: Widget;
    type Key: Clone + Copy;

    fn get_widget(&self, Self::Key) -> Option<&WidgetData<Self::Widget>>;
    fn get_widget_mut(&mut self, Self::Key) -> Option<&mut WidgetData<Self::Widget>>;

    fn insert_widget(&mut self, key: Self::Key, widget: Self::Widget) -> Option<Self::Widget>;
    fn remove_widget(&mut self, key: Self::Key) -> Option<Self::Widget>;

    fn get_widget_iter(&self) -> <&Self as ContainerRef>::WDIter;
    fn get_widget_iter_mut(&mut self) -> <&Self as ContainerRef>::WDIterMut;
}

pub trait ContainerRef<'a> {
    type Widget: Widget + 'a;
    type WDIter: Iterator<Item = &'a WidgetData<Self::Widget>>;
    type WDIterMut: Iterator<Item = &'a mut WidgetData<Self::Widget>>;
}


#[derive(Debug, Clone, Copy)]
pub enum LayoutUpdate<K: Clone + Copy> {
    RowMinSize(Tr, Px),
    RowMaxSize(Tr, Px),
    ColMinSize(Tr, Px),
    ColMaxSize(Tr, Px),

    WidgetSizeBounds(K, SizeBounds),
    WidgetNodeSpan(K, NodeSpan),
    WidgetPlaceInCell(K, PlaceInCell),

    GridSize(GridSize),
    PixelSize(OriginRect),
    PixelSizeBounds(SizeBounds)
}

#[derive(Default)]
struct UQHeapCache {
    frac_tracks: TrackVec<Tr>,
    potential_frac_tracks: TrackVec<Tr>,
    rigid_tracks_widget: Vec<Tr>,
    frac_tracks_widget: Vec<Tr>
}

struct IdStackEntry {
    engine_id: u32,
    update_queue_index: usize,
    container_contents_changed: bool
}

pub struct UpdateQueue<K: Clone + Copy> {
    update_queue: Vec<LayoutUpdate<K>>,
    id_stack: Vec<IdStackEntry>,
    heap_cache: UQHeapCache,
    unsolvable_id: u64
}

impl<K: Clone + Copy> UpdateQueue<K> {
    pub fn new() -> UpdateQueue<K> {
        UpdateQueue {
            update_queue: Vec::new(),
            id_stack: Vec::new(),
            heap_cache: UQHeapCache::default(),
            unsolvable_id: 0
        }
    }

    pub fn push_engine<C: Container>(&mut self, engine: &LayoutEngine<C>)
            where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
    {
        self.id_stack.push(IdStackEntry {
            engine_id: engine.id,
            update_queue_index: self.update_queue.len(),
            container_contents_changed: false
        });
    }

    pub fn engine_is_top<C: Container>(&mut self, engine: &LayoutEngine<C>) -> bool
            where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
    {
        self.id_stack.last().map(|id_stack_top| id_stack_top.engine_id) == Some(engine.id)
    }

    pub fn push_update(&mut self, update: LayoutUpdate<K>) {
        assert_ne!(0, self.id_stack.len());
        self.update_queue.push(update);
    }

    pub fn insert_widget<C: Container>(&mut self, key: K, widget: C::Widget,
                                       engine: &mut LayoutEngine<C>)
            -> Option<C::Widget>
            where C: Container<Key = K>,
                  for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
    {
        let id_stack_top = self.id_stack.last_mut().expect("Attempted to modify engine with no engine on stack");
        assert_eq!(engine.id, id_stack_top.engine_id);
        id_stack_top.container_contents_changed = true;

        engine.container.insert_widget(key, widget)
    }

    pub fn remove_widget<C: Container>(&mut self, key: K, engine: &mut LayoutEngine<C>)
            -> Option<C::Widget>
            where C: Container<Key = K>,
                  for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
    {
        let id_stack_top = self.id_stack.last_mut().expect("Attempted to modify engine with no engine on stack");
        assert_eq!(engine.id, id_stack_top.engine_id);
        id_stack_top.container_contents_changed = true;

        engine.container.remove_widget(key)
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
    pub fn pop_engine<C>(&mut self, engine: &mut LayoutEngine<C>)
            where C: Container<Key = K>,
                  for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>,
                  K: ::std::fmt::Debug
    {
        let id_stack_top = self.id_stack.pop().expect("Attempted to pop engine with no engine on stack");
        assert_eq!(engine.id, id_stack_top.engine_id);

        // Perform the processing only if updates were actually pushed to the update queue, or if widgets
        // were added or removed from the container.
        if id_stack_top.update_queue_index < self.update_queue.len() ||
           id_stack_top.container_contents_changed
        {
            for update in self.update_queue.drain(id_stack_top.update_queue_index..) {
                use self::LayoutUpdate::*;

                match update {
                    RowMinSize(tr, px) => engine.grid.get_row_mut(tr).unwrap().set_min_size_master(px),
                    RowMaxSize(tr, px) => engine.grid.get_row_mut(tr).unwrap().set_max_size_master(px),
                    ColMinSize(tr, px) => engine.grid.get_col_mut(tr).unwrap().set_min_size_master(px),
                    ColMaxSize(tr, px) => engine.grid.get_col_mut(tr).unwrap().set_max_size_master(px),

                    WidgetSizeBounds(k, sb) => engine.container.get_widget_mut(k).unwrap().layout_info.size_bounds = sb,
                    WidgetNodeSpan(k, ns) => engine.container.get_widget_mut(k).unwrap().layout_info.node_span = ns,
                    WidgetPlaceInCell(k, pic) => engine.container.get_widget_mut(k).unwrap().layout_info.placement = pic,

                    GridSize(gs)  => engine.grid.set_grid_size(gs),
                    PixelSize(ps) => engine.desired_size = ps,
                    PixelSizeBounds(psb) => engine.desired_size_bounds = psb
                }
            }

            let mut frac_tracks = &mut self.heap_cache.frac_tracks;
            let mut potential_frac_tracks = &mut self.heap_cache.potential_frac_tracks;

            let mut rigid_tracks_widget = &mut self.heap_cache.rigid_tracks_widget;
            let mut frac_tracks_widget = &mut self.heap_cache.frac_tracks_widget;

            // We start out by setting the free space to its maximum possible value.
            let mut free_width = engine.desired_size.width();
            let mut fr_total_width = 0.0;
            let mut free_height = engine.desired_size.height();
            let mut fr_total_height = 0.0;

            // Reset the actual size bounds to zero.
            engine.actual_size_bounds = SizeBounds {
                min: OriginRect::min(),
                max: OriginRect::min()
            };

            // Next, we perform an iteration over the tracks, subtracting from the free space if the track is
            // rigid.
            macro_rules! first_track_pass {
                ($axis:ident, $push_track:ident, $track_range_mut:ident, $free_size:expr, $fr_total:expr) => {
                    for (index, track) in engine.grid.$track_range_mut(..).unwrap().iter_mut().enumerate() {
                        engine.actual_size_bounds.min.lowright.$axis += track.min_size_master();

                        if track.fr_size <= 0.0 {
                            track.shrink_size();
                            $free_size -= track.size();
                        } else {
                            engine.actual_size_bounds.max.lowright.$axis =
                                engine.actual_size_bounds.max.lowright.$axis.saturating_add(track.max_size_master());
                            track.expand_size();
                            $fr_total += track.fr_size;
                            frac_tracks.$push_track(index as Tr);
                        }
                    }
                }
            }

            first_track_pass!(x, push_col, col_range_mut, free_width, fr_total_width);
            first_track_pass!(y, push_row, row_range_mut, free_height, fr_total_height);

            engine.actual_size_bounds.min =
                engine.desired_size_bounds.bound_rect(engine.actual_size_bounds.min).converge();
            engine.actual_size_bounds.max =
                engine.desired_size_bounds.bound_rect(engine.actual_size_bounds.max).converge();

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
                        while let Some(track_index) = potential_frac_tracks.$get_track(pft_index).cloned() {
                            let track = engine.grid.$get_track(track_index).unwrap();
                            let mut track_copy = track.clone();

                            // While this isn't an *exact* calculation of the new size of the track (due to remainders and whatnot
                            // as implemented in `FrDivider`), it's a good enough estimate.
                            let new_size = (($free_size + track.size()) as Fr * track.fr_size / ($fr_total + track.fr_size)) as Px;

                            match track_copy.change_size(new_size) {
                                // If the track can be freely rescaled, add it back to `frac_tracks` and remove it from
                                // `potential_frac_tracks`.
                                SizeResult::SizeUpscale    |
                                SizeResult::SizeDownscale => {
                                    $free_size += track.size();
                                    $fr_total += track.fr_size;
                                    frac_tracks.$push_track(track_index);
                                    potential_frac_tracks.$remove_track(pft_index);
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
                            let mut px_expander = FrDivider::new(frac_tracks.$num_tracks_method(), $free_size, $fr_total);
                            while let Some(track_index) = frac_tracks.$get_track(frac_index).map(|t| *t as Tr) {
                                let track = engine.grid.$get_track_mut(track_index).unwrap();

                                let new_size = px_expander.divvy(track.fr_size);

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
                                        $fr_total -= track.fr_size;
                                        frac_tracks.$remove_track(frac_index);
                                        potential_frac_tracks.$push_track(track_index as u32);
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

                for &mut WidgetData{ref mut widget, ref mut layout_info} in engine.container.get_widget_iter_mut() {
                    macro_rules! widget_scale {
                        ($axis:ident, $size:ident, $track_range:ident, $track_range_mut:ident, $free_size:expr, $fr_axis:expr) => {{
                            let mut min_size_debt = layout_info.size_bounds.min.$size();
                            let mut fr_widget = 0.0;
                            let mut axis_size = 0;

                            // If the widget has been flagged as unsolvable, check to see that is was marked as unsolvable
                            // during the current call to `pop_engine`. If it wasn't, then tentatively mark it as solvable.
                            if let SolveAxis::Unsolvable(unsolvable_id) = layout_info.solvable.$axis {
                                if unsolvable_id != self.unsolvable_id {
                                    layout_info.solvable.$axis = SolveAxis::Solvable;
                                }
                            }

                            for (index, track) in engine.grid.$track_range(layout_info.node_span.$axis).expect("Node span larger than grid").iter().enumerate() {
                                min_size_debt = min_size_debt.saturating_sub(track.size());
                                axis_size += track.size();

                                if track.fr_size == 0.0 ||
                                   track.size() < track.min_size_master() ||
                                   track.size() > track.max_size_master()
                                {
                                    rigid_tracks_widget.push(index as Tr);
                                } else {
                                    fr_widget += track.fr_size;
                                    frac_tracks_widget.push(index as Tr);
                                }
                            }

                            // Calculate the minimum size the layout engine can be to contain the widget, and expand the
                            // engine minimum size to match that.
                            let min_size_axis = (layout_info.size_bounds.min.$size() as Fr * $fr_axis / fr_widget).ceil() as Px;
                            engine.actual_size_bounds.min.lowright.$axis = cmp::max(
                                min_size_axis,
                                engine.actual_size_bounds.min.$size()
                            );

                            // If the minimum size hasn't been met without expansion and the widget hasn't been marked as
                            // unsolvable during this update, expand the rigid tracks to meet the minimum.
                            if 0 < min_size_debt && !layout_info.solvable.$axis.is_unsolvable_with(self.unsolvable_id) {
                                while 0 < min_size_debt && 0 < rigid_tracks_widget.len() {
                                    // The size that each widget individually needs to expand.
                                    let size_expand = min_size_debt / rigid_tracks_widget.len() as Px;
                                    let mut expand_rem = min_size_debt % rigid_tracks_widget.len() as Px;

                                    let mut rigid_index = 0;
                                    while let Some(track_index) = rigid_tracks_widget.get(rigid_index).cloned() {
                                        let track = &mut engine.grid.$track_range_mut(layout_info.node_span.$axis).unwrap()[track_index as usize];

                                        let old_size = track.size();
                                        let expansion = size_expand + (expand_rem != 0) as Px;
                                        match track.change_size(old_size + expansion) {
                                            // If the size was upscaled with no issues, just subtract the expansion from `min_size_debt`.
                                            SizeResult::SizeUpscale => rigid_index += 1,

                                            SizeResult::NoEffectEq        |
                                            SizeResult::NoEffectUp        |
                                            SizeResult::SizeUpscaleClamp => {rigid_tracks_widget.remove(rigid_index);},

                                            SizeResult::NoEffectDown        |
                                            SizeResult::SizeDownscale       |
                                            SizeResult::SizeDownscaleClamp => unreachable!()
                                        }
                                        let expanded_size = track.size() - old_size;
                                        engine.actual_size_bounds.min.lowright.$axis += expanded_size;
                                        engine.actual_size_bounds.max.lowright.$axis += expanded_size;
                                        min_size_debt = min_size_debt.saturating_sub(expansion);

                                        expand_rem = expand_rem.saturating_sub(1);
                                    }
                                }

                                // If, after all the rigid tracks have been expanded to their maximum size the minimum size *still*
                                // hasn't been met, expand the entire container by the necessary amount to get the fractional tracks
                                // up to size.
                                if 0 < min_size_debt {
                                    let frac_expand = min_size_debt;
                                    let desired_size = axis_size + frac_expand;

                                    // Check that the fractional tracks can actually be expanded by `frac_expand` amount. Note that this
                                    // does not actually expand the fractional tracks - it only checks that they can get to the full size.
                                    // Expansion is deferred to the main rescaling loop in the `track_constraints!` macro.
                                    let mut frac_expand_debt = frac_expand;
                                    while 0 < frac_tracks_widget.len() {
                                        let mut need_repass = false;

                                        let mut index = 0;
                                        let mut px_expander = FrDivider::new(frac_tracks_widget.len() as Tr, desired_size, fr_widget);
                                        while let Some(track_index) = frac_tracks_widget.get(index).cloned() {
                                            let mut track = engine.grid.$track_range(layout_info.node_span.$axis).unwrap()[track_index as usize].clone();

                                            let old_size = track.size();
                                            let new_size = px_expander.divvy(track.fr_size);

                                            match track.change_size(new_size) {
                                                SizeResult::SizeUpscale => {
                                                    frac_expand_debt = frac_expand_debt.saturating_sub(track.size() - old_size);
                                                    index += 1;
                                                },
                                                SizeResult::SizeUpscaleClamp => {
                                                    frac_expand_debt = frac_expand_debt.saturating_sub(track.size() - old_size);
                                                    fr_widget -= track.fr_size;
                                                    frac_tracks_widget.remove(index);
                                                    need_repass = true;
                                                },
                                                SizeResult::NoEffectEq  |
                                                SizeResult::NoEffectUp => {
                                                    fr_widget -= track.fr_size;
                                                    frac_tracks_widget.remove(index);
                                                    need_repass = true;
                                                },
                                                SizeResult::NoEffectDown        |
                                                SizeResult::SizeDownscale       |
                                                SizeResult::SizeDownscaleClamp => panic!("Unexpected fractional size downscale")
                                            }
                                        }

                                        if !need_repass {break}
                                    }

                                    // If, after all of the fractional tracks have been maxed out there *still* isn't enough room, the
                                    // constraints cannot be solved. Flag the widget as unsolvable and move on.
                                    //
                                    // I don't have the free width expansion inside of an else clause here, becaue expanding the tracks
                                    // as much as possible should hopefully minimize visual bugs.
                                    if 0 < frac_expand_debt {
                                        layout_info.solvable.$axis = SolveAxis::Unsolvable(self.unsolvable_id);
                                    }

                                    let frac_proportion = $fr_axis / fr_widget;
                                    let size_expand = (frac_expand as f32 * frac_proportion).ceil() as Px;
                                    $free_size += size_expand;

                                    let new_engine_rect = {
                                        let mut ner = engine.actual_size;
                                        ner.lowright.$axis += size_expand;
                                        ner
                                    };
                                    match engine.actual_size_bounds.bound_rect(new_engine_rect) {
                                        Ok(r) => engine.actual_size = r,
                                        Err(r) => {
                                            engine.actual_size = r;
                                            layout_info.solvable.$axis = SolveAxis::Unsolvable(self.unsolvable_id);
                                        }
                                    }
                                }

                                rigid_tracks_widget.clear();
                                frac_tracks_widget.clear();
                                continue 'update;
                            }

                            rigid_tracks_widget.clear();
                            frac_tracks_widget.clear();

                            axis_size
                        }}
                    }

                    // The widget_scale macro isn't guaranteed to return, but if it does it returns the axis size
                    // if it does. If it doesn't, the rest of this body is skipped and we go back to the beginning
                    // of the `update` loop.
                    let size_x = widget_scale!(x, width, col_range, col_range_mut, free_width, fr_total_width);
                    let size_y = widget_scale!(y, height, row_range, row_range_mut, free_height, fr_total_height);

                    // Perform cell hinting and set
                    let widget_origin_rect = OriginRect::new(size_x, size_y);

                    let offset = engine.grid.get_cell_offset(
                        layout_info.node_span.x.start.unwrap_or(0),
                        layout_info.node_span.y.start.unwrap_or(0)
                    ).unwrap();

                    let outer_rect = widget_origin_rect.offset(offset);
                    let cell_hinter = CellHinter::new(outer_rect, layout_info.placement);

                    if let Ok(widget_rect) = cell_hinter.hint_with_bounds(layout_info.size_bounds) {
                        widget.set_rect(widget_rect);
                    }
                }

                break 'update;
            }

            self.unsolvable_id.wrapping_add(1);
            frac_tracks.clear();
            potential_frac_tracks.clear();
            rigid_tracks_widget.clear();
            frac_tracks_widget.clear();
        }
    }
}

pub struct LayoutEngine<C: Container>
        where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
{
    container: C,
    grid: TrackVec,
    desired_size: OriginRect,
    actual_size: OriginRect,
    desired_size_bounds: SizeBounds,
    actual_size_bounds: SizeBounds,
    id: u32
}

impl<C: Container> LayoutEngine<C>
        where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
{
    pub fn new(container: C) -> LayoutEngine<C> {
        static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

        LayoutEngine {
            container: container,
            grid: TrackVec::new(),
            desired_size: OriginRect::min(),
            actual_size: OriginRect::min(),
            desired_size_bounds: SizeBounds::default(),
            actual_size_bounds: SizeBounds::default(),
            id: ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32
        }
    }

    pub fn get_widget(&self, key: C::Key) -> Option<&C::Widget> {
        self.container.get_widget(key).map(|w| &w.widget)
    }

    pub fn get_widget_mut(&mut self, key: C::Key) -> Option<&mut C::Widget> {
        self.container.get_widget_mut(key).map(|w| &mut w.widget)
    }

    pub fn desired_size(&self) -> OriginRect {
        self.desired_size
    }

    pub fn actual_size(&self) -> OriginRect {
        self.actual_size
    }

    pub fn desired_size_bounds(&self) -> SizeBounds {
        self.desired_size_bounds
    }

    pub fn actual_size_bounds(&self) -> SizeBounds {
        self.actual_size_bounds
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
    Unsolvable(u64)
}

impl SolveAxis {
    fn is_unsolvable_with(self, unsolvable_id: u64) -> bool {
        if let SolveAxis::Unsolvable(id) = self {
            if id == unsolvable_id {
                return true;
            }
        }
        false
    }
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

    pub fn hint_with_bounds(&self, bounds: SizeBounds) -> Result<OffsetRect, HintError> {
        if bounds.min.width() > self.outer_rect.width() ||
           bounds.min.height() > self.outer_rect.height()
        {
            return Err(HintError::ORTooSmall)
        }

        let mut inner_rect = OffsetRect::default();

        macro_rules! place_on_axis {
            ($axis:ident $size:ident) => {
                match self.place_in_or.$axis {
                    Place::Stretch => {
                        inner_rect.topleft.$axis = self.outer_rect.topleft.$axis;
                        inner_rect.lowright.$axis = self.outer_rect.lowright.$axis;

                        if inner_rect.$size() > bounds.max.$size() {
                            let size_diff = inner_rect.$size() - bounds.max.$size();

                            inner_rect.topleft.$axis += size_diff / 2 + size_diff % 2;
                            inner_rect.lowright.$axis -= size_diff / 2;
                        }
                    },
                    Place::Start => {
                        inner_rect.topleft.$axis = self.outer_rect.topleft.$axis;
                        inner_rect.lowright.$axis = self.outer_rect.topleft.$axis + bounds.min.$size();
                    },
                    Place::End => {
                        inner_rect.lowright.$axis = self.outer_rect.lowright.$axis;
                        inner_rect.topleft.$axis = self.outer_rect.lowright.$axis - bounds.min.$size();
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
                    }
                }
            }
        }

        place_on_axis!(x width);
        place_on_axis!(y height);

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
