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

pub(crate) type OffsetWidgetScanPath<'a, F> = WidgetPath<'a, OffsetWidgetScan<'a, F>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    Parent,
    /// Sibling with a widget delta. // TODO EXPLAIN MORE
    Sibling(isize),
    ChildIdent(WidgetIdent),
    ChildIndex(usize)
}

pub(crate) struct WidgetTraverserBase<F: RenderFrame> {
    stack_cache: WidgetStackCache<F>,
    virtual_widget_tree: VirtualWidgetTree,
}

pub(crate) struct WidgetTraverser<'a, F: RenderFrame> {
    stack: WidgetStack<'a, F>,
    virtual_widget_tree: &'a mut VirtualWidgetTree,
    update_state: Rc<UpdateStateCell>,
}

impl<F> WidgetTraverserBase<F>
    where F: RenderFrame
{
    pub fn new(root_id: WidgetID) -> Self {
        WidgetTraverserBase {
            stack_cache: WidgetStackCache::new(),
            virtual_widget_tree: VirtualWidgetTree::new(root_id),
        }
    }

    pub fn with_root_ref<'a>(&'a mut self, root: &'a mut dyn Widget<F>, update_state: Rc<UpdateStateCell>) -> WidgetTraverser<'a, F> {
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

impl<F> WidgetTraverser<'_, F>
    where F: RenderFrame
{
    pub fn get_widget(&mut self, id: WidgetID) -> Option<OffsetWidgetScanPath<'_, F>> {
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

    pub fn get_widget_relation(&mut self, id: WidgetID, relation: Relation) -> Option<OffsetWidgetScanPath<'_, F>> {
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

    fn get_widget_with_tree(&mut self, id: WidgetID) -> Option<OffsetWidgetPath<'_, F>> {
        let mut widget_path_rev = self.virtual_widget_tree.ident_chain_reversed(id).unwrap().cloned().collect::<Vec<_>>();
        self.move_to_path(widget_path_rev.drain(..).rev())
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
    pub fn crawl_widgets(&mut self, mut for_each: impl FnMut(OffsetWidgetPath<'_, F>)) {
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

impl<F> WidgetTraverser<'_, F>
    where F: RenderFrame
{
    fn scan_for_widget(&mut self, widget_id: WidgetID) -> Option<OffsetWidgetPath<F>> {
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

    fn move_to_path<I>(&mut self, ident_path: I) -> Option<OffsetWidgetPath<F>>
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
            true => Some(self.stack.top_mut()),
            false => None
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
