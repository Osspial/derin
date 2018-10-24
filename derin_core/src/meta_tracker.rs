// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::event::{FocusChange, WidgetEvent};
use crate::tree::WidgetIdent;
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

pub(crate) struct MetaEvent<I: Iterator<Item=WidgetIdent>> {
    pub source: I,
    pub variant: MetaEventVariant
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MetaEventVariant {
    FocusChange(FocusChange),
    EventBubble(WidgetEvent)
}

#[derive(Debug, Clone)]
enum MetaCode {
    FocusChange(FocusChange),
    EventBubble(WidgetEvent),
    Ident(WidgetIdent)
}

impl MetaEventTracker {
    pub fn push_focus<I>(&mut self, focus: FocusChange, at_path: I)
        where I: IntoIterator<Item=WidgetIdent>
    {
        self.code_vec.push(MetaCode::FocusChange(focus));
        self.code_vec.extend(at_path.into_iter().map(|i| MetaCode::Ident(i)));
    }

    pub fn push_bubble<I>(&mut self, event: WidgetEvent, at_path: I)
        where I: IntoIterator<Item=WidgetIdent>
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
        where I: IntoIterator<Item=WidgetIdent>
    {
        self.code_vec.push(MetaCode::FocusChange(focus));
        self.code_vec.extend(at_path.into_iter().map(|i| MetaCode::Ident(i)));
    }

    pub fn push_bubble<I>(&mut self, event: WidgetEvent, at_path: I)
        where I: IntoIterator<Item=WidgetIdent>
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

    pub fn next<'b>(&'b mut self) -> Option<MetaEvent<impl 'b + Iterator<Item=WidgetIdent>>> {
        let variant = match self.index.next().and_then(|i| self.code_vec.get(i).cloned())? {
            MetaCode::FocusChange(f) => MetaEventVariant::FocusChange(f),
            MetaCode::EventBubble(e) => MetaEventVariant::EventBubble(e),
            MetaCode::Ident(_) => unreachable!()
        };


        let MetaDrain{ ref mut index, ref code_vec } = *self;
        let unwrap_ident = move |i| {
            match code_vec[i] {
                MetaCode::Ident(ref ident) => ident.clone(),
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
    use std::sync::Arc;

    #[test]
    fn focus_push_drain() {
        let mut focus_tracker = MetaEventTracker::default();
        let hello: Arc<str> = Arc::from("hello");
        let goodbye: Arc<str> = Arc::from("goodbye");
        let again: Arc<str> = Arc::from("again");

        focus_tracker.push_focus(FocusChange::Next, [WidgetIdent::Num(0), WidgetIdent::Str(hello.clone())].into_iter().cloned());
        focus_tracker.push_focus(FocusChange::Prev, [WidgetIdent::Str(goodbye.clone()), WidgetIdent::Str(again.clone()), WidgetIdent::Num(3)].into_iter().cloned());

        {
        let mut drain = focus_tracker.drain_meta();
            let MetaEvent {
                mut source,
                variant
            } = drain.next().unwrap();
            assert_eq!(MetaEventVariant::FocusChange(FocusChange::Next), variant);
            assert_eq!(Some(WidgetIdent::Num(0)), source.next());
            assert_eq!(Some(WidgetIdent::Str(hello.clone())), source.next());
            assert_eq!(None, source.next());
        }

        {
        let mut drain = focus_tracker.drain_meta();
            let MetaEvent {
                mut source,
                variant
            } = drain.next().unwrap();
            assert_eq!(MetaEventVariant::FocusChange(FocusChange::Prev), variant);
            assert_eq!(Some(WidgetIdent::Str(goodbye.clone())), source.next());
            assert_eq!(Some(WidgetIdent::Str(again.clone())), source.next());
            assert_eq!(Some(WidgetIdent::Num(3)), source.next());
            assert_eq!(None, source.next());
        }

        let mut drain = focus_tracker.drain_meta();
        assert!(drain.next().is_none());
    }
}
