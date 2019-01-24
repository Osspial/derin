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
use crate::widget::{Widget, WidgetID, WidgetIdent, WidgetSummary, ROOT_IDENT};
use super::virtual_widget_tree::PathRevItem;

use crate::cgmath::{Bounded, EuclideanSpace, Point2, Vector2};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};

use crate::offset_widget::{OffsetWidget, OffsetWidgetTrait};

// TODO: GET CODE REVIEWED FOR SAFETY

struct StackElement<F: RenderFrame> {
    widget: *mut (Widget<F>),
    rectangles: Option<ElementRects>,
    index: usize,
    widget_id: WidgetID
}

impl<F: RenderFrame> std::fmt::Debug for StackElement<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("StackElement")
            .field("rectangles", &self.rectangles)
            .field("index", &self.index)
            .field("widget_id", &self.widget_id)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            widget_id: widget.widget_id(),
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

    // #[inline]
    // pub fn top_ident(&self) -> WidgetIdent {
    //     self.ident_vec.last().cloned().unwrap()
    // }

    #[inline]
    pub fn top_index(&self) -> usize {
        self.vec.last().unwrap().index
    }

    #[inline]
    pub fn top_id(&self) -> WidgetID {
        self.vec.last().unwrap().widget_id
    }

    // pub fn top_parent_offset(&self) -> Vector2<i32> {
    //     self.top_parent_offset
    // }

    // pub fn clip_rect(&self) -> Option<BoundBox<D2, i32>> {
    //     self.clip_rect
    // }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn truncate(&mut self, len: usize) {
        assert_ne!(0, len);

        self.vec.truncate(len);
        self.vec.last_mut().unwrap().rectangles = None;
        self.ident_vec.truncate(len);
        self.truncate_offset_and_clip(len);
    }

    fn truncate_offset_and_clip(&mut self, len: usize) {
        match self.vec.get(len.wrapping_sub(2)).map(|e| e.rectangles.expect("Bad widget bounds stack")) {
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
        let top_rect = old_top.widget.rect();

        let new_top_opt = with_top(old_top.widget.inner_mut());

        if let Some(new_top_summary) = new_top_opt {
            let new_top_id = new_top_summary.widget.widget_id();
            let new_top_widget = new_top_summary.widget as *mut _;
            let new_top_index = new_top_summary.index;
            let new_top_ident = new_top_summary.ident.clone();

            assert_ne!(new_top_widget, self.top_mut().widget.inner_mut() as *mut Widget<F>);
            {
                let old_top = self.vec.last_mut().unwrap();
                let top_clip = self.clip_rect.and_then(|r| r.intersect_rect(top_rect));

                old_top.rectangles = Some(ElementRects {
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

    pub fn move_to_path_rev(&mut self, path_rev: impl Iterator<Item=PathRevItem> + ExactSizeIterator) -> Option<OffsetWidgetPath<'_, F>> {
        let new_path_len = path_rev.len();
        self.ident_vec.resize(new_path_len, WidgetIdent::Num(0));

        let mut path_rev = path_rev.peekable();
        let target_widget_id = path_rev.peek().expect("Requires path with len >= 1").id;

        let mut diverge_index = 0;
        for (i, path_item) in path_rev.enumerate().map(|(i, item)| (new_path_len - (i + 1), item)) {
            if Some(path_item.id) == self.vec.get(i).map(|e| e.widget_id) {
                diverge_index = i + 1;
                break;
            }

            self.ident_vec[i] = path_item.ident;
        }

        self.vec.truncate(diverge_index);
        self.vec.last_mut().unwrap().rectangles = None;
        self.truncate_offset_and_clip(diverge_index);

        let new_widget = try {
            while self.vec.len() < self.ident_vec.len() {
                let i = self.vec.len() - 1;
                let top = &mut self.vec[i];
                let top_widget = unsafe{ &mut *top.widget };

                {
                    let top_rect = top_widget.rect() + self.top_parent_offset;
                    let top_clip = self.clip_rect.and_then(|r| r.intersect_rect(top_rect));

                    top.rectangles = Some(ElementRects {
                        bounds: top_rect,
                        bounds_clipped: top_clip
                    });
                    self.clip_rect = top_clip;
                    self.top_parent_offset = top_rect.min().to_vec();
                }

                let new_top = top_widget
                    .as_parent_mut()?
                    .child_mut(self.ident_vec[i + 1].clone())?;
                self.vec.push(StackElement {
                    widget_id: new_top.widget.widget_id(),
                    widget: new_top.widget as *mut _,
                    rectangles: None,
                    index: new_top.index,
                });
            }
            assert_eq!(self.vec.len(), self.ident_vec.len());

            Some(())
        };

        match new_widget {
            Some(_) => {
                Some(self.top_mut())
            },
            None => {
                self.vec.last_mut().unwrap().rectangles = None;
                self.truncate(self.vec.len());
                None
            }
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
