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

use crate::LoopFlow;
use std::cmp::{Ordering, Ord};
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use crate::render::RenderFrame;
use crate::tree::{Widget, WidgetSummary, WidgetIdent, ChildEventRecv, WidgetTag, RootID, WidgetID, ROOT_IDENT};
use crate::tree::dynamic::ParentDyn;

use self::inner::{NRAllocCache, NRVec};
pub use self::inner::WidgetPath;

use crate::cgmath::{Vector2, EuclideanSpace};
use cgmath_geometry::{D2, rect::{BoundBox, GeoBox}};
use crate::offset_widget::{OffsetWidget, OffsetWidgetTrait, OffsetWidgetTraitAs};

pub(crate) struct WidgetStackBase<A, F: RenderFrame> {
    stack: NRAllocCache<A, F>
}

pub struct WidgetStack<'a, A: 'static, F: 'a + RenderFrame, Root: 'a + ?Sized = Widget<A, F>> {
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
    #[inline]
    pub fn top(&mut self) -> WidgetPath<A, F> {
        self.stack.top()
    }

    #[inline]
    pub fn top_ident(&self) -> WidgetIdent {
        self.stack.top_ident()
    }

    #[inline]
    pub fn path(&self) -> &[WidgetIdent] {
        self.stack.path()
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

    pub fn move_to_path<I>(&mut self, ident_path: I) -> Option<WidgetPath<A, F>>
        where I: IntoIterator<Item=WidgetIdent>
    {
        let mut ident_path_iter = ident_path.into_iter().peekable();

        // Find the depth at which the given path and the current path diverge, and move the stack
        // to that depth.
        let mut diverge_depth = 0;
        {
            let mut active_path_iter = self.stack.path().iter();
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

    pub(crate) fn search_for_widget(&mut self, widget_id: WidgetID) -> Option<WidgetPath<A, F>> {
        self.move_to_path(Some(ROOT_IDENT));
        let mut widget_found = false;
        let mut child_index = 0;
        loop {
            let top_parent_offset = self.stack.top_parent_offset();
            let clip_rect = self.stack.clip_rect();
            let valid_child = self.stack.try_push(|top_widget, top_path| {
                let top_id = top_widget.widget_tag().widget_id;

                if widget_id == top_id {
                    widget_found = true;
                    return None;
                }

                if let Some(top_widget_as_parent) = top_widget.as_parent_mut() {
                    return top_widget_as_parent.child_by_index_mut(child_index);
                }

                None
            }).is_some();

            if widget_found {
                break Some(self.top());
            }

            match valid_child {
                true => child_index = 0,
                false => {
                    child_index = self.stack.top_index() + 1;
                    if self.stack.pop().is_none() {
                        break None
                    }
                }
            }
        }
    }
}
