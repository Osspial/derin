// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox, GeoBox};

use core::LoopFlow;
use core::event::{EventOps, WidgetEvent, InputState};
use core::tree::{WidgetIdent, UpdateTag, WidgetSummary, Widget, Parent, OnFocus};
use core::render::FrameRectStack;
use core::popup::ChildPopupsMut;
use dct::layout::{SizeBounds, WidgetPos, GridSize, WidgetSpan, TrackHints};

use std::cell::RefCell;

use gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame, RenderString};
use dle::{GridEngine, UpdateHeapCache, SolveError};

use arrayvec::ArrayVec;

/// A page within a greater list of tabs.
///
/// Only has a use as a child of a [`TabList`].
///
/// [`TabList`]: ./struct.TabList.html
#[derive(Debug, Clone)]
pub struct TabPage<W> {
    title: RenderString,
    /// The widget that's displayed within the tab page.
    pub page: W,
    open: bool,
    rect: BoundBox<Point2<i32>>
}

/// A list of tabs.
///
/// This widget lets you display a single widget at a time, from a greater selection of widgets.
/// Users can switch between these widgets by clicking on a list of tabs at the top of the widget.
#[derive(Debug, Clone)]
pub struct TabList<W> {
    update_tag: UpdateTag,
    rect: BoundBox<Point2<i32>>,
    layout_engine: GridEngine,

    tabs: Vec<TabPage<W>>
}

impl<W> TabPage<W> {
    /// Create a new tab page, with the given title and contained widget.
    pub fn new(title: String, page: W) -> TabPage<W> {
        TabPage {
            title: RenderString::new(title),
            page,
            open: true,
            rect: BoundBox::new2(0, 0, 0, 0)
        }
    }


    /// Retrieves a reference to the tab's title.
    pub fn string(&self) -> &str {
        self.title.string()
    }

    /// Retrieves the tab's title, for mutation.
    pub fn string_mut(&mut self) -> &mut String {
        self.title.string_mut()
    }
}

impl<W> TabList<W> {
    /// Create a new list of tabs.
    pub fn new(tabs: Vec<TabPage<W>>) -> TabList<W> {
        TabList {
            update_tag: UpdateTag::new(),
            rect: BoundBox::new2(0, 0, 0, 0),
            layout_engine: GridEngine::new(),

            tabs
        }
    }

    /// Retrieves a reference to the tab list.
    pub fn tabs(&self) -> &[TabPage<W>] {
        &self.tabs
    }

    /// Retrieves a reference to the tab list, for mutation.
    ///
    /// Calling this function forces the tab list to be re-drawn, so you're discouraged from calling
    /// it unless you're actually changing the contents.
    pub fn tabs_mut(&mut self) -> &mut Vec<TabPage<W>> {
        self.update_tag.mark_update_child().mark_update_layout().mark_render_self();
        &mut self.tabs
    }
}

