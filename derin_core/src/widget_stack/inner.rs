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

use std::mem;
use crate::render::RenderFrame;
use crate::tree::{Widget, WidgetID, WidgetIdent, WidgetSummary, RootID, Update, ROOT_IDENT};

use crate::cgmath::{Bounded, EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use crate::offset_widget::{OffsetWidget, OffsetWidgetTrait};

// TODO: GET CODE REVIEWED FOR SAFETY

struct StackElement<A, F: RenderFrame> {
    widget: *mut (Widget<A, F>),
    rectangles: Option<ElementRects>,
    index: usize,
    widget_id: WidgetID
}

#[derive(Debug, Clone, Copy)]
struct ElementRects {
    bounds: BoundBox<D2, i32>,
    bounds_clipped: Option<BoundBox<D2, i32>>
}

pub(crate) struct NRAllocCache<A, F: RenderFrame> {
    vec: Vec<StackElement<A, F>>,
    ident_vec: Vec<WidgetIdent>
}

pub struct NRVec<'a, A: 'static, F: 'a + RenderFrame> {
    vec: &'a mut Vec<StackElement<A, F>>,
    ident_vec: &'a mut Vec<WidgetIdent>,
    clip_rect: Option<BoundBox<D2, i32>>,
    top_parent_offset: Vector2<i32>,
}

pub struct WidgetPath<'a, A: 'static, F: 'a + RenderFrame> {
    pub widget: OffsetWidget<'a, dyn Widget<A, F>>,
    pub path: &'a [WidgetIdent],
    pub index: usize
}

impl<A, F: RenderFrame> NRAllocCache<A, F> {
    pub fn new() -> NRAllocCache<A, F> {
        NRAllocCache {
            vec: Vec::new(),
            ident_vec: Vec::new(),
        }
    }

    pub fn use_cache<'a>(&'a mut self, widget: &mut (Widget<A, F> + 'a)) -> NRVec<'a, A, F> {
        let mut cache_swap = Vec::new();
        mem::swap(&mut cache_swap, &mut self.vec);

        self.vec.clear();
        self.ident_vec.clear();

        self.vec.push(StackElement {
            widget_id: widget.widget_tag().widget_id,
            widget,
            rectangles: None,
            index: 0,
        });
        self.ident_vec.push(ROOT_IDENT);

        NRVec {
            vec: &mut self.vec,
            ident_vec: &mut self.ident_vec,
            clip_rect: Some(BoundBox::new(Point2::new(0, 0), Point2::max_value())),
            top_parent_offset: Vector2::new(0, 0),
        }
    }
}

impl<'a, A: 'static, F: RenderFrame> NRVec<'a, A, F> {
    #[inline]
    pub fn top(&mut self) -> WidgetPath<A, F> {
        let widget = self.vec.last_mut().map(|n| unsafe{ &mut *n.widget }).unwrap();
        WidgetPath {
            widget: OffsetWidget::new(widget, self.top_parent_offset, self.clip_rect),
            path: &self.ident_vec,
            index: self.top_index()
        }
    }

    #[inline]
    pub fn top_ident(&self) -> WidgetIdent {
        self.ident_vec.last().cloned().unwrap()
    }

    #[inline]
    pub fn top_index(&self) -> usize {
        self.vec.last().unwrap().index
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        assert_ne!(0, len);
        for widget_slice in self.vec[len-1..].windows(2).rev() {
            let parent = unsafe{ &*widget_slice[0].widget };
            let child = unsafe{ &*widget_slice[1].widget };
        }

        self.vec.truncate(len);
        self.ident_vec.truncate(len);

        match self.vec.get(self.vec.len().wrapping_sub(2)).map(|e| e.rectangles.expect("Bad widget bounds stack")) {
            None => {
                self.top_parent_offset = Vector2::new(0, 0);
                self.clip_rect = Some(BoundBox::new(Point2::new(0, 0), Point2::max_value()));
            },
            Some(rectangles) => {
                self.top_parent_offset = rectangles.bounds.min().to_vec();
                self.clip_rect = rectangles.bounds_clipped;
            }
        }
    }

    #[inline]
    pub fn widgets<'b>(&'b self) -> impl 'b + Iterator<Item=&'a Widget<A, F>> + DoubleEndedIterator + ExactSizeIterator {
        self.vec.iter().map(|n| unsafe{ &*n.widget })
    }

    #[inline]
    pub fn path(&self) -> &[WidgetIdent] {
        debug_assert_eq!(self.ident_vec.len(), self.vec.len());
        &self.ident_vec
    }

    #[inline]
    pub fn try_push<'b, G>(&'b mut self, with_top: G) -> Option<WidgetSummary<&'b mut Widget<A, F>>>
        where G: FnOnce(&'b mut Widget<A, F>, &[WidgetIdent]) -> Option<WidgetSummary<&'b mut Widget<A, F>>>
    {
        let new_top_opt = with_top(unsafe{ mem::transmute(self.top().widget.inner_mut()) }, &self.ident_vec );
        if let Some(new_top_summary) = new_top_opt {
            assert_ne!(new_top_summary.widget as *mut Widget<A, F>, self.top().widget.inner_mut() as *mut Widget<A, F>);
            {
                let cur_top = self.vec.last_mut().unwrap();

                let top_rect = unsafe{ &*cur_top.widget }.rect() + self.top_parent_offset;
                let top_clip = self.clip_rect.and_then(|r| r.intersect_rect(top_rect));
                cur_top.rectangles = Some(ElementRects {
                    bounds: top_rect,
                    bounds_clipped: top_clip
                });
                self.clip_rect = top_clip;

                self.top_parent_offset = top_rect.min().to_vec();
            }

            self.vec.push(StackElement {
                widget_id: new_top_summary.widget.widget_tag().widget_id,
                widget: new_top_summary.widget,
                rectangles: None,
                index: new_top_summary.index
            });
            self.ident_vec.push(new_top_summary.ident.clone());
            Some(new_top_summary)
        } else {
            None
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&'a mut Widget<A, F>> {
        // Ensure the base is never popped
        if self.vec.len() == 1 {
            return None;
        }

        let popped = self.vec.pop().map(|n| unsafe{ &mut *n.widget }).unwrap();
        self.ident_vec.pop();
        let last_mut = self.vec.last_mut().unwrap();
        last_mut.rectangles = None;
        match self.vec.get(self.vec.len().wrapping_sub(2)).map(|e| e.rectangles.expect("Bad widget bounds stack")) {
            None => {
                self.top_parent_offset = Vector2::new(0, 0);
                self.clip_rect = Some(BoundBox::new(Point2::new(0, 0), Point2::max_value()));
            },
            Some(rectangles) => {
                self.top_parent_offset = rectangles.bounds.min().to_vec();
                self.clip_rect = rectangles.bounds_clipped;
            }
        }

        Some(popped)
    }
}
