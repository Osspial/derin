// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.


use crate::{
    core::{
        LoopFlow,
        event::{EventOps, WidgetEvent, WidgetEventSourced, InputState},
        widget::{WidgetIdent, WidgetRenderable, WidgetTag, WidgetInfo, WidgetInfoMut, Widget, Parent},
        render::Renderer,
    },
    gl_render::{RelPoint, ThemedPrim, Prim, PrimFrame, RenderString},
    widgets::assistants::ButtonState,
};

use derin_common_types::layout::{SizeBounds, WidgetPos, GridSize, WidgetSpan, TrackHints};

use std::cell::RefCell;
use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use derin_layout_engine::{GridEngine, UpdateHeapCache, SolveError};

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
    button_state: ButtonState,
    rect: BoundBox<D2, i32>
}

/// A list of tabs.
///
/// This widget lets you display a single widget at a time, from a greater selection of widgets.
/// Users can switch between these widgets by clicking on a list of tabs at the top of the widget.
#[derive(Debug, Clone)]
pub struct TabList<W> {
    widget_tag: WidgetTag,
    rect: BoundBox<D2, i32>,
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
            button_state: ButtonState::Normal,
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
            widget_tag: WidgetTag::new(),
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
        self.widget_tag.request_relayout().request_redraw();
        &mut self.tabs
    }
}

impl<W> Widget for TabList<W>
    where W: Widget
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.rect
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        self.widget_tag.request_relayout().request_redraw();
        &mut self.rect
    }

    #[inline]
    fn size_bounds(&self) -> SizeBounds {
        self.layout_engine.actual_size_bounds()
    }

    #[inline]
    fn on_widget_event(&mut self, event: WidgetEventSourced, _: InputState) -> EventOps {
        // TODO: PASS FOCUS TO CHILD
        if let WidgetEventSourced::This(ref event) = event {
            match event {
                WidgetEvent::MouseMove{new_pos, in_widget: true, ..} => {
                    let mut state_changed = false;
                    for tab in self.tabs.iter_mut() {
                        let new_state = match (tab.button_state, tab.rect.contains(*new_pos)) {
                            (ButtonState::Pressed, _) => ButtonState::Pressed,
                            (_, true) => ButtonState::Hover,
                            (_, false) => ButtonState::Normal,
                        };
                        state_changed |= new_state != tab.button_state;
                        tab.button_state = new_state;
                    }
                    if state_changed {
                        self.widget_tag.request_redraw();
                    }
                },
                // WidgetEvent::MouseEnterChild{..} => {
                //     let mut state_changed = false;
                //     for tab in self.tabs.iter_mut() {
                //         let new_state = match tab.button_state {
                //             ButtonState::Pressed => ButtonState::Pressed,
                //             _ => ButtonState::Normal,
                //         };
                //         state_changed |= new_state != tab.button_state;
                //         tab.button_state = new_state;
                //     }
                //     if state_changed {
                //         self.widget_tag.request_redraw();
                //     }
                // }
                WidgetEvent::MouseDown{pos, in_widget: true, ..} => {
                    let mut state_changed = false;
                    for tab in self.tabs.iter_mut() {
                        let new_state = match tab.rect.contains(*pos) {
                            true => ButtonState::Pressed,
                            false => tab.button_state
                        };
                        state_changed |= new_state != tab.button_state;
                        tab.button_state = new_state;
                    }
                    if state_changed {
                        self.widget_tag.request_redraw();
                    }
                },
                WidgetEvent::MouseUp{in_widget: true, pressed_in_widget: true, pos, down_pos, ..} => {
                    // Change tab selection.
                    let mut state_changed = false;
                    let (mut old_open, mut new_open) = (None, None);
                    for (index, tab) in self.tabs.iter_mut().enumerate() {
                        let tab_contains = tab.rect.contains(*pos);
                        let is_open = tab_contains && tab.rect.contains(*down_pos);
                        let new_state = match tab_contains {
                            true => ButtonState::Hover,
                            false => ButtonState::Normal
                        };
                        state_changed |= is_open != tab.open || new_state != tab.button_state;

                        if tab.open && old_open == None {
                            old_open = Some(index);
                        }
                        if is_open && new_open == None {
                            new_open = Some(index);
                        }

                        tab.open = is_open;
                        tab.button_state = new_state;
                    }
                    if state_changed {
                        self.widget_tag.request_redraw();
                    }
                    if !(old_open == new_open || new_open == None) {
                        self.widget_tag.request_relayout();
                    }
                },
                WidgetEvent::MouseUp{in_widget: false, pressed_in_widget: true, ..} => {
                    let mut state_changed = false;
                    for tab in self.tabs.iter_mut() {
                        let new_state = ButtonState::Normal;
                        state_changed |= new_state != tab.button_state;
                        tab.button_state = new_state;
                    }
                    if state_changed {
                        self.widget_tag.request_redraw();
                    }
                },
                _ => ()
            }
        }

        EventOps {
            focus: None,
            bubble: event.default_bubble() || event.is_bubble(),
        }
    }
}

