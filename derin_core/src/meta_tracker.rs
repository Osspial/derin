use event::{FocusChange, NodeEventOwned};
use tree::NodeIdent;
use std::cmp;
use std::ops::RangeFrom;
use itertools::Itertools;

#[derive(Debug, Clone, Default)]
pub(crate) struct MetaEventTracker {
    code_vec: Vec<MetaCode>
}

#[derive(Debug)]
pub(crate) struct MetaDrain<'a> {
    index: RangeFrom<usize>,
    code_vec: &'a mut Vec<MetaCode>
}

pub(crate) struct MetaEvent<I: Iterator<Item=NodeIdent>> {
    pub source: I,
    pub variant: MetaEventVariant
}

pub(crate) enum MetaEventVariant {
    FocusChange(FocusChange),
    EventBubble(NodeEventOwned)
}

#[derive(Debug, Clone, Copy)]
enum MetaCode {
    FocusChange(FocusChange),
    EventBubble(NodeEventOwned),
    Ident(NodeIdent)
}

impl MetaEventTracker {
    pub fn push_focus<I>(&mut self, focus: FocusChange, at_path: I)
        where I: IntoIterator<Item=NodeIdent>
    {
        self.code_vec.push(MetaCode::FocusChange(focus));
        self.code_vec.extend(at_path.into_iter().map(|i| MetaCode::Ident(i)));
    }

    pub fn push_bubble<I>(&mut self, event: NodeEventOwned, at_path: I)
        where I: IntoIterator<Item=NodeIdent>
    {
        self.code_vec.push(MetaCode::EventBubble(event));
        self.code_vec.extend(at_path.into_iter().map(|i| MetaCode::Ident(i)));
    }

    pub fn drain_meta<'a>(&'a mut self) -> MetaDrain<'a> {
        MetaDrain {
            index: 0..,
            code_vec: &mut self.code_vec
        }
    }
}

impl<'a> MetaDrain<'a> {
    pub fn push_focus<I>(&mut self, focus: FocusChange, at_path: I)
        where I: IntoIterator<Item=NodeIdent>
    {
        self.code_vec.push(MetaCode::FocusChange(focus));
        self.code_vec.extend(at_path.into_iter().map(|i| MetaCode::Ident(i)));
    }

    pub fn push_bubble<I>(&mut self, event: NodeEventOwned, at_path: I)
        where I: IntoIterator<Item=NodeIdent>
    {
        self.code_vec.push(MetaCode::EventBubble(event));
        self.code_vec.extend(at_path.into_iter().map(|i| MetaCode::Ident(i)));
    }

    // pub fn next_focus(&self) -> Option<FocusChange> {
    //     match self.index.next().and_then(|i| self.code_vec.get(i).cloned())? {
    //         MetaCode::FocusChange(f) => Some(f),
    //         MetaCode::Ident(_) => unreachable!()
    //     }
    // }

    pub fn next<'b>(&'b mut self) -> Option<MetaEvent<impl 'b + Iterator<Item=NodeIdent>>> {
        let variant = match self.index.next().and_then(|i| self.code_vec.get(i).cloned())? {
            MetaCode::FocusChange(f) => MetaEventVariant::FocusChange(f),
            MetaCode::EventBubble(e) => MetaEventVariant::EventBubble(e),
            MetaCode::Ident(_) => unreachable!()
        };


        let MetaDrain{ ref mut index, ref code_vec } = *self;
        let unwrap_ident = move |i| {
            match code_vec[i] {
                MetaCode::Ident(ident) => ident,
                MetaCode::FocusChange(..) |
                MetaCode::EventBubble(..) => panic!("not an ident"),
            }
        };
        let iter = index
            .take_while_ref(
                move |i| code_vec.get(*i).map(MetaCode::is_ident).unwrap_or(false)
            ).map(unwrap_ident);

        Some(MetaEvent {
            source: iter,
            variant
        })
    }
}

impl<'a> Drop for MetaDrain<'a> {
    fn drop(&mut self) {
        let code_vec_len = self.code_vec.len();
        self.code_vec.drain(..cmp::min(self.index.start, code_vec_len));
    }
}

impl MetaCode {
    #[inline]
    fn is_ident(&self) -> bool {
        match *self {
            MetaCode::FocusChange(..) |
            MetaCode::EventBubble(..) => false,
            MetaCode::Ident(..) => true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_push_drain() {
        let mut focus_tracker = MetaEventTracker::default();
        focus_tracker.push_focus(FocusChange::Next, [NodeIdent::Num(0), NodeIdent::Str("hello")].into_iter().cloned());
        focus_tracker.push_focus(FocusChange::Prev, [NodeIdent::Str("goodbye"), NodeIdent::Str("again"), NodeIdent::Num(3)].into_iter().cloned());

        {
        let mut drain = focus_tracker.drain_meta();
            let (focus, mut iter) = drain.next().unwrap();
            assert_eq!(FocusChange::Next, focus);
            assert_eq!(Some(NodeIdent::Num(0)), iter.next());
            assert_eq!(Some(NodeIdent::Str("hello")), iter.next());
            assert_eq!(None, iter.next());
        }

        {
        let mut drain = focus_tracker.drain_meta();
            let (focus, mut iter) = drain.next().unwrap();
            assert_eq!(FocusChange::Prev, focus);
            assert_eq!(Some(NodeIdent::Str("goodbye")), iter.next());
            assert_eq!(Some(NodeIdent::Str("again")), iter.next());
            assert_eq!(Some(NodeIdent::Num(3)), iter.next());
            assert_eq!(None, iter.next());
        }

        let mut drain = focus_tracker.drain_meta();
        assert!(drain.next().is_none());
    }
}
