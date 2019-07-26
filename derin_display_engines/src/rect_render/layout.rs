use derin_core::widget::WidgetId;
use std::collections::HashMap;
use super::{
    Rect,
    text::StringLayoutData,
};

pub struct LayoutCache {
    widget_rects: HashMap<WidgetId, Vec<Rect>>,
    string_layout: HashMap<WidgetId, StringLayoutData>,
}

pub
