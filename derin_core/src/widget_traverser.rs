// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod widget_stack;
mod virtual_widget_tree;
mod offset_widget_scan;

pub(crate) use self::{
    offset_widget_scan::OffsetWidgetScan,
    widget_stack::{WidgetPath, OffsetWidgetPath},
};
use crate::{
    render::DisplayEngine,
    widget::{WidgetDyn, WidgetId, WidgetIdent},
    update_state::UpdateStateCell,
};
use std::rc::Rc;
use self::{
    widget_stack::{WidgetStack, WidgetStackCache},
    virtual_widget_tree::{WidgetInsertError, VirtualWidgetTree}
};

pub(crate) type OffsetWidgetScanPath<'a, D> = WidgetPath<'a, OffsetWidgetScan<'a, D>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    Parent,
    /// Sibling with a widget delta. // TODO EXPLAIN MORE
    Sibling(isize),
    ChildIdent(WidgetIdent),
    ChildIndex(usize)
}

pub(crate) struct WidgetTraverserBase<D>
    where for<'d> D: DisplayEngine<'d>
{
    stack_cache: WidgetStackCache<D>,
    virtual_widget_tree: VirtualWidgetTree,
}

pub(crate) struct WidgetTraverser<'a, D>
    where for<'d> D: DisplayEngine<'d>
{
    stack: WidgetStack<'a, D>,
    virtual_widget_tree: &'a mut VirtualWidgetTree,
    update_state: Rc<UpdateStateCell>,
}

impl<D> WidgetTraverserBase<D>
    where for<'d> D: DisplayEngine<'d>
{
    pub fn new(root_id: WidgetId) -> Self {
        WidgetTraverserBase {
            stack_cache: WidgetStackCache::new(),
            virtual_widget_tree: VirtualWidgetTree::new(root_id),
        }
    }

    pub fn with_root_ref<'a>(&'a mut self, root: &'a mut dyn WidgetDyn<D>, update_state: Rc<UpdateStateCell>) -> WidgetTraverser<'a, D> {
        // This isn't a necessary limitation with the code, but the current code assumes this assertion
        // holds.
        assert_eq!(root.widget_id(), self.virtual_widget_tree.root_id());

        WidgetTraverser {
            stack: self.stack_cache.use_cache(root),
            virtual_widget_tree: &mut self.virtual_widget_tree,
            update_state
        }
    }
}

impl<D> WidgetTraverser<'_, D>
    where for<'d> D: DisplayEngine<'d>
{
    pub fn get_widget(&mut self, id: WidgetId) -> Option<OffsetWidgetScanPath<'_, D>> {
        // Move the stack top to the desired widget.
        match self.get_widget_with_tree(id) {
            Some(_) => (),
            None => {
                self.scan_for_widget(id)?;
                self.add_stack_top_to_widget_tree();
            }
        };

        let WidgetTraverser {
            ref mut stack,
            ref mut virtual_widget_tree,
            ref update_state,
        } = self;

        Some(stack.top_mut().map(move |w| OffsetWidgetScan::new(w, virtual_widget_tree, update_state)))
    }

    pub fn get_widget_relation(&mut self, id: WidgetId, relation: Relation) -> Option<OffsetWidgetScanPath<'_, D>> {
        let relation_id = match relation {
            Relation::Parent => {
                self.virtual_widget_tree.parent(id).ok()?
            },
            Relation::Sibling(delta) => {
                self.virtual_widget_tree.sibling(id, delta).ok()?
            },
            Relation::ChildIdent(ident) => {
                self.virtual_widget_tree.child_ident(id, ident).ok()?
            },
            Relation::ChildIndex(index) => {
                self.virtual_widget_tree.child_index(id, index).ok()?
            },
        };

        self.get_widget(relation_id)
    }

    fn get_widget_with_tree(&mut self, id: WidgetId) -> Option<OffsetWidgetPath<'_, D>> {
        self.stack.move_to_path_rev(self.virtual_widget_tree.path_reversed(id)?)
    }

    pub fn remove_widget(&mut self, id: WidgetId) {
        self.virtual_widget_tree.remove(id);
    }

    /// Sorts the widgets in the provided slice by depth. Returns the sorted slice with all
    /// widgets not in the tree truncated.
    ///
    /// Widgets not in the tree are moved to the back of the `widgets` array.
    ///
    /// If multiple instances of the same Widget ID are in the array, they are placed next to
    /// each other.
    #[must_use]
    pub fn sort_widgets_by_depth<'a>(&mut self, widgets: &'a mut [WidgetId]) -> &'a mut [WidgetId] {
        widgets.sort_unstable_by_key(|id| {
            (
                self.virtual_widget_tree
                    .get_widget(*id)
                    .map(|data| data.depth())
                    .unwrap_or(u32::max_value()),
                id.to_u32()
            )
        });

        // Truncate all widgets that aren't in the tree.
        let mut valid_widgets = widgets.len();
        for id in widgets.iter().rev().cloned() {
            if self.virtual_widget_tree.get_widget(id).is_some() {
                break;
            }
            valid_widgets -= 1;
        }

        &mut widgets[..valid_widgets]
    }

    /// Crawl over all widgets in the tree. Any operations performed on the widget *should not*
    /// modify the structure of the child widgets.
    pub fn crawl_widgets(&mut self, mut for_each: impl FnMut(OffsetWidgetPath<'_, D>)) {
        let stack = &mut self.stack;

        stack.truncate(1);
        for_each(stack.top_mut());

        let mut child_index = 0;
        loop {
            let child_opt = stack.try_push(|top_widget| {
                top_widget.child_by_index_mut(child_index)
            });


            match child_opt {
                Some(child) => {
                    for_each(child);
                    child_index = 0;
                },
                None => {
                    child_index = stack.top_index() + 1;
                    if stack.pop().is_none() {
                        break
                    }
                }
            }
        }
    }

    pub fn crawl_widget_children(&mut self, parent: WidgetId, mut for_each: impl FnMut(OffsetWidgetPath<'_, D>)) {
        if let None = self.get_widget_with_tree(parent) {
            return;
        }

        let stack = &mut self.stack;
        let mut child_index = 0;
        loop {
            let child_opt = stack.try_push(|top_widget| {
                top_widget.child_by_index_mut(child_index)
            });


            match child_opt {
                Some(child) => {
                    for_each(child);
                    child_index = 0;
                },
                None => {
                    child_index = stack.top_index() + 1;
                    if stack.top().widget_id == parent {
                        break;
                    } else {
                        stack.pop();
                    }
                }
            }
        }

        assert_eq!(self.stack.top_id(), parent);
        offset_widget_scan::update_recursive(
            self.stack.top_mut().widget.inner_mut(),
            &mut self.virtual_widget_tree,
            &self.update_state
        );
    }

    pub fn root_id(&self) -> WidgetId {
        self.virtual_widget_tree.root_id()
    }

    pub fn all_widgets(&self) -> impl '_ + Iterator<Item=WidgetId> {
        self.virtual_widget_tree.all_nodes().map(|(id, _)| id)
    }
}

