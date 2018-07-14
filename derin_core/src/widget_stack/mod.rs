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

mod inner;

use LoopFlow;
use std::cmp::{Ordering, Ord};
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use render::RenderFrame;
use tree::{Widget, WidgetSummary, WidgetIdent, WidgetTag, RootID, WidgetID};
use tree::dynamic::ParentDyn;

use self::inner::{NRAllocCache, NRVec};
pub use self::inner::WidgetPath;

use cgmath::{Vector2, EuclideanSpace};
use cgmath_geometry::{BoundBox, GeoBox};
use offset_widget::{OffsetWidget, OffsetWidgetTrait, OffsetWidgetTraitAs};

pub(crate) struct WidgetStackBase<A, F: RenderFrame> {
    stack: NRAllocCache<A, F>
}

pub struct WidgetStack<'a, A: 'static, F: 'a + RenderFrame, Root: 'a + ?Sized> {
    stack: NRVec<'a, A, F>,
    root: *mut Root
}

impl<A: 'static, F: RenderFrame> WidgetStackBase<A, F> {
    pub fn new() -> WidgetStackBase<A, F> {
        WidgetStackBase {
            stack: NRAllocCache::new()
        }
    }

    // pub fn use_stack<'a, Root: Widget<A, F>>(&'a mut self, widget: &'a mut Root, root_id: RootID) -> WidgetStack<'a, A, F, Root> {
    //     WidgetStack {
    //         root: widget,
    //         stack: self.stack.use_cache(widget, root_id)
    //     }
    // }

    pub fn use_stack_dyn<'a>(&'a mut self, widget: &'a mut Widget<A, F>, root_id: RootID) -> WidgetStack<'a, A, F, Widget<A, F>> {
        WidgetStack {
            root: widget,
            stack: self.stack.use_cache(widget, root_id)
        }
    }
}

impl<'a, A, F: RenderFrame, Root: Widget<A, F> + ?Sized> WidgetStack<'a, A, F, Root> {
    pub fn drain_to_root<G>(&mut self, mut for_each: G) -> WidgetPath<Root>
        where G: FnMut(OffsetWidget<Widget<A, F>>, &[WidgetIdent])
    {
        self.drain_to_root_while(|widget, ident| {for_each(widget, ident); true}).unwrap()
    }

    pub fn drain_to_root_while<G>(&mut self, mut for_each: G) -> Option<WidgetPath<Root>>
        where G: FnMut(OffsetWidget<Widget<A, F>>, &[WidgetIdent]) -> bool
    {
        let mut continue_drain = true;
        while self.stack.len() > 1 && continue_drain {
            {
                let top = self.stack.top();
                continue_drain = for_each(top.widget, top.path);
            }
            self.stack.pop();
        }

        if !continue_drain {
            return None;
        }

        let top = self.stack.top();
        for_each(top.widget, top.path);

        let mut top = self.stack.top();
        assert_eq!(self.root as *mut () as usize, top.widget.inner_mut() as *mut _ as *mut () as usize);
        Some(WidgetPath {
            widget: OffsetWidget::new(
                unsafe{ &mut *self.root },
                Vector2::new(0, 0),
                Some(BoundBox::new2(0, 0, i32::max_value(), i32::max_value()))
            ),
            path: top.path
        })
    }

    #[inline]
    pub fn move_to_root(&mut self) -> WidgetPath<Root> {
        self.stack.truncate(1);

        let mut top = self.stack.top();
        assert_eq!(self.root as *mut () as usize, top.widget.inner_mut() as *mut _ as *mut () as usize);
        WidgetPath {
            widget: OffsetWidget::new(
                unsafe{ &mut *self.root },
                Vector2::new(0, 0),
                Some(BoundBox::new2(0, 0, i32::max_value(), i32::max_value()))
            ),
            path: top.path
        }
    }

    pub fn move_to_sibling_delta(&mut self, sibling_dist: isize) -> Result<WidgetPath<Widget<A, F>>, Ordering> {
        if sibling_dist == 0 {
            return Ok(self.stack.top());
        }

        let top_index = self.stack.top_index();
        let sibling_index = top_index as isize + sibling_dist;
        let left_cmp = sibling_index.cmp(&0);
        self.stack.pop().ok_or(left_cmp)?;

        let num_children = self.stack.top().widget.as_parent_mut().unwrap().num_children();
        let right_cmp = sibling_index.cmp(&(num_children as isize));

        match (left_cmp, right_cmp) {
            (Ordering::Greater, Ordering::Less) |
            (Ordering::Equal, Ordering::Less) => {
                let child = self.stack.try_push(|widget, _|
                    widget.as_parent_mut().unwrap().child_by_index_mut(sibling_index as usize)
                ).unwrap();
                Ok(WidgetPath {
                    widget: OffsetWidget::new(
                        child.widget,
                        self.stack.top_parent_offset(),
                        self.stack.clip_rect()
                    ),
                    path: self.stack.ident()
                })
            },
            _ => {
                self.stack.try_push(|widget, _|
                    widget.as_parent_mut().unwrap().child_by_index_mut(top_index)
                ).unwrap();
                Err(left_cmp)
            }
        }
    }

