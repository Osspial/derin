#![feature(const_fn)]

pub mod geometry;
#[macro_use]
pub mod layout;
mod grid;

use geometry::{Rect, OriginRect, OffsetRect};
use layout::{NodeSpan, PlaceInCell, GridSize};
use grid::{TrackVec, SizeResult};

use std::sync::atomic::{AtomicUsize, Ordering};

pub type Tr = u32;
pub type Px = u32;
pub type Fr = f32;

#[derive(Debug, Clone, Copy)]
pub struct SizeBounds {
    pub min: OriginRect,
    pub max: OriginRect
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

    fn get(&self, Self::Key) -> Option<&WidgetData<Self::Widget>>;
    fn get_mut(&mut self, Self::Key) -> Option<&mut WidgetData<Self::Widget>>;

    fn insert(&mut self, key: Self::Key, widget: Self::Widget) -> Option<WidgetData<Self::Widget>>;
    fn remove(&mut self, key: Self::Key) -> Option<WidgetData<Self::Widget>>;

    fn get_iter(&self) -> <&Self as ContainerRef>::WDIter;
    fn get_iter_mut(&mut self) -> <&Self as ContainerRef>::WDIterMut;
}

pub trait ContainerRef<'a> {
    type Widget: Widget + 'a;
    type WDIter: Iterator<Item = &'a WidgetData<Self::Widget>>;
    type WDIterMut: Iterator<Item = &'a mut WidgetData<Self::Widget>>;
}


pub enum LayoutUpdate<K: Clone + Copy> {
    RowMinSize(Tr, Px),
    RowMaxSize(Tr, Px),
    ColMinSize(Tr, Px),
    ColMaxSize(Tr, Px),

    WidgetMinRect(K, OriginRect),
    WidgetMaxRect(K, OriginRect),

    GridSize(GridSize),
    PixelSize(OriginRect)
}

pub struct UpdateQueue<K: Clone + Copy> {
    update_queue: Vec<LayoutUpdate<K>>,
    id_stack: Vec<(u32, usize)>,
    unsolvable_id: u64
}

impl<K: Clone + Copy> UpdateQueue<K> {
    pub fn new() -> UpdateQueue<K> {
        UpdateQueue {
            update_queue: Vec::new(),
            id_stack: Vec::new(),
            unsolvable_id: 0
        }
    }

    pub fn push_engine<C: Container>(&mut self, engine: &LayoutEngine<C>)
            where for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
    {
        self.id_stack.push((engine.id, self.update_queue.len()));
    }

