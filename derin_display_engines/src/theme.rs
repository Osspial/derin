use crate::Content;
use derin_core::widget::{WidgetIdent, WidgetId, WidgetPathEntry};

pub trait Theme<T> {
    fn set_widget_content<C: Content>(&mut self, path: &[WidgetPathEntry], content: &C);
    fn theme(&self, widget_id: WidgetId) -> T;
}