impl<A, F, W> Widget<A, F> for TabList<W>
    where A: 'static,
          F: PrimFrame,
          W: Widget<A, F>
{
    #[inline]
    fn update_tag(&self) -> &UpdateTag {
        &self.update_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<Point2<i32>> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<Point2<i32>> {
        self.update_tag.mark_update_layout().mark_render_self();
        &mut self.rect
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.layout_engine.actual_size_bounds()
    }

    fn render(&mut self, frame: &mut FrameRectStack<F>) {
        for tab in &mut self.tabs {
            frame.upload_primitives(ArrayVec::from([
                ThemedPrim {
                    theme_path: "Tab",
                    min: Point2::new(
                        RelPoint::new(-1.0, tab.rect.min.x),
                        RelPoint::new(-1.0, tab.rect.min.y),
                    ),
                    max: Point2::new(
                        RelPoint::new(-1.0, tab.rect.max.x),
                        RelPoint::new(-1.0, tab.rect.max.y)
                    ),
                    prim: Prim::Image,
                    rect_px_out: None
                },
                ThemedPrim {
                    theme_path: "Tab",
                    min: Point2::new(
                        RelPoint::new(-1.0, tab.rect.min.x),
                        RelPoint::new(-1.0, tab.rect.min.y),
                    ),
                    max: Point2::new(
                        RelPoint::new(-1.0, tab.rect.max.x),
                        RelPoint::new(-1.0, tab.rect.max.y)
                    ),
                    prim: Prim::String(&mut tab.title),
                    rect_px_out: None
                },
            ]));
        }
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEvent, _: InputState, _: Option<ChildPopupsMut<A, F>>, bubble_source: &[WidgetIdent]) -> EventOps<A, F> {
        // Change tab selection.
        if let (0, &WidgetEvent::MouseUp{in_widget: true, pressed_in_widget: true, pos, down_pos, ..}) = (bubble_source.len(), &event) {
            let mut state_changed = false;
            let (mut old_open, mut new_open) = (None, None);
            for (index, tab) in self.tabs.iter_mut().enumerate() {
                let is_open = tab.rect.contains(pos) && tab.rect.contains(down_pos);
                state_changed |= is_open != tab.open;

                if tab.open && old_open == None {
                    old_open = Some(index);
                }
                if is_open && new_open == None {
                    new_open = Some(index);
                }

                tab.open = is_open;
            }
            if state_changed && !(old_open == new_open || new_open == None) {
                self.update_tag.mark_update_layout().mark_render_self();
            }
        }

        EventOps {
            action: None,
            focus: None,
            bubble: event.default_bubble() || bubble_source.len() != 0,
            cursor_pos: None,
            cursor_icon: None,
            popup: None
        }
    }

    fn accepts_focus(&self) -> OnFocus {
        OnFocus::FocusChild
    }
}

impl<A, F, W> Parent<A, F> for TabList<W>
    where A: 'static,
          F: PrimFrame,
          W: Widget<A, F>
{
    fn num_children(&self) -> usize {
        self.tabs.len()
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<A, F>>> {
        if let WidgetIdent::Num(index) = widget_ident {
            self.tabs.get(index as usize).map(|t| WidgetSummary::new(widget_ident, index as usize, &t.page).to_dyn())
        } else {
            None
        }
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        if let WidgetIdent::Num(index) = widget_ident {
            self.tabs.get_mut(index as usize).map(|t| WidgetSummary::new_mut(widget_ident, index as usize, &mut t.page).to_dyn_mut())
        } else {
            None
        }
    }

    fn children<'a, G, R>(&'a self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a Widget<A, F>>) -> LoopFlow<R>
    {
        for (index, tab) in self.tabs.iter().enumerate() {
            match for_each(WidgetSummary::new(WidgetIdent::Num(index as u32), index, &tab.page)) {
                LoopFlow::Continue => (),
                LoopFlow::Break(r) => return Some(r)
            }
        }

        None
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each: G) -> Option<R>
        where A: 'a,
              G: FnMut(WidgetSummary<&'a mut Widget<A, F>>) -> LoopFlow<R>
    {
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            match for_each(WidgetSummary::new_mut(WidgetIdent::Num(index as u32), index, &mut tab.page)) {
                LoopFlow::Continue => (),
                LoopFlow::Break(r) => return Some(r)
            }
        }

        None
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<A, F>>> {
        self.tabs.get(index).map(|t| WidgetSummary::new(WidgetIdent::Num(index as u32), index, &t.page).to_dyn())
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<A, F>>> {
        self.tabs.get_mut(index).map(|t| WidgetSummary::new_mut(WidgetIdent::Num(index as u32), index, &mut t.page).to_dyn_mut())
    }

    fn update_child_layout(&mut self) {
        #[derive(Default)]
        struct HeapCache {
            update_heap_cache: UpdateHeapCache,
            hints_vec: Vec<WidgetPos>,
            rects_vec: Vec<Result<BoundBox<Point2<i32>>, SolveError>>
        }
        thread_local! {
            static HEAP_CACHE: RefCell<HeapCache> = RefCell::new(HeapCache::default());
        }

        HEAP_CACHE.with(|hc| {
            let mut hc = hc.borrow_mut();

            let HeapCache {
                ref mut update_heap_cache,
                ref mut hints_vec,
                ref mut rects_vec
            } = *hc;

            let grid_dims = GridSize::new(self.tabs.len() as u32 + 1, 2);

            self.layout_engine.desired_size = DimsBox::new(self.rect.dims());
            self.layout_engine.set_grid_size(grid_dims);
            self.layout_engine.set_row_hints(
                0,
                TrackHints {
                    fr_size: 0.0,
                    ..TrackHints::default()
                }
            );
            self.layout_engine.set_col_hints(
                grid_dims.x - 1,
                TrackHints {
                    fr_size: 1.0,
                    ..TrackHints::default()
                }
            );

            let mut active_tab_index_opt = None;
            for (index, tab) in self.tabs.iter_mut().enumerate() {
                match (active_tab_index_opt, tab.open) {
                    (None, true) => active_tab_index_opt = Some(index),
                    (Some(_), _) => tab.open = false,
                    _ => ()
                }

                hints_vec.push(WidgetPos {
                    size_bounds: SizeBounds::new_min(tab.title.min_size()),
                    widget_span: WidgetSpan::new(index as u32, 0),
                    ..WidgetPos::default()
                });
                rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
                self.layout_engine.set_col_hints(
                    index as u32,
                    TrackHints {
                        fr_size: 0.0,
                        ..TrackHints::default()
                    }
                );
            }

            let (active_tab, active_tab_index): (&TabPage<W>, usize);

            match (active_tab_index_opt, self.tabs.len()) {
                (Some(i), _) => {
                    active_tab = &self.tabs[i];
                    active_tab_index = i;
                },
                (None, 0) => {
                    hints_vec.clear();
                    rects_vec.clear();
                    return;
                },
                (None, _) => {
                    let tab = &mut self.tabs[0];
                    tab.open = true;
                    active_tab = tab;
                    active_tab_index = 0;
                }
            }

            hints_vec.push(WidgetPos {
                widget_span: WidgetSpan::new(.., 1),
                size_bounds: active_tab.page.size_bounds(),
                ..WidgetPos::default()
            });
            rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            let mut rects_iter = rects_vec.drain(..);
            for (tab, rect) in self.tabs.iter_mut().zip(&mut rects_iter) {
                tab.rect = rect.unwrap_or(BoundBox::new2(-1, -1, -1, -1));
            }
            let active_tab_mut = &mut self.tabs[active_tab_index];
            *active_tab_mut.page.rect_mut() = rects_iter.next().unwrap().unwrap_or(BoundBox::new2(-1, -1, -1, -1));

            for (_, tab) in self.tabs.iter_mut().enumerate().filter(|&(i, _)| i != active_tab_index) {
                *tab.page.rect_mut() = BoundBox::new2(-1, -1, -1, -1);
            }

            hints_vec.clear();
        })
    }
}
