use crate::Content;
use derin_core::widget::{WidgetId, WidgetPathEntry};

pub trait Theme {
    type Style;
    fn set_widget_content<C: Content>(&mut self, path: &[WidgetPathEntry], content: &C);
    fn style(&self, widget_id: WidgetId) -> Self::Style;
}
