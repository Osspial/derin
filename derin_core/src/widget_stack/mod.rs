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
    LoopFlow,
    render::RenderFrame,
    tree::{
        Widget, WidgetIdent, WidgetID, ROOT_IDENT,
        dynamic::ParentDyn
    },
    offset_widget::OffsetWidget,
    virtual_widget_tree::VirtualWidgetTree
};
use std::iter::{DoubleEndedIterator, ExactSizeIterator};

use self::inner::{NRAllocCache, NRVec};
pub(crate) use self::inner::WidgetPath;

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
        // The len is always going to be >= 1, so when it's 1 we're at the root widget (depth 0)
        self.stack.len() - 1
    }

    pub fn move_to_widget_with_tree(&mut self, widget_id: WidgetID, widget_tree: &mut VirtualWidgetTree) -> Option<WidgetPath<A, F>> {
        // TODO: GET RID OF COLLECT ALLOCATION
        let active_ident_rev_opt = widget_tree.ident_chain_reversed(widget_id).map(|c| c.cloned().collect::<Vec<_>>());

        if let Some(mut active_ident_rev) = active_ident_rev_opt {
            // If the widget is where the tree says it is, return it.
            if self.move_to_path(active_ident_rev.drain(..).rev()).is_some() && self.top().widget_id == widget_id {
                return Some(self.top());
            }
        }

        // If the widget wasn't found where the tree said it would be found, update the tree
        // and store the new location.
        let (widget_index, widget_ident);
        match self.scan_for_widget(widget_id) {
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

        assert_eq!(self.top().widget_id, widget_id);
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
            valid_path = self.stack.try_push(|widget| {
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

    pub(crate) fn scan_for_widget(&mut self, widget_id: WidgetID) -> Option<WidgetPath<A, F>> {
        self.move_to_path(Some(ROOT_IDENT));
        let mut widget_found = false;
        let mut child_index = 0;
        loop {
            let valid_child = self.stack.try_push(|top_widget| {
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

    /// Scan the tree for the leftmost deepest widget that returns `true`, and has all its parents
    /// return `true`, for the provided function.
    pub(crate) fn scan_for_widget_with(&mut self, mut f: impl FnMut(&OffsetWidget<'_, dyn Widget<A, F>>) -> bool) -> Option<WidgetPath<'_, A, F>> {
        if !f(&self.move_to_path(Some(ROOT_IDENT)).unwrap().widget) {
            return None;
        }
        let mut deepest_found: bool;

        loop {
            let top_parent_offset = self.stack.top_parent_offset();
            let clip_rect = self.stack.clip_rect();

            deepest_found = self.stack.try_push(|top_widget| {
                match top_widget.as_parent_mut() {
                    None => None,
                    Some(mut top_widget_as_parent) => {
                        let mut valid_index = None;
                        top_widget_as_parent.children_mut(&mut |mut child_summary_list| {
                            for child_summary in child_summary_list.drain(..) {
                                if f(&OffsetWidget::new(child_summary.widget, top_parent_offset, clip_rect)) {
                                    valid_index = Some(child_summary.index);
                                    return LoopFlow::Break(());
                                }
                            }
                            LoopFlow::Continue
                        });

                        // For some reason this works but Option::map doesn't.
                        match valid_index {
                            Some(i) => Some(
                                top_widget_as_parent
                                    .child_by_index_mut(i)
                                    .expect("widget children changed at wrong time")
                            ),
                            None => None
                        }
                    }
                }
            }).is_none();

            if deepest_found {
                break Some(self.top());
            }
        }
    }
}