impl<D> WidgetTraverser<'_, D>
    where for<'d> D: DisplayEngine<'d>
{
    fn scan_for_widget(&mut self, widget_id: WidgetId) -> Option<OffsetWidgetPath<D>> {
        let stack = &mut self.stack;

        stack.truncate(1);
        let mut widget_found = false;
        let mut child_index = 0;
        loop {
            let valid_child = stack.try_push(|top_widget| {
                let top_id = top_widget.widget_id();

                if widget_id == top_id {
                    widget_found = true;
                    return None;
                }

                top_widget.child_by_index_mut(child_index)
            }).is_some();

            if widget_found {
                break Some(stack.top_mut());
            }

            match valid_child {
                true => child_index = 0,
                false => {
                    child_index = stack.top_index() + 1;
                    if stack.pop().is_none() {
                        break None
                    }
                }
            }
        }
    }

    fn add_stack_top_to_widget_tree(&mut self) {
        if let Some(parent) = self.stack.widgets().rev().skip(1).next() {
            let WidgetPath {
                path,
                index,
                widget_id,
                ..
            } = self.stack.top();
            match self.virtual_widget_tree.insert(parent.widget_id, widget_id, index, path.last().unwrap().clone()) {
                Ok(()) => (),
                Err(WidgetInsertError::ParentNotInTree) => {
                    for (parent, widget) in self.stack.widgets().zip(self.stack.widgets().skip(1)) {
                        self.virtual_widget_tree.insert(
                            parent.widget_id,
                            widget.widget_id,
                            widget.index,
                            widget.path.last().unwrap().clone()
                        ).ok();
                    }
                },
                Err(WidgetInsertError::WidgetIsRoot) => ()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        message_bus::MessageBus,
        test_helpers::TestRenderFrame,
        update_state::UpdateState,
    };
    use cgmath_geometry::rect::BoundBox;

    #[test]
    fn test_crawl_widgets() {
        test_widget_tree!{
            let event_list = crate::test_helpers::EventList::new();
            let mut tree = root {
                rect: (0, 0, 0, 0);
                a {
                    rect: (0, 0, 0, 0);
                    aa { rect: (0, 0, 0, 0) },
                    ab { rect: (0, 0, 0, 0) }
                },
                b { rect: (0, 0, 0, 0) },
                c { rect: (0, 0, 0, 0) }
            };
        }

        let mut traverser_base: WidgetTraverserBase<TestRenderFrame> = WidgetTraverserBase::new(root);
        let message_bus = MessageBus::new();
        let update_state = UpdateState::new(&message_bus);
        let mut traverser = traverser_base.with_root_ref(&mut tree, update_state.clone());

        let mut expected_id_iter = vec![
            root,
            a,
            aa,
            ab,
            b,
            c
        ].into_iter();

        traverser.crawl_widgets(|path| {
            assert_eq!(Some(path.widget_id), expected_id_iter.next());
        });
        assert_eq!(None, expected_id_iter.next());
    }

    #[test]
    fn get_widget() {
        test_widget_tree!{
            let event_list = crate::test_helpers::EventList::new();
            let mut tree = root {
                rect: (0, 0, 100, 100);
                a {
                    rect: (10, 10, 30, 30);
                    aa {
                        rect: (-10, -10, 10, 10);
                        aaa {rect: (0, 0, 10, 10)}
                    },
                    ab {rect: (10, 10, 20, 20)}
                },
                b {
                    rect: (20, 20, 40, 40);
                    ba {rect: (-10, -10, -1, -1)}
                }
            };
        }

        for _ in 0..1000 {
            let mut traverser_base: WidgetTraverserBase<TestRenderFrame> = WidgetTraverserBase::new(root);
            let message_bus = MessageBus::new();
            let update_state = UpdateState::new(&message_bus);
            let mut traverser = traverser_base.with_root_ref(&mut tree, update_state.clone());

            let mut assert_widget = |id, rect: (i32, i32, i32, i32), rect_clipped: Option<(i32, i32, i32, i32)>| {
                let widget_path = traverser.get_widget(id).unwrap();
                assert_eq!(widget_path.widget_id, id);

                let widget = widget_path.widget;
                assert_eq!(widget.rect(), BoundBox::new2(rect.0, rect.1, rect.2, rect.3));
                assert_eq!(widget.rect_clipped(), rect_clipped.map(|rect_clipped| BoundBox::new2(rect_clipped.0, rect_clipped.1, rect_clipped.2, rect_clipped.3)));
            };

            let mut assert_widget_index = |i: u32| {
                match i {
                    0 => {println!("root"); assert_widget(root, (0, 0, 100, 100), Some((0, 0, 100, 100)))},
                    1 => {println!("a"); assert_widget(a, (10, 10, 30, 30), Some((10, 10, 30, 30)))},
                    2 => {println!("aa"); assert_widget(aa, (0, 0, 20, 20), Some((10, 10, 20, 20)))},
                    3 => {println!("aaa"); assert_widget(aaa, (0, 0, 10, 10), Some((10, 10, 10, 10)))},
                    4 => {println!("ab"); assert_widget(ab, (20, 20, 30, 30), Some((20, 20, 30, 30)))},
                    5 => {println!("b"); assert_widget(b, (20, 20, 40, 40), Some((20, 20, 40, 40)))},
                    6 => {println!("ba"); assert_widget(ba, (10, 10, 19, 19), None)},
                    _ => panic!("invalid index")
                }
            };

            for _ in 0..1000 {
                use rand::Rng;
                let index = rand::thread_rng().gen_range(0, 7);
                assert_widget_index(index);
            }
            println!();
        }
    }

    #[test]
    fn crawl_widget_children() {
        test_widget_tree!{
            let event_list = crate::test_helpers::EventList::new();
            let mut tree = root {
                rect: (0, 0, 100, 100);
                a {
                    rect: (10, 10, 30, 30);
                    aa {
                        rect: (-10, -10, 10, 10);
                        aaa {rect: (0, 0, 10, 10)}
                    },
                    ab {rect: (10, 10, 20, 20)}
                },
                b {
                    rect: (20, 20, 40, 40);
                    ba {rect: (-10, -10, -1, -1)}
                }
            };
        }

        let mut traverser_base: WidgetTraverserBase<TestRenderFrame> = WidgetTraverserBase::new(root);
        let message_bus = MessageBus::new();
        let update_state = UpdateState::new(&message_bus);
        let mut traverser = traverser_base.with_root_ref(&mut tree, update_state.clone());

        let mut test_crawl_children = |id, children: &[WidgetId]| {
            println!();
            dbg!((&id, children));
            let mut children_iter = children.into_iter().cloned();
            traverser.crawl_widget_children(id, |wpath| {
                dbg!(wpath.widget_id);
                assert_eq!(Some(wpath.widget_id), children_iter.next());
            });
            assert_eq!(None, children_iter.next());
        };

        test_crawl_children(root, &[a, aa, aaa, ab, b, ba]);
        test_crawl_children(a, &[aa, aaa, ab]);
        test_crawl_children(aa, &[aaa]);
        test_crawl_children(aaa, &[]);
        test_crawl_children(ab, &[]);
        test_crawl_children(b, &[ba]);
        test_crawl_children(ba, &[]);
    }
}
