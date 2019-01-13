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
use crate::tree::{Parent, WidgetIdent, WidgetSummary, Widget};

use arrayvec::ArrayVec;
use std::mem;

const CHILD_BATCH_SIZE: usize = 24;

pub type ForEachSummary<'a, W> = &'a mut FnMut(ArrayVec<[WidgetSummary<W>; CHILD_BATCH_SIZE]>) -> LoopFlow;
pub trait ParentDyn<F: RenderFrame>: Widget<F> {
    fn as_widget(&mut self) -> &mut Widget<F>;
    fn num_children(&self) -> usize;

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<F>>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<F>>>;

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<F>>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<F>>>;

    fn children<'a>(&'a self, for_each: ForEachSummary<&'a Widget<F>>);
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<&'a mut Widget<F>>);
}

impl<F, P> ParentDyn<F> for P
    where F: RenderFrame,
          P: Parent<F>
{
    fn as_widget(&mut self) -> &mut Widget<F> {
        self as &mut Widget<F>
    }
    fn num_children(&self) -> usize {
        <Self as Parent<F>>::num_children(self)
    }

    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Widget<F>>> {
        <Self as Parent<F>>::child(self, widget_ident)
    }
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Widget<F>>> {
        <Self as Parent<F>>::child_mut(self, widget_ident)
    }

    fn child_by_index(&self, index: usize) -> Option<WidgetSummary<&Widget<F>>> {
        <Self as Parent<F>>::child_by_index(self, index)
    }
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetSummary<&mut Widget<F>>> {
        <Self as Parent<F>>::child_by_index_mut(self, index)
    }

    fn children<'a>(&'a self, for_each: ForEachSummary<&'a Widget<F>>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        <Self as Parent<F>>::children::<_>(self, |summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    let full_avec = mem::replace(&mut child_avec, ArrayVec::new());
                    match for_each(full_avec) {
                        LoopFlow::Break => return LoopFlow::Break,
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
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<&'a mut Widget<F>>) {
        let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

        <Self as Parent<F>>::children_mut::<_>(self, |summary| {
            match child_avec.try_push(summary) {
                Ok(()) => (),
                Err(caperr) => {
                    let full_avec = mem::replace(&mut child_avec, ArrayVec::new());
                    match for_each(full_avec) {
                        LoopFlow::Break => return LoopFlow::Break,
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
}

impl<F: RenderFrame> ParentDyn<F> {
    #[inline]
    pub fn from_widget<W>(widget: &W) -> Option<&ParentDyn<F>>
        where W: Widget<F> + ?Sized
    {
        trait AsParent<F>
            where F: RenderFrame
        {
            fn as_parent_dyn(&self) -> Option<&ParentDyn<F>>;
        }
        impl<F, W> AsParent<F> for W
            where F: RenderFrame,
                  W: Widget<F> + ?Sized
        {
            #[inline(always)]
            default fn as_parent_dyn(&self) -> Option<&ParentDyn<F>> {
                None
            }
        }
        impl<F, W> AsParent<F> for W
            where F: RenderFrame,
                  W: ParentDyn<F>
        {
            #[inline(always)]
            fn as_parent_dyn(&self) -> Option<&ParentDyn<F>> {
                Some(self)
            }
        }

        widget.as_parent_dyn()
    }

    #[inline]
    pub fn from_widget_mut<W>(widget: &mut W) -> Option<&mut ParentDyn<F>>
        where W: Widget<F> + ?Sized
    {
        trait AsParent<F>
            where F: RenderFrame
        {
            fn as_parent_dyn(&mut self) -> Option<&mut ParentDyn<F>>;
        }
        impl<F, W> AsParent<F> for W
            where F: RenderFrame,
                  W: Widget<F> + ?Sized
        {
            #[inline(always)]
            default fn as_parent_dyn(&mut self) -> Option<&mut ParentDyn<F>> {
                None
            }
        }
        impl<F, W> AsParent<F> for W
            where F: RenderFrame,
                  W: ParentDyn<F>
        {
            #[inline(always)]
            fn as_parent_dyn(&mut self) -> Option<&mut ParentDyn<F>> {
                Some(self)
            }
        }

        widget.as_parent_dyn()
    }
}


pub fn to_widget_object<F, W>(widget: &W) -> &Widget<F>
    where W: Widget<F> + ?Sized,
          F: RenderFrame
{
    trait AsWidget<'a, F>
        where F: RenderFrame
    {
        fn as_widget_dyn(self) -> &'a Widget<F>;
    }
    impl<'a, F, W> AsWidget<'a, F> for &'a W
        where F: RenderFrame,
              W: Widget<F> + ?Sized
    {
        #[inline(always)]
        default fn as_widget_dyn(self) -> &'a Widget<F> {
            panic!("Invalid")
        }
    }
    impl<'a, F, W> AsWidget<'a, F> for &'a W
        where F: RenderFrame,
              W: Widget<F>
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a Widget<F> {
            self
        }
    }
    impl<'a, F> AsWidget<'a, F> for &'a Widget<F>
        where F: RenderFrame
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a Widget<F> {
            self
        }
    }

    widget.as_widget_dyn()
}

pub fn to_widget_object_mut<F, W>(widget: &mut W) -> &mut Widget<F>
    where W: Widget<F> + ?Sized,
          F: RenderFrame
{
    trait AsWidget<'a, F>
        where F: RenderFrame
    {
        fn as_widget_dyn(self) -> &'a mut Widget<F>;
    }
    impl<'a, F, W> AsWidget<'a, F> for &'a mut W
        where F: RenderFrame,
              W: Widget<F> + ?Sized
    {
        #[inline(always)]
        default fn as_widget_dyn(self) -> &'a mut Widget<F> {
            panic!("Invalid")
        }
    }
    impl<'a, F, W> AsWidget<'a, F> for &'a mut W
        where F: RenderFrame,
              W: Widget<F>
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a mut Widget<F> {
            self
        }
    }
    impl<'a, F> AsWidget<'a, F> for &'a mut Widget<F>
        where F: RenderFrame
    {
        #[inline(always)]
        fn as_widget_dyn(self) -> &'a mut Widget<F> {
            self
        }
    }

    widget.as_widget_dyn()
}
