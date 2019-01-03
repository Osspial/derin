mod widget_stack;
// mod virtual_widget_tree;

pub(crate) use self::widget_stack::WidgetPath;
use crate::{
    offset_widget::OffsetWidget,
    render::RenderFrame,
    tree::{Widget, WidgetID, WidgetIdent},
};
use self::{
    widget_stack::WidgetStack
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Relation {
    Parent,
    /// Sibling with a widget delta. // TODO EXPLAIN MORE
    Sibling(isize),
    ChildIdent(WidgetIdent),
    ChildIndex(usize)
}

pub(crate) struct WidgetTraverser<'a, A: 'static, F: RenderFrame> {
    stack: WidgetStack<'a, A, F>
}

impl<A, F> WidgetTraverser<'_, A, F>
    where F: RenderFrame
{
    pub fn get_widget(&mut self, id: WidgetID) -> Option<WidgetPath<'_, A, F>> {
        // TODO: OPTIMIZE
        self.scan_for_widget(id)
    }
    pub fn get_widget_relation(&mut self, id: WidgetID, relation: Relation) -> Option<WidgetPath<'_, A, F>> {
        // TODO: OPTIMIZE
        self.scan_for_widget(id)?;

        match relation {
            Relation::Parent => {
                self.stack.pop()?;
                Some(self.stack.top())
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

    pub fn remove_widget(&mut self, _id: WidgetID) {
        // TODO: IMPLEMENT IF NECESSARY
    }

    pub fn root_id(&self) -> WidgetID {
        self.stack.widgets().next().unwrap().widget_tag().widget_id
    }
}

impl<A, F> WidgetTraverser<'_, A, F>
    where F: RenderFrame
{
    fn scan_for_widget(&mut self, widget_id: WidgetID) -> Option<WidgetPath<A, F>> {
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
                break Some(self.stack.top());
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
