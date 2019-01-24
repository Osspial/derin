use std::{
    ops::{Deref, DerefMut, Drop},
    rc::Rc,
};
use crate::{
    LoopFlow,
    render::RenderFrame,
    offset_widget::OffsetWidget,
    update_state::UpdateStateCell,
    widget::Widget,
};
use super::virtual_widget_tree::VirtualWidgetTree;

pub(crate) struct OffsetWidgetScan<'a, F: RenderFrame> {
    offset_widget: OffsetWidget<'a, dyn Widget<F>>,
    virtual_widget_tree: &'a mut VirtualWidgetTree,
    update_state: &'a Rc<UpdateStateCell>,
    scan: bool
}

impl<'a, F: RenderFrame> Deref for OffsetWidgetScan<'a, F> {
    type Target = OffsetWidget<'a, dyn Widget<F>>;

    fn deref(&self) -> &Self::Target {
        &self.offset_widget
    }
}

impl<'a, F: RenderFrame> DerefMut for OffsetWidgetScan<'a, F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scan = true;

        &mut self.offset_widget
    }
}


impl<F: RenderFrame> Drop for OffsetWidgetScan<'_, F> {
    fn drop(&mut self) {
        if self.scan {
            update_recursive(self.offset_widget.inner(), self.virtual_widget_tree, &self.update_state);
        }
    }
}

impl<F: RenderFrame> OffsetWidgetScan<'_, F> {
    pub fn new<'a>(offset_widget: OffsetWidget<'a, dyn Widget<F>>, virtual_widget_tree: &'a mut VirtualWidgetTree, update_state: &'a Rc<UpdateStateCell>) -> OffsetWidgetScan<'a, F> {
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

pub(crate) fn update_recursive<F: RenderFrame>(widget: &dyn Widget<F>, tree: &mut VirtualWidgetTree, update_state: &Rc<UpdateStateCell>) {
    let widget_tag = widget.widget_tag();
    let widget_id = widget_tag.widget_id;
    widget_tag.set_owning_update_state(update_state);

    if let Some(widget) = widget.as_parent() {
        widget.children(&mut |children| {
            for child in children {
                tree.insert(widget_id, child.widget.widget_id(), child.index, child.ident).expect("Widget insert error");
                update_recursive(child.widget, tree, update_state);
            }
            LoopFlow::Continue
        });
    }
}
