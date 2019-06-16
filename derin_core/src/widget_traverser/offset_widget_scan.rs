// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::{
    ops::{Deref, DerefMut, Drop},
    rc::Rc,
};
use crate::{
    LoopFlow,
    render::DisplayEngine,
    offset_widget::OffsetWidget,
    update_state::UpdateStateCell,
    widget::WidgetDyn,
};
use super::virtual_widget_tree::VirtualWidgetTree;

pub(crate) struct OffsetWidgetScan<'a, D>
    where D: DisplayEngine
{
    offset_widget: OffsetWidget<'a, D>,
    virtual_widget_tree: &'a mut VirtualWidgetTree,
    update_state: &'a Rc<UpdateStateCell>,
    scan: bool
}

impl<'a, D> Deref for OffsetWidgetScan<'a, D>
    where D: DisplayEngine
{
    type Target = OffsetWidget<'a, D>;

    fn deref(&self) -> &Self::Target {
        &self.offset_widget
    }
}

impl<'a, D> DerefMut for OffsetWidgetScan<'a, D>
    where D: DisplayEngine
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.scan = true;

        &mut self.offset_widget
    }
}


impl<'a, D: DisplayEngine> Drop for OffsetWidgetScan<'a, D> {
    fn drop(&mut self) {
        if self.scan {
            update_recursive(self.offset_widget.inner(), self.virtual_widget_tree, &self.update_state);
        }
    }
}

impl<D> OffsetWidgetScan<'_, D>
    where D: DisplayEngine
{
    pub fn new<'a>(offset_widget: OffsetWidget<'a, D>, virtual_widget_tree: &'a mut VirtualWidgetTree, update_state: &'a Rc<UpdateStateCell>) -> OffsetWidgetScan<'a, D> {
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

pub(crate) fn update_recursive<D>(widget: &dyn WidgetDyn<D>, tree: &mut VirtualWidgetTree, update_state: &Rc<UpdateStateCell>)
    where D: DisplayEngine
{
    let widget_tag = widget.widget_tag();
    let widget_id = widget_tag.widget_id;
    widget_tag.set_owning_update_state(update_state);

    widget.children(&mut |children| {
        for child in children {
            tree.insert(widget_id, child.widget.widget_id(), child.index, child.ident).expect("Widget insert error");
            update_recursive(child.widget, tree, update_state);
        }
        LoopFlow::Continue
    });
}
