mod widget_stack;
mod virtual_widget_tree;

pub(crate) use self::widget_stack::{WidgetPath, OffsetWidgetPath};
use crate::{
    offset_widget::OffsetWidget,
    render::RenderFrame,
    tree::{Widget, WidgetID, WidgetIdent},
};
use self::{
    widget_stack::{WidgetStack, WidgetStackCache},
    virtual_widget_tree::VirtualWidgetTree
};

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
}

impl<A, F> WidgetTraverserBase<A, F>
    where F: RenderFrame
{
    pub fn new(root_id: WidgetID) -> Self {
        WidgetTraverserBase {
            stack_cache: WidgetStackCache::new(),
            virtual_widget_tree: VirtualWidgetTree::new(root_id)
        }
    }

    pub fn with_root_ref<'a>(&'a mut self, root: &'a mut dyn Widget<A, F>) -> WidgetTraverser<'a, A, F> {
        // This isn't a necessary limitation with the code, but the current code assumes this assertion
        // holds.
        assert_eq!(root.widget_tag().widget_id, self.virtual_widget_tree.root_id());

        WidgetTraverser {
            stack: self.stack_cache.use_cache(root),
            virtual_widget_tree: &mut self.virtual_widget_tree
        }
    }
}

impl<A, F> WidgetTraverser<'_, A, F>
    where F: RenderFrame
{
    pub fn get_widget(&mut self, id: WidgetID) -> Option<OffsetWidgetPath<'_, A, F>> {
        // TODO: OPTIMIZE
        self.scan_for_widget(id)?;

        if let Some(parent) = self.stack.widgets().rev().skip(1).next() {
            let WidgetPath {
                path,
                index,
                ..
            } = self.stack.top();
            self.virtual_widget_tree.insert(parent.widget_id, id, index, path.last().unwrap().clone());
            // TODO: SCAN CHILDREN FOR CHANGES ON `OffsetWidgetPath` DROP
        }

        Some(self.stack.top_mut())
    }
    pub fn get_widget_relation(&mut self, id: WidgetID, relation: Relation) -> Option<OffsetWidgetPath<'_, A, F>> {
        // TODO: OPTIMIZE
        self.scan_for_widget(id)?;

        match relation {
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

    pub fn root_id(&self) -> WidgetID {
        self.virtual_widget_tree.root_id()
    }
}

impl<A, F> WidgetTraverser<'_, A, F>
    where F: RenderFrame
{
    fn scan_for_widget(&mut self, widget_id: WidgetID) -> Option<OffsetWidgetPath<A, F>> {
        self.stack.truncate(1);
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
                break Some(self.stack.top_mut());
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
