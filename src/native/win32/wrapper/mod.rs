use ui::Control;
use dle::LayoutUpdate;
use dle::hints::WidgetHints;
use dct::geometry::{SizeBounds, Rect, OriginRect, OffsetRect};

#[derive(UserMsg)]
pub enum Dm<'a, A: 'a> {
    SetWidgetHints(&'a WidgetHints, u64),
    RemoveChild(usize),

    OpenUpdateQueue,
    FlushUpdateQueue,
    QueueChildUpdates(&'a [LayoutUpdate<usize>]),
    SetRect(OffsetRect),
    SetControlPtr(&'a *const Control<Action = A>)
}