    pub fn move_to_sibling_index(&mut self, sibling_index: usize) -> Result<WidgetPath<Widget<A, F>>, Ordering> {
        let top_index = self.stack.top_index();
        if self.stack.pop().is_none() {
            return match sibling_index {
                0 => Ok(self.stack.top()),
                _ => Err(Ordering::Greater)
            };
        }
        let child = self.stack.try_push(|widget, _|
            widget.as_parent_mut().unwrap().child_by_index_mut(sibling_index)
        );
        match child {
            Some(child) => Ok(WidgetPath {
                widget: OffsetWidget::new(
                    child.widget,
                    self.stack.top_parent_offset(),
                    self.stack.clip_rect()
                ),
                path: self.stack.ident()
            }),
            None => {
                self.stack.try_push(|widget, _|
                    widget.as_parent_mut().unwrap().child_by_index_mut(top_index)
                ).unwrap();
                Err(Ordering::Greater)
            }
        }
    }

    #[inline]
    pub fn top(&mut self) -> WidgetPath<Widget<A, F>> {
        self.stack.top()
    }

    #[inline]
    pub fn top_ident(&self) -> WidgetIdent {
        self.stack.top_ident()
    }

    #[inline]
    pub fn ident(&self) -> &[WidgetIdent] {
        self.stack.ident()
    }

    pub fn parent(&self) -> Option<&ParentDyn<A, F>> {
        self.stack.widgets().rev().skip(1).next().map(|n| n.as_parent().unwrap())
    }

    #[inline]
    pub fn widgets<'b>(&'b self) -> impl 'b + Iterator<Item=&'a Widget<A, F>> + DoubleEndedIterator + ExactSizeIterator {
        self.stack.widgets()
    }

    pub fn try_push<G>(&mut self, with_top: G) -> Option<WidgetSummary<&'a mut Widget<A, F>>>
        where G: FnOnce(&'a mut Widget<A, F>, &[WidgetIdent]) -> Option<WidgetSummary<&'a mut Widget<A, F>>>
    {
        self.stack.try_push(with_top)
    }

    #[inline]
    pub fn pop(&mut self) -> Option<&'a mut Widget<A, F>> {
        self.stack.pop()
    }

    #[inline]
    pub fn depth(&self) -> usize {
        // The len is always going to be >= 1, so when it's 1 we're at the root widget (dpeth 0)
        self.stack.len() - 1
    }

    #[inline]
    #[must_use]
    pub(crate) fn move_to_widget(&mut self, widget_id: WidgetID) -> Option<WidgetPath<Widget<A, F>>> {
        self.move_to_root();

        let mut child_index = 0;
        while self.top().widget.widget_tag().widget_id() != widget_id {
            let new_widget = self.stack.try_push(|top_widget, _| {
                if let Some(top_widget_as_parent) = top_widget.as_parent_mut() {
                    return top_widget_as_parent.child_by_index_mut(child_index);
                }

                None
            });

            if let Some(ref widget_path) = new_widget {
                if widget_id == widget_path.widget.widget_tag().widget_id() {
                    return Some(self.top());
                }
            }

            match new_widget.is_some() {
                true => child_index = 0,
                false => {
                    child_index = self.stack.top_index() + 1;
                    if self.stack.pop().is_none() {
                        break
                    }
                }
            }
        }

        None
    }

    pub fn move_to_path<I>(&mut self, ident_path: I) -> Option<WidgetPath<Widget<A, F>>>
        where I: IntoIterator<Item=WidgetIdent>
    {
        let mut ident_path_iter = ident_path.into_iter().peekable();

        // Find the depth at which the given path and the current path diverge, and move the stack
        // to that depth.
        let mut diverge_depth = 0;
        {
            let mut active_path_iter = self.stack.ident().iter();
            // While the next item in the ident path and the active path are equal, increment the
            // diverge depth.
            while active_path_iter.next().and_then(|ident| ident_path_iter.peek().map(|i| i == ident)).unwrap_or(false) {
                diverge_depth += 1;
                ident_path_iter.next();
            }
        }
        if diverge_depth == 0 {
            return None;
        }
        self.stack.truncate(diverge_depth);

        let mut valid_path = true;
        for ident in ident_path_iter {
            valid_path = self.stack.try_push(|widget, _| {
                if let Some(widget_as_parent) = widget.as_parent_mut() {
                    widget_as_parent.child_mut(ident)
                } else {
                    None
                }
            }).is_some();

            if !valid_path {
                break;
            }
        }

        match valid_path {
            true => Some(self.stack.top()),
            false => None
        }
    }

    pub(crate) fn move_over_widgets<I, G>(&mut self, widget_ids: I, mut for_each_widget: G)
        where I: IntoIterator<Item=WidgetID> + Clone,
              G: FnMut(OffsetWidget<Widget<A, F>>, &[WidgetIdent], usize)
    {
        self.move_to_root();
        let mut widgets_left = widget_ids.clone().into_iter().count();
        let mut child_index = 0;
        while widgets_left > 0 {
            let top_parent_offset = self.stack.top_parent_offset();
            let clip_rect = self.stack.clip_rect();
            let valid_child = self.stack.try_push(|top_widget, top_path| {
                let top_id = top_widget.widget_tag().widget_id();

                if let Some((iter_index, wid)) = widget_ids.clone().into_iter().enumerate().find(|&(_, id)| id == top_id) {
                    for_each_widget(OffsetWidget::new(top_widget, top_parent_offset, clip_rect), top_path, iter_index);
                    widgets_left -= 1;
                }

                if let Some(top_widget_as_parent) = top_widget.as_parent_mut() {
                    return top_widget_as_parent.child_by_index_mut(child_index);
                }

                None
            }).is_some();

            match valid_child {
                true => child_index = 0,
                false => {
                    child_index = self.stack.top_index() + 1;
                    if self.stack.pop().is_none() {
                        break
                    }
                }
            }
        }
    }
}