    /// This method is the heart and soul of the derin layout engine, and is easily the most complex
    /// method it has. This takes a layout engine, iterates over all of the updates performed on that
    /// engine, and performs constraint solving to ensure that all* of the constraints within the engine
    /// are solved.
    ///
    /// <sup>\* The only situation where some constraints may end up violated would be when the maximum
    /// size is less than the minimum size. In that case, minimum size overrides maximum size, as doing
    /// otherwise could cause rendering issues. </sup>
    pub fn pop_engine<C>(&mut self, engine: &mut LayoutEngine<C>)
            where C: Container<Key = K>,
                  for<'a> &'a C: ContainerRef<'a, Widget = C::Widget>
    {
        let id_stack_top = self.id_stack.pop().expect("Attempted to pop engine with no engine on stack");
        assert_eq!(engine.id, id_stack_top.0);

        // Perform the processing only if updates were actually pushed to the update queue.
        if id_stack_top.1 > self.update_queue.len() {
            for update in self.update_queue.drain(id_stack_top.1..) {
                use self::LayoutUpdate::*;

                match update {
                    RowMinSize(tr, px) => engine.grid.get_row_mut(tr).unwrap().set_min_size_master(px),
                    RowMaxSize(tr, px) => engine.grid.get_row_mut(tr).unwrap().set_max_size_master(px),
                    ColMinSize(tr, px) => engine.grid.get_col_mut(tr).unwrap().set_min_size_master(px),
                    ColMaxSize(tr, px) => engine.grid.get_col_mut(tr).unwrap().set_max_size_master(px),

                    WidgetMinRect(k, r) => engine.container.get_mut(k).unwrap().layout_info.size_bounds.min = r,
                    WidgetMaxRect(k, r) => engine.container.get_mut(k).unwrap().layout_info.size_bounds.max = r,

                    GridSize(gs)  => engine.grid.set_grid_size(gs),
                    PixelSize(ps) => engine.desired_size = ps
                }
            }

            // TODO: CACHE HEAP ALLOCS
            let mut frac_tracks: TrackVec<Tr> = TrackVec::new();

            let mut rigid_tracks_widget: Vec<Tr> = Vec::new();
            let mut frac_tracks_widget: Vec<Tr> = Vec::new();

            let mut potential_frac_tracks: TrackVec<Tr> = TrackVec::new();


            // We start out by setting the free space to its maximum possible value.
            let mut free_width = engine.desired_size.width();
            let mut fr_total_width = 0.0;
            let mut free_height = engine.desired_size.height();
            let mut fr_total_height = 0.0;


            // Next, we perform an iteration over the tracks, subtracting from the free space if the track is
            // rigid.
            for (index, track) in engine.grid.col_range_mut(..).unwrap().iter_mut().enumerate() {
                if track.fr_size == 0.0 {
                    track.shrink_size();
                    free_width -= track.size();
                } else {
                    track.expand_size();
                    fr_total_width += track.fr_size;
                    frac_tracks.push_col(index as Tr);
                }
            }

            'update: loop {
                /// Macro for solving the track constraints independent of axis. Because each axis is
                /// independent from the other but the computations required for both are basically the same,
                /// they're placed in a macro to allow DRY.
                macro_rules! track_constraints {
                    ($get_track:ident, $get_track_mut:ident, $push_track:ident,
                     $remove_track:ident, $free_size:expr, $fr_total:expr) => {(|| {
                        //                                                      ^^
                        // Why is this a closure? Consecutive loops in the same function get a warning
                        // for label shadowing if they have the same label, and this supresses that.

                        let mut pft_index = 0;
                        while let Some(track_index) = potential_frac_tracks.$get_track(pft_index).cloned() {
                            let track = engine.grid.$get_track(track_index).unwrap();
                            let mut track_copy = track.clone();

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
                            let mut px_expander = PxExpander::default();
                            while let Some(track_index) = frac_tracks.$get_track(frac_index).map(|t| *t as Tr) {
                                let track = engine.grid.$get_track_mut(track_index).unwrap();

                                let new_size = px_expander.expand($free_size as Fr * track.fr_size / $fr_total);

                                match track.change_size(new_size) {
                                    // If the resize occured without issues, increment frac_index and go on to the next track.
                                    SizeResult::SizeUpscale    |
                                    SizeResult::SizeDownscale => frac_index += 1,

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
                                    SizeResult::NoEffectEq => panic!("Unexpected fractional track scale equality"),
                                }
                            }

                            break;
                        }
                    })()}
                }

                track_constraints!(get_col, get_col_mut, push_col, remove_col, free_width, fr_total_width);
                track_constraints!(get_row, get_row_mut, push_row, remove_row, free_height, fr_total_height);

                for &mut WidgetData{ref mut widget, ref mut layout_info} in engine.container.get_iter_mut() {
                    macro_rules! widget_scale {
                        ($axis:ident, $size:ident, $track_range:ident, $track_range_mut:ident, $free_size:expr) => {
                            let mut min_size_debt = layout_info.size_bounds.min.$size();
                            let mut frac_size = 0;
                            let mut fr_widget = 0.0;

                            // If the widget has been flagged as unsolvable, check to see that is was marked as unsolvable
                            // during the current call to `pop_engine`. If it wasn't, then tentatively mark it as solvable.
                            if let SolveAxis::Unsolvable(unsolvable_id) = layout_info.solvable.$axis {
                                if unsolvable_id != self.unsolvable_id {
                                    layout_info.solvable.$axis = SolveAxis::Solvable;
                                }
                            }

                            for (index, track) in engine.grid.$track_range(layout_info.node_span.x).unwrap().iter().enumerate() {
                                min_size_debt -= track.size();

                                if track.fr_size == 0.0 ||
                                   track.size() < track.min_size_master() ||
                                   track.size() > track.max_size_master()
                                {
                                    rigid_tracks_widget.push(index as Tr);
                                } else {
                                    fr_widget += track.fr_size;
                                    frac_size += track.size();
                                    frac_tracks_widget.push(index as Tr);
                                }
                            }

                            // If the minimum size hasn't been met without expansion and the widget hasn't been marked as
                            // unsolvable during this update, expand the rigid tracks to meet the minimum.
                            if 0 < min_size_debt && !layout_info.solvable.$axis.is_unsolvable_with(self.unsolvable_id) {
                                while 0 < min_size_debt && 0 < rigid_tracks_widget.len() {
                                    // The size that each widget individually needs to expand.
                                    let size_expand = min_size_debt / rigid_tracks_widget.len() as Px;
                                    let mut expand_rem = min_size_debt % rigid_tracks_widget.len() as Px;

                                    let mut rigid_index = 0;
                                    while let Some(track_index) = rigid_tracks_widget.get(rigid_index).cloned() {
                                        let track = &mut engine.grid.$track_range_mut(layout_info.node_span.x).unwrap()[track_index as usize];

                                        let old_size = track.size();
                                        let expansion = size_expand + (expand_rem != 0) as Px;
                                        match track.change_size(old_size + expansion) {
                                            // If the size was upscaled with no issues, just subtract the expansion from `min_size_debt`.
                                            SizeResult::SizeUpscale => {
                                                min_size_debt -= expansion;
                                                rigid_index += 1;
                                            },
                                            // If the size was upscaled but ended up being clamped, reduce `min_size_debt` and also remove
                                            // the clamped rigid track from the list.
                                            SizeResult::SizeUpscaleClamp => {
                                                min_size_debt -= track.size() - old_size;
                                                rigid_tracks_widget.remove(rigid_index);
                                            }
                                            SizeResult::NoEffectEq  |
                                            SizeResult::NoEffectUp => {rigid_tracks_widget.remove(rigid_index);},
                                            SizeResult::NoEffectDown        |
                                            SizeResult::SizeDownscale       |
                                            SizeResult::SizeDownscaleClamp => unreachable!()
                                        }

                                        expand_rem = expand_rem.saturating_sub(1);
                                    }
                                }

                                // If, after all the rigid tracks have been expanded to their maximum size the minimum size *still*
                                // hasn't been met, expand the entire container by the necessary amount to get the fractional tracks
                                // up to size.
                                if 0 < min_size_debt {
                                    let frac_expand = min_size_debt;

                                    // Check that the fractional tracks can actually be expanded by `frac_expand` amount. Note that this
                                    // does not expand the fractional tracks - it only checks that they can get to the full size.
                                    let mut frac_expand_debt = frac_expand;
                                    while 0 < frac_tracks_widget.len() {
                                        let mut need_repass = false;

                                        let mut index = 0;
                                        let mut px_expander = PxExpander::default();
                                        while let Some(track_index) = frac_tracks_widget.get(index).cloned() {
                                            let mut track = engine.grid.$track_range(layout_info.node_span.x).unwrap()[track_index as usize].clone();

                                            let old_size = track.size();
                                            let new_size = px_expander.expand(frac_expand as f32 * track.fr_size / fr_widget);

                                            match track.change_size(new_size) {
                                                SizeResult::SizeUpscale => {
                                                    frac_expand_debt -= track.size() - old_size;
                                                    index += 1;
                                                },
                                                SizeResult::SizeUpscaleClamp => {
                                                    frac_expand_debt -= track.size() - old_size;
                                                    frac_tracks_widget.remove(index);
                                                    need_repass = true;
                                                },
                                                SizeResult::NoEffectEq  |
                                                SizeResult::NoEffectUp => {
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

                                    let frac_proportion = $free_size as f32 / frac_size as f32;
                                    let size_expand = (frac_expand as f32 * frac_proportion).ceil() as Px;
                                    $free_size += size_expand;

                                    rigid_tracks_widget.clear();
                                    frac_tracks_widget.clear();
                                    continue 'update;
                                }
                            }

                            rigid_tracks_widget.clear();
                            frac_tracks_widget.clear();
                        }
                    }

                    widget_scale!(x, width, col_range, col_range_mut, free_width);
                    widget_scale!(y, height, row_range, row_range_mut, free_height);
                }

                break 'update;
            }

            self.unsolvable_id.wrapping_add(1);
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
            id: ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32
        }
    }

    pub fn get_widget(&self, key: C::Key) -> Option<&C::Widget> {
        self.container.get(key).map(|w| &w.widget)
    }

    pub fn get_widget_mut(&mut self, key: C::Key) -> Option<&mut C::Widget> {
        self.container.get_mut(key).map(|w| &mut w.widget)
    }
}

/// Struct for converting `f32`s to `Px` via remainder accumulation.
#[derive(Default)]
struct PxExpander {
    remainder: f32
}

impl PxExpander {
    fn expand(&mut self, new_size_float: f32) -> Px {
        self.remainder += new_size_float.fract();
        let new_size = new_size_float as Px + self.remainder as Px;
        self.remainder -= self.remainder.trunc();

        new_size
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct Solvable {
    x: SolveAxis,
    y: SolveAxis
}

impl Solvable {
    fn new(x: SolveAxis, y: SolveAxis) -> Solvable {
        Solvable {
            x: x,
            y: y
        }
    }
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
