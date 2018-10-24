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

//! This module is Rust at its ugliest.

use crate::LoopFlow;
use crate::render::RenderFrame;
use crate::tree::{Parent, WidgetIdent, WidgetSummary, Widget, OnFocusOverflow};

use arrayvec::ArrayVec;
use std::mem;

const CHILD_BATCH_SIZE: usize = 24;

pub type ForEachSummary<'a, W> = &'a mut dyn FnMut(ArrayVec<[WidgetSummary<W>; CHILD_BATCH_SIZE]>) -> LoopFlow<()>;
pub trait ParentDyn<A, F: RenderFrame>: Widget<A, F> {
    fn as_widget(&mut self) -> &mut dyn Widget<A, F>;
    fn num_children(&self) -> usize;

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&dyn Widget<A, F>>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut dyn Widget<A, F>>>;

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&dyn Widget<A, F>>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut dyn Widget<A, F>>>;

    fn children<'a>(&'a self, for_each: ForEachSummary<&'a dyn Widget<A, F>>);
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<&'a mut dyn Widget<A, F>>);

    fn update_child_layout(&mut self);

    fn on_child_focus_overflow(&self) -> OnFocusOverflow;
}

impl<A, F, P> ParentDyn<A, F> for P
    where F: RenderFrame,
          P: Parent<A, F>
{
    fn as_widget(&mut self) -> &mut dyn Widget<A, F> {
        self as &mut dyn Widget<A, F>
    }
    fn num_children(&self) -> usize {
        <Self as Parent<A, F>>::num_children(self)
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&dyn Widget<A, F>>> {
        <Self as Parent<A, F>>::child(self, widget_ident)
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut dyn Widget<A, F>>> {
        <Self as Parent<A, F>>::child_mut(self, widget_ident)
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&dyn Widget<A, F>>> {
        <Self as Parent<A, F>>::child_by_index(self, index)
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut dyn Widget<A, F>>> {
        <Self as Parent<A, F>>::child_by_index_mut(self, index)
    }

    fn children<'a>(&'a self, for_each: ForEachSummary<&'a dyn Widget<A, F>>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        <Self as Parent<A, F>>::children::<_, ()>(self, |summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    let full_avec = mem::replace(&mut child_avec, ArrayVec::new());
                    match for_each(full_avec) {
                        LoopFlow::Break(_) => return LoopFlow::Break(()),
                        LoopFlow::Continue => ()
                    }
                    child_avec.push(caperr.element());
                }
            }

            LoopFlow::Continue
        });

        if child_avec.len() != 0 {
            let _ = for_each(child_avec);
        }
    }
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<&'a mut dyn Widget<A, F>>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        <Self as Parent<A, F>>::children_mut::<_, ()>(self, |summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    let full_avec = mem::replace(&mut child_avec, ArrayVec::new());
                    match for_each(full_avec) {
                        LoopFlow::Break(_) => return LoopFlow::Break(()),
                        LoopFlow::Continue => ()
                    }
                    child_avec.push(caperr.element());
                }
            }

            LoopFlow::Continue
        });

        if child_avec.len() != 0 {
            let _ = for_each(child_avec);
        }
    }

    fn update_child_layout(&mut self) {
        <Self as Parent<A, F>>::update_child_layout(self)
    }

    fn on_child_focus_overflow(&self) -> OnFocusOverflow {
        <Self as Parent<A, F>>::on_child_focus_overflow(self)
    }
}

impl<A, F: RenderFrame> dyn ParentDyn<A, F> {
    #[inline]
    pub fn from_widget<W>(widget: &W) -> Option<&dyn ParentDyn<A, F>>
        where W: Widget<A, F> + ?Sized
    {
        trait AsParent<A, F>
            where F: RenderFrame
        {
            fn as_parent_dyn(&self) -> Option<&dyn ParentDyn<A, F>>;
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: Widget<A, F> + ?Sized
        {
            #[inline(always)]
            default fn as_parent_dyn(&self) -> Option<&dyn ParentDyn<A, F>> {
                None
            }
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: ParentDyn<A, F>
        {
            #[inline(always)]
            fn as_parent_dyn(&self) -> Option<&dyn ParentDyn<A, F>> {
                Some(self)
            }
        }

        widget.as_parent_dyn()
    }

    #[inline]
    pub fn from_widget_mut<W>(widget: &mut W) -> Option<&mut dyn ParentDyn<A, F>>
        where W: Widget<A, F> + ?Sized
    {
        trait AsParent<A, F>
            where F: RenderFrame
        {
            fn as_parent_dyn(&mut self) -> Option<&mut dyn ParentDyn<A, F>>;
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: Widget<A, F> + ?Sized
        {
            #[inline(always)]
            default fn as_parent_dyn(&mut self) -> Option<&mut dyn ParentDyn<A, F>> {
                None
            }
        }
        impl<A, F, W> AsParent<A, F> for W
            where F: RenderFrame,
                  W: ParentDyn<A, F>
        {
            #[inline(always)]
            fn as_parent_dyn(&mut self) -> Option<&mut dyn ParentDyn<A, F>> {
                Some(self)
            }
        }

        widget.as_parent_dyn()
    }
}


pub fn to_widget_object<A, F, W>(widget: &W) -> &dyn Widget<A, F>
    where W: Widget<A, F> + ?Sized,
          F: RenderFrame
{
    trait AsWidget<'a, A, F>
        where F: RenderFrame
    {
        fn as_widget_dyn(self) -> &'a dyn Widget<A, F>;
    }
    impl<'a, A, F, W> AsWidget<'a, A, F> for &'a W
        where F: RenderFrame,
              W: Widget<A, F> + ?Sized
    {
        #[inline(always)]
        default fn as_widget_dyn(self) -> &'a dyn Widget<A, F> {
            panic!("Invalid")
        }
    }
    impl<'a, A, F, W> AsWidget<'a, A, F> for &'a W
        where F: RenderFrame,
              W: Widget<A, F>
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a dyn Widget<A, F> {
            self
        }
    }
    impl<'a, A, F> AsWidget<'a, A, F> for &'a dyn Widget<A, F>
        where F: RenderFrame
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a dyn Widget<A, F> {
            self
        }
    }

    widget.as_widget_dyn()
}

pub fn to_widget_object_mut<A, F, W>(widget: &mut W) -> &mut dyn Widget<A, F>
    where W: Widget<A, F> + ?Sized,
          F: RenderFrame
{
    trait AsWidget<'a, A, F>
        where F: RenderFrame
    {
        fn as_widget_dyn(self) -> &'a mut dyn Widget<A, F>;
    }
    impl<'a, A, F, W> AsWidget<'a, A, F> for &'a mut W
        where F: RenderFrame,
              W: Widget<A, F> + ?Sized
    {
        #[inline(always)]
        default fn as_widget_dyn(self) -> &'a mut dyn Widget<A, F> {
            panic!("Invalid")
        }
    }
    impl<'a, A, F, W> AsWidget<'a, A, F> for &'a mut W
        where F: RenderFrame,
              W: Widget<A, F>
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a mut dyn Widget<A, F> {
            self
        }
    }
    impl<'a, A, F> AsWidget<'a, A, F> for &'a mut dyn Widget<A, F>
        where F: RenderFrame
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a mut dyn Widget<A, F> {
            self
        }
    }

    widget.as_widget_dyn()
}
