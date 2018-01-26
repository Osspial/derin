use event::FocusChange;
use tree::NodeIdent;
use std::cmp;
use std::ops::RangeFrom;
use itertools::Itertools;

#[derive(Debug, Clone, Default)]
pub struct KeyboardFocusTracker {
    code_vec: Vec<FocusCode>
}

#[derive(Debug)]
pub struct FocusDrain<'a> {
    index: RangeFrom<usize>,
    code_vec: &'a mut Vec<FocusCode>
}

#[derive(Debug, Clone, Copy)]
enum FocusCode {
    FocusChange(FocusChange),
    Ident(NodeIdent)
}

impl KeyboardFocusTracker {
    pub fn push_focus<I>(&mut self, focus: FocusChange, at_path: I)
        where I: IntoIterator<Item=NodeIdent>
    {
        self.code_vec.push(FocusCode::FocusChange(focus));
        self.code_vec.extend(at_path.into_iter().map(|i| FocusCode::Ident(i)));
    }

    pub fn drain_focus<'a>(&'a mut self) -> FocusDrain<'a> {
        FocusDrain {
            index: 0..,
            code_vec: &mut self.code_vec
        }
    }
}

impl<'a> FocusDrain<'a> {
    pub fn push_focus<I>(&mut self, focus: FocusChange, at_path: I)
        where I: IntoIterator<Item=NodeIdent>
    {
        self.code_vec.push(FocusCode::FocusChange(focus));
        self.code_vec.extend(at_path.into_iter().map(|i| FocusCode::Ident(i)));
    }

    // pub fn next_focus(&self) -> Option<FocusChange> {
    //     match self.index.next().and_then(|i| self.code_vec.get(i).cloned())? {
    //         FocusCode::FocusChange(f) => Some(f),
    //         FocusCode::Ident(_) => unreachable!()
    //     }
    // }

    pub fn next<'b>(&'b mut self) -> Option<(FocusChange, impl 'b + Iterator<Item=NodeIdent>)> {
        let focus = match self.index.next().and_then(|i| self.code_vec.get(i).cloned())? {
            FocusCode::FocusChange(f) => f,
            FocusCode::Ident(_) => unreachable!()
        };


        let FocusDrain{ ref mut index, ref code_vec } = *self;
        let unwrap_ident = move |i| {
            match code_vec[i] {
                FocusCode::Ident(ident) => ident,
                FocusCode::FocusChange(..) => panic!("not an ident"),
            }
        };
        Some((focus, index.take_while_ref(move |i| code_vec.get(*i).map(FocusCode::is_ident).unwrap_or(false)).map(unwrap_ident)))
    }
}

impl<'a> Drop for FocusDrain<'a> {
    fn drop(&mut self) {
        let code_vec_len = self.code_vec.len();
        self.code_vec.drain(..cmp::min(self.index.start, code_vec_len));
    }
}

impl FocusCode {
    #[inline]
    fn is_ident(&self) -> bool {
        match *self {
            FocusCode::FocusChange(..) => false,
            FocusCode::Ident(..) => true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_push_drain() {
        let mut focus_tracker = KeyboardFocusTracker::default();
        focus_tracker.push_focus(FocusChange::Next, [NodeIdent::Num(0), NodeIdent::Str("hello")].into_iter().cloned());
        focus_tracker.push_focus(FocusChange::Prev, [NodeIdent::Str("goodbye"), NodeIdent::Str("again"), NodeIdent::Num(3)].into_iter().cloned());

        {
        let mut drain = focus_tracker.drain_focus();
            let (focus, mut iter) = drain.next().unwrap();
            assert_eq!(FocusChange::Next, focus);
            assert_eq!(Some(NodeIdent::Num(0)), iter.next());
            assert_eq!(Some(NodeIdent::Str("hello")), iter.next());
            assert_eq!(None, iter.next());
        }

        {
        let mut drain = focus_tracker.drain_focus();
            let (focus, mut iter) = drain.next().unwrap();
            assert_eq!(FocusChange::Prev, focus);
            assert_eq!(Some(NodeIdent::Str("goodbye")), iter.next());
            assert_eq!(Some(NodeIdent::Str("again")), iter.next());
            assert_eq!(Some(NodeIdent::Num(3)), iter.next());
            assert_eq!(None, iter.next());
        }

        let mut drain = focus_tracker.drain_focus();
        assert!(drain.next().is_none());
    }
}
