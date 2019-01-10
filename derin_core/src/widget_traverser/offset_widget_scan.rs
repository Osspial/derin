use std::{
    ops::{Deref, DerefMut, Drop},
    rc::Rc,
};
use crate::{
    LoopFlow,
    render::RenderFrame,
    offset_widget::OffsetWidget,
    update_state::UpdateStateCell,
    tree::Widget,
};
use super::virtual_widget_tree::VirtualWidgetTree;

pub(crate) struct OffsetWidgetScan<'a, A, F: RenderFrame> {
    offset_widget: OffsetWidget<'a, dyn Widget<A, F>>,
    virtual_widget_tree: &'a mut VirtualWidgetTree,
    update_state: &'a Rc<UpdateStateCell>,
    scan: bool
}

impl<'a, A, F: RenderFrame> Deref for OffsetWidgetScan<'a, A, F> {
    type Target = OffsetWidget<'a, dyn Widget<A, F>>;

    fn deref(&self) -> &Self::Target {
        &self.offset_widget
    }
}

impl<'a, A, F: RenderFrame> DerefMut for OffsetWidgetScan<'a, A, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scan = true;

        &mut self.offset_widget
    }
}


impl<A, F: RenderFrame> Drop for OffsetWidgetScan<'_, A, F> {
    fn drop(&mut self) {
        if self.scan {
            update_recursive(self.offset_widget.inner(), self.virtual_widget_tree, &self.update_state);
        }
    }
}

impl<A, F: RenderFrame> OffsetWidgetScan<'_, A, F> {
    pub fn new<'a>(offset_widget: OffsetWidget<'a, dyn Widget<A, F>>, virtual_widget_tree: &'a mut VirtualWidgetTree, update_state: &'a Rc<UpdateStateCell>) -> OffsetWidgetScan<'a, A, F> {
        OffsetWidgetScan {
            offset_widget,
            virtual_widget_tree,
            update_state,
            scan: false,
        }
    }

    pub fn cancel_scan(&mut self) {
        self.scan = false;
    }
}

fn update_recursive<A, F: RenderFrame>(widget: &dyn Widget<A, F>, tree: &mut VirtualWidgetTree, update_state: &Rc<UpdateStateCell>) {
    let widget_tag = widget.widget_tag();
    let widget_id = widget_tag.widget_id;
    widget_tag.set_owning_update_state(update_state);

    if let Some(widget) = widget.as_parent() {
        widget.children(&mut |children| {
            for child in children {
                tree.insert(widget_id, child.widget.widget_tag().widget_id, child.index, child.ident);
                update_recursive(child.widget, tree, update_state);
            }
            LoopFlow::Continue
        });
    }
}