impl<W> Parent for TabList<W>
    where W: Widget
{
    fn num_children(&self) -> usize {
        self.tabs.len()
    }

    fn framed_child<R: Renderer>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, R>> {
        if let WidgetIdent::Num(index) = widget_ident {
            self.tabs.get(index as usize).map(|t| WidgetInfo::new(widget_ident, index as usize, &t.page))
        } else {
            None
        }
    }
    fn framed_child_mut<R: Renderer>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, R>> {
        if let WidgetIdent::Num(index) = widget_ident {
            self.tabs.get_mut(index as usize).map(|t| WidgetInfoMut::new(widget_ident, index as usize, &mut t.page))
        } else {
            None
        }
    }

    fn framed_children<'a, R, G>(&'a self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfo<'a, R>) -> LoopFlow
    {
        for (index, tab) in self.tabs.iter().enumerate() {
            match for_each(WidgetInfo::new(WidgetIdent::Num(index as u32), index, &tab.page)) {
                LoopFlow::Continue => (),
                LoopFlow::Break => return
            }
        }
    }

    fn framed_children_mut<'a, R, G>(&'a mut self, mut for_each: G)
        where R: Renderer,
              G: FnMut(WidgetInfoMut<'a, R>) -> LoopFlow
    {
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            match for_each(WidgetInfoMut::new(WidgetIdent::Num(index as u32), index, &mut tab.page)) {
                LoopFlow::Continue => (),
                LoopFlow::Break => return
            }
        }
    }

    fn framed_child_by_index<R: Renderer>(&self, index: usize) -> Option<WidgetInfo<'_, R>> {
        self.tabs.get(index).map(|t| WidgetInfo::new(WidgetIdent::Num(index as u32), index, &t.page))
    }
    fn framed_child_by_index_mut<R: Renderer>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, R>> {
        self.tabs.get_mut(index).map(|t| WidgetInfoMut::new(WidgetIdent::Num(index as u32), index, &mut t.page))
    }
}

impl<R, W> WidgetRenderable<R> for TabList<W>
    where R: Renderer,
          W: Widget
{
    fn render(&mut self, frame: &mut R::SubFrame) {
        for tab in &mut self.tabs {
            let theme_path = match tab.button_state {
                ButtonState::Normal => "Tab::Normal",
                ButtonState::Hover => "Tab::Hover",
                ButtonState::Pressed => "Tab::Pressed"
            };
            frame.upload_primitives(ArrayVec::from([
                ThemedPrim {
                    theme_path,
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
                    theme_path,
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

    fn update_layout(&mut self, _: &R::Theme) {
        #[derive(Default)]
        struct HeapCache {
            update_heap_cache: UpdateHeapCache,
            hints_vec: Vec<WidgetPos>,
            rects_vec: Vec<Result<BoundBox<D2, i32>, SolveError>>
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

            self.layout_engine.desired_size = self.rect.dims();
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
