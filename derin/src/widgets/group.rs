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

use crate::{
    container::WidgetContainer,
    core::{
        LoopFlow,
        event::{EventOps, WidgetEventSourced, InputState},
        widget::{WidgetIdent, WidgetRender, WidgetTag, WidgetInfo, WidgetInfoMut, Widget, Parent},
        render::{RenderFrame, RenderFrameClipped},
    },
    gl_render::{ThemedPrim, PrimFrame, RelPoint, Prim},
    layout::GridLayout,
};

use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox, GeoBox}};
use derin_common_types::layout::{SizeBounds, WidgetPos};

use std::cell::RefCell;
use arrayvec::ArrayVec;

use derin_layout_engine::{GridEngine, UpdateHeapCache, SolveError};

/// A group of widgets.
///
/// Children of the group are specified by creating structs which implement [`WidgetContainer`].
/// You're encouraged to use the `derive` macro in `derin_macros` to do so.
#[derive(Debug, Clone)]
pub struct Group<C, L>
    where L: GridLayout
{
    widget_tag: WidgetTag,
    bounds: BoundBox<D2, i32>,
    layout_engine: GridEngine,
    container: C,
    layout: L
}

impl<C, L> Group<C, L>
    where L: GridLayout
{
    /// Create a new `Group` containing the widgets specified in `container`, with the layout
    /// specified in `layout`.
    pub fn new(container: C, layout: L) -> Group<C, L> {
        Group {
            widget_tag: WidgetTag::new(),
            bounds: BoundBox::new2(0, 0, 0, 0),
            layout_engine: GridEngine::new(),
            container, layout
        }
    }

    /// Retrieve the widgets contained within the group.
    pub fn container(&self) -> &C {
        &self.container
    }

    /// Retrieve the widgets contained within the group, for mutation.
    pub fn container_mut(&mut self) -> &mut C {
        &mut self.container
    }
}

impl<C, L> Widget for Group<C, L>
    where C: WidgetContainer<dyn Widget>,
          L: GridLayout
{
    #[inline]
    fn widget_tag(&self) -> &WidgetTag {
        &self.widget_tag
    }

    #[inline]
    fn rect(&self) -> BoundBox<D2, i32> {
        self.bounds
    }

    #[inline]
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        self.widget_tag.request_relayout();
        &mut self.bounds
    }
    fn size_bounds(&self) -> SizeBounds {
        self.layout_engine.actual_size_bounds()
    }

    #[inline]
    fn on_widget_event(&mut self, _: WidgetEventSourced, _: InputState) -> EventOps {
        // TODO: PASS FOCUS THROUGH SELF
        EventOps {
            focus: None,
            bubble: true,
        }
    }
}

impl<C, L> Parent for Group<C, L>
    where C: WidgetContainer<dyn Widget>,
          L: GridLayout
{
    fn num_children(&self) -> usize {
        self.container.num_children()
    }

    fn framed_child<F: RenderFrame>(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, F>> {
        self.container.framed_child(widget_ident).map(WidgetInfo::erase_subtype)
    }
    fn framed_child_mut<F: RenderFrame>(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, F>> {
        self.container.framed_child_mut(widget_ident).map(WidgetInfoMut::erase_subtype)
    }

    fn framed_children<'a, F, G>(&'a self, mut for_each: G)
        where F: RenderFrame,
              G: FnMut(WidgetInfo<'a, F>) -> LoopFlow
    {
        self.container.framed_children(|summary| for_each(WidgetInfo::erase_subtype(summary)))
    }

    fn framed_children_mut<'a, F, G>(&'a mut self, mut for_each: G)
        where F: RenderFrame,
              G: FnMut(WidgetInfoMut<'a, F>) -> LoopFlow
    {
        self.container.framed_children_mut(|summary| for_each(WidgetInfoMut::erase_subtype(summary)))
    }

    fn framed_child_by_index<F: RenderFrame>(&self, index: usize) -> Option<WidgetInfo<'_, F>> {
        self.container.framed_child_by_index(index).map(WidgetInfo::erase_subtype)
    }
    fn framed_child_by_index_mut<F: RenderFrame>(&mut self, index: usize) -> Option<WidgetInfoMut<'_, F>> {
        self.container.framed_child_by_index_mut(index).map(WidgetInfoMut::erase_subtype)
    }
}

impl<F, C, L> WidgetRender<F> for Group<C, L>
    where F: PrimFrame,
          C: WidgetContainer<dyn Widget>,
          L: GridLayout
{
    fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
        frame.upload_primitives(ArrayVec::from([
            ThemedPrim {
                theme_path: "Group",
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image,
                rect_px_out: None
            }
        ]).into_iter());
    }

    fn update_layout(&mut self, _: &F::Theme) {
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

            let num_children = self.num_children();
            self.container.children::<_>(|summary| {
                let widget_size_bounds = summary.widget().size_bounds();
                let mut layout_hints = self.layout.positions(summary.ident, summary.index, num_children).unwrap_or(WidgetPos::default());

                layout_hints.size_bounds = SizeBounds {
                    min: layout_hints.size_bounds.bound_rect(widget_size_bounds.min),
                    max: layout_hints.size_bounds.bound_rect(widget_size_bounds.max),
                };
                hints_vec.push(layout_hints);
                rects_vec.push(Ok(BoundBox::new2(0, 0, 0, 0)));
                LoopFlow::Continue
            });

            self.layout_engine.desired_size = DimsBox::new2(self.bounds.width(), self.bounds.height());
            self.layout_engine.set_grid_size(self.layout.grid_size(num_children));
            self.layout_engine.update_engine(hints_vec, rects_vec, update_heap_cache);

            let mut rects_iter = rects_vec.drain(..);
            self.container.children_mut::<_>(|mut summary| {
                match rects_iter.next() {
                    Some(rect) => *summary.widget_mut().rect_mut() = rect.unwrap_or(BoundBox::new2(0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF, 0xDEDBEEF)),
                    None => return LoopFlow::Break
                }
                LoopFlow::Continue
            });

            hints_vec.clear();
        })
    }
}
