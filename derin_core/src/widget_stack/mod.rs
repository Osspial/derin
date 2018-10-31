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

use crate::{
    render::RenderFrame,
    tree::{
        Widget, WidgetIdent, WidgetID, ROOT_IDENT,
        dynamic::ParentDyn
    },
    widget_tree::WidgetTree
};
use std::iter::{DoubleEndedIterator, ExactSizeIterator};

use self::inner::{NRAllocCache, NRVec};
pub use self::inner::WidgetPath;

use crate::offset_widget::{OffsetWidgetTrait, OffsetWidgetTraitAs};

pub(crate) struct WidgetStackBase<A, F: RenderFrame> {
    stack: NRAllocCache<A, F>
}

pub(crate) struct WidgetStack<'a, A: 'static, F: 'a + RenderFrame> {
    stack: NRVec<'a, A, F>,
}

impl<A: 'static, F: RenderFrame> WidgetStackBase<A, F> {
    pub fn new() -> WidgetStackBase<A, F> {
        WidgetStackBase {
            stack: NRAllocCache::new()
        }
    }

    pub fn use_stack_dyn<'a>(&'a mut self, widget: &'a mut Widget<A, F>) -> WidgetStack<'a, A, F> {
        WidgetStack {
            stack: self.stack.use_cache(widget)
        }
    }
}

impl<'a, A, F: RenderFrame> WidgetStack<'a, A, F> {
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

    #[inline]
    pub fn pop(&mut self) -> Option<&'a mut Widget<A, F>> {
        self.stack.pop()
    }

    #[inline]
    pub fn depth(&self) -> usize {
        // The len is always going to be >= 1, so when it's 1 we're at the root widget (dpeth 0)
        self.stack.len() - 1
    }

    pub fn move_to_widget_with_tree(&mut self, widget_id: WidgetID, widget_tree: &mut WidgetTree) -> Option<WidgetPath<A, F>> {
        // TODO: GET RID OF COLLECT ALLOCATION
        let mut active_ident_rev = widget_tree.ident_chain_reversed(widget_id)?.cloned().collect::<Vec<_>>();

        // If the widget is where the tree says it is, return it.
        if self.move_to_path(active_ident_rev.drain(..).rev()).is_some() {
            return Some(self.top());
        }

        // If the widget wasn't found where the tree said it would be found, update the tree
        // and store the new location.
        let (widget_index, widget_ident);
        match self.search_for_widget(widget_id) {
            Some(widget) => {
                widget_index = widget.index;
                widget_ident = widget.path.last().unwrap().clone();
            },
            // If the widget isn't anywhere to be found, return none.
            None => {
                widget_tree.remove(widget_id);
                return None
            }
        }

        if let Some(parent) = self.parent() {
            widget_tree.insert(parent.widget_tag().widget_id, widget_id, widget_index, widget_ident).ok();
        }

        Some(self.top())
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
