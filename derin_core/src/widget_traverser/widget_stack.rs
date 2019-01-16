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
use crate::tree::{Widget, WidgetID, WidgetIdent, WidgetSummary, ROOT_IDENT};

use crate::cgmath::{Bounded, EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use crate::offset_widget::OffsetWidget;

// TODO: GET CODE REVIEWED FOR SAFETY

struct StackElement<F: RenderFrame> {
    widget: *mut (Widget<F>),
    rectangles: Option<ElementRects>,
    index: usize,
    widget_id: WidgetID
}

#[derive(Debug, Clone, Copy)]
struct ElementRects {
    bounds: BoundBox<D2, i32>,
    bounds_clipped: Option<BoundBox<D2, i32>>
}

pub(crate) struct WidgetStackCache<F: RenderFrame> {
    vec: Vec<StackElement<F>>,
    ident_vec: Vec<WidgetIdent>
}

pub(crate) struct WidgetStack<'a, F: 'a + RenderFrame> {
    vec: &'a mut Vec<StackElement<F>>,
    ident_vec: &'a mut Vec<WidgetIdent>,
    clip_rect: Option<BoundBox<D2, i32>>,
    top_parent_offset: Vector2<i32>,
}

pub(crate) type OffsetWidgetPath<'a, F> = WidgetPath<'a, OffsetWidget<'a, dyn Widget<F>>>;

pub(crate) struct WidgetPath<'a, W> {
    pub widget: W,
    pub path: &'a [WidgetIdent],
    pub index: usize,
    pub widget_id: WidgetID
}

impl<F: RenderFrame> WidgetStackCache<F> {
    pub fn new() -> WidgetStackCache<F> {
        WidgetStackCache {
            vec: Vec::new(),
            ident_vec: Vec::new(),
        }
    }

    pub fn use_cache<'a>(&'a mut self, widget: &mut Widget<F>) -> WidgetStack<'a, F> {
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

        WidgetStack {
            vec: &mut self.vec,
            ident_vec: &mut self.ident_vec,
            clip_rect: Some(BoundBox::new(Point2::new(0, 0), Point2::max_value())),
            top_parent_offset: Vector2::new(0, 0),
        }
    }
}

impl<'a, F: RenderFrame> WidgetStack<'a, F> {
    #[inline]
    pub fn top(&self) -> WidgetPath<'_, &'_ Widget<F>> {
        let (widget, widget_id) = self.vec.last().map(|n| unsafe{ (&*n.widget, n.widget_id) }).unwrap();
        WidgetPath {
            widget: widget,
            path: &self.ident_vec,
            index: self.top_index(),
            widget_id
        }
    }

    #[inline]
    pub fn top_mut(&mut self) -> OffsetWidgetPath<F> {
        let (widget, widget_id) = self.vec.last_mut().map(|n| unsafe{ (&mut *n.widget, n.widget_id) }).unwrap();
        OffsetWidgetPath {
            widget: OffsetWidget::new(widget, self.top_parent_offset, self.clip_rect),
            path: &self.ident_vec,
            index: self.top_index(),
            widget_id
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

    pub fn top_parent_offset(&self) -> Vector2<i32> {
        self.top_parent_offset
    }

    pub fn clip_rect(&self) -> Option<BoundBox<D2, i32>> {
        self.clip_rect
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        assert_ne!(0, len);

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
    pub fn widgets(&self) -> impl '_ + Iterator<Item=WidgetPath<'_, &'_ Widget<F>>> + DoubleEndedIterator + ExactSizeIterator {
        let path = &self.ident_vec[..];
        self.vec.iter().enumerate().map(move |(i, n)| WidgetPath {
            widget: unsafe{ &*n.widget },
            path: &path[..i],
            index: n.index,
            widget_id: n.widget_id,
        })
    }

    #[inline]
    pub fn path(&self) -> &[WidgetIdent] {
        debug_assert_eq!(self.ident_vec.len(), self.vec.len());
        &self.ident_vec
    }

    #[inline]
    pub fn try_push<G>(&mut self, with_top: G) -> Option<OffsetWidgetPath<'_, F>>
        where G: FnOnce(&'_ mut dyn Widget<F>) -> Option<WidgetSummary<&'_ mut Widget<F>>>
    {
        let mut old_top = self.top_mut();
        let new_top_opt = with_top(old_top.widget.inner_mut());
        if let Some(new_top_summary) = new_top_opt {
            let new_top_id = new_top_summary.widget.widget_tag().widget_id;
            let new_top_widget = new_top_summary.widget as *mut _;
            let new_top_index = new_top_summary.index;
            let new_top_ident = new_top_summary.ident.clone();

            assert_ne!(new_top_widget, self.top_mut().widget.inner_mut() as *mut Widget<F>);
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
                widget_id: new_top_id,
                widget: new_top_widget,
                rectangles: None,
                index: new_top_index
            });
            self.ident_vec.push(new_top_ident);
            Some(self.top_mut())
        } else {
            None
        }
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&mut Widget<F>> {
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

impl<'a, W> WidgetPath<'a, W> {
    pub fn map<X>(self, f: impl FnOnce(W) -> X) -> WidgetPath<'a, X> {
        WidgetPath {
            widget: f(self.widget),
            path: self.path,
            index: self.index,
            widget_id: self.widget_id,
        }
    }
}
