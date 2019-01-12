mod widget_stack;
mod virtual_widget_tree;
mod offset_widget_scan;

pub(crate) use self::{
    offset_widget_scan::OffsetWidgetScan,
    widget_stack::{WidgetPath, OffsetWidgetPath},
};
use crate::{
    render::RenderFrame,
    tree::{Widget, WidgetID, WidgetIdent},
    update_state::UpdateStateCell,
};
use std::rc::Rc;
use self::{
    widget_stack::{WidgetStack, WidgetStackCache},
    virtual_widget_tree::VirtualWidgetTree
};

pub(crate) type OffsetWidgetScanPath<'a, A, F> = WidgetPath<'a, OffsetWidgetScan<'a, A, F>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    Parent,
    /// Sibling with a widget delta. // TODO EXPLAIN MORE
    Sibling(isize),
    ChildIdent(WidgetIdent),
    ChildIndex(usize)
}

pub(crate) struct WidgetTraverserBase<A: 'static, F: RenderFrame> {
    stack_cache: WidgetStackCache<A, F>,
    virtual_widget_tree: VirtualWidgetTree,
}

pub(crate) struct WidgetTraverser<'a, A: 'static, F: RenderFrame> {
    stack: WidgetStack<'a, A, F>,
    virtual_widget_tree: &'a mut VirtualWidgetTree,
    update_state: Rc<UpdateStateCell>,
}

impl<A, F> WidgetTraverserBase<A, F>
    where F: RenderFrame
{
    pub fn new(root_id: WidgetID) -> Self {
        WidgetTraverserBase {
            stack_cache: WidgetStackCache::new(),
            virtual_widget_tree: VirtualWidgetTree::new(root_id),
        }
    }

    pub fn with_root_ref<'a>(&'a mut self, root: &'a mut dyn Widget<A, F>, update_state: Rc<UpdateStateCell>) -> WidgetTraverser<'a, A, F> {
        // This isn't a necessary limitation with the code, but the current code assumes this assertion
        // holds.
        assert_eq!(root.widget_tag().widget_id, self.virtual_widget_tree.root_id());

        WidgetTraverser {
            stack: self.stack_cache.use_cache(root),
            virtual_widget_tree: &mut self.virtual_widget_tree,
            update_state
        }
    }
}

impl<A, F> WidgetTraverser<'_, A, F>
    where F: RenderFrame
{
    pub fn get_widget(&mut self, id: WidgetID) -> Option<OffsetWidgetScanPath<'_, A, F>> {
        // TODO: OPTIMIZE
        self.scan_for_widget(id)?;
        self.add_stack_top_to_widget_tree();

        let WidgetTraverser {
            ref mut stack,
            ref mut virtual_widget_tree,
            ref update_state,
        } = self;

        Some(stack.top_mut().map(move |w| OffsetWidgetScan::new(w, virtual_widget_tree, update_state)))
    }

    pub fn get_widget_relation(&mut self, id: WidgetID, relation: Relation) -> Option<OffsetWidgetScanPath<'_, A, F>> {
        // TODO: OPTIMIZE
        self.scan_for_widget(id)?;

        let widget_opt = match relation {
            Relation::Parent => {
                self.stack.pop()?;
                Some(self.stack.top_mut())
            },
            Relation::Sibling(delta) => {
                let index = self.stack.top_index();
                self.stack.pop()?;
                self.stack.try_push(|widget| {
                    let widget_as_parent = widget.as_parent_mut()?;
                    widget_as_parent.child_by_index_mut((index as isize + delta) as usize)
                })
            },
            Relation::ChildIdent(ident) => {
                self.stack.try_push(|widget| {
                    let widget_as_parent = widget.as_parent_mut()?;
                    widget_as_parent.child_mut(ident)
                })
            },
            Relation::ChildIndex(index) => {
                self.stack.try_push(|widget| {
                    let widget_as_parent = widget.as_parent_mut()?;
                    widget_as_parent.child_by_index_mut(index)
                })
            },
        };
        match widget_opt {
            Some(_) => {
                self.add_stack_top_to_widget_tree();
                let WidgetTraverser {
                    ref mut stack,
                    ref mut virtual_widget_tree,
                    ref update_state,
                } = self;

                Some(stack.top_mut().map(move |w| OffsetWidgetScan::new(w, virtual_widget_tree, update_state)))
            },
            None => None
        }
    }

    pub fn remove_widget(&mut self, id: WidgetID) {
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
    pub fn sort_widgets_by_depth<'a>(&mut self, widgets: &'a mut [WidgetID]) -> &'a mut [WidgetID] {
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
    pub fn crawl_widgets(&mut self, mut for_each: impl FnMut(OffsetWidgetPath<'_, A, F>)) {
        let stack = &mut self.stack;

        stack.truncate(1);
        for_each(stack.top_mut());

        let mut child_index = 0;
        loop {
            let child_opt = stack.try_push(|top_widget| {
                if let Some(top_widget_as_parent) = top_widget.as_parent_mut() {
                    return top_widget_as_parent.child_by_index_mut(child_index);
                }

                None
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

    pub fn root_id(&self) -> WidgetID {
        self.virtual_widget_tree.root_id()
    }

    pub fn all_widgets(&self) -> impl '_ + Iterator<Item=WidgetID> {
        self.virtual_widget_tree.all_nodes().map(|(id, _)| id)
    }
}

impl<A, F> WidgetTraverser<'_, A, F>
    where F: RenderFrame
{
    fn scan_for_widget(&mut self, widget_id: WidgetID) -> Option<OffsetWidgetPath<A, F>> {
        let stack = &mut self.stack;

        stack.truncate(1);
        let mut widget_found = false;
        let mut child_index = 0;
        loop {
            let valid_child = stack.try_push(|top_widget| {
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
            self.virtual_widget_tree.insert(parent.widget_id, widget_id, index, path.last().unwrap().clone()).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_state::UpdateState;

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

        let mut traverser_base = WidgetTraverserBase::new(root);
        let update_state = UpdateState::new();
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
            println!("crawl {:?}", path.path);
            assert_eq!(Some(path.widget_id), expected_id_iter.next());
        });
        assert_eq!(None, expected_id_iter.next());
    }
}
