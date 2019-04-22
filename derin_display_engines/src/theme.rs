use crate::Content;
use derin_core::widget::{WidgetIdent, WidgetId};

pub trait Theme<T> {
    fn widget_content<C: Content>(&mut self, path: &[ThemePathEntry], content: &C);
    fn theme(&self, path: &[ThemePathEntry]) -> T;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThemePathEntry {
    pub widget_id: WidgetId,
    pub ident: WidgetIdent,
    pub type_name: &'static str,
}
