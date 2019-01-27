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

use crate::{
    LoopFlow,
    event::{EventOps, InputState, WidgetEventSourced},
    render::{RenderFrame, RenderFrameClipped},
    widget::{Parent, WidgetIdent, Widget, WidgetRender, WidgetID, WidgetTag, WidgetInfo, WidgetInfoMut},
};
use arrayvec::ArrayVec;
use std::{
    mem,
    any::{Any, TypeId},
};
use cgmath_geometry::{
    D2, rect::BoundBox,
};
use derin_common_types::layout::SizeBounds;

const CHILD_BATCH_SIZE: usize = 24;

pub type ForEachSummary<'a, W> = &'a mut FnMut(ArrayVec<[W; CHILD_BATCH_SIZE]>) -> LoopFlow;

/// Trait used internally to aid with dynamic dispatch.
pub(crate) trait WidgetDyn<F: RenderFrame>: 'static {
    fn widget_tag(&self) -> &WidgetTag;
    fn widget_id(&self) -> WidgetID;

    fn rect(&self) -> BoundBox<D2, i32>;
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32>;
    fn on_widget_event(
        &mut self,
        event: WidgetEventSourced<'_>,
        input_state: InputState,
    ) -> EventOps;

    fn size_bounds(&self) -> SizeBounds;
    fn dispatch_message(&mut self, message: &Any);

    // Parent methods
    fn num_children(&self) -> usize;
    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, F>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, F>>;
    fn child_by_index(&self, index: usize) -> Option<WidgetInfo<'_, F>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetInfoMut<'_, F>>;
    fn children<'a>(&'a self, for_each: ForEachSummary<WidgetInfo<'a, F>>);
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<WidgetInfoMut<'a, F>>);

    // WidgetRender methods
    fn render(&mut self, frame: &mut RenderFrameClipped<F>);
    fn update_layout(&mut self, theme: &F::Theme);

    fn get_type_id(&self) -> TypeId;
    fn to_widget(&self) -> &Widget;
    fn to_widget_mut(&mut self) -> &mut Widget;
}

macro_rules! isolate_params {
    ($fn:ident(&               $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<F>>::$fn($self, $($ident),*));
    ($fn:ident(&mut            $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<F>>::$fn($self, $($ident),*));
    ($fn:ident(&$($lt:tt)?     $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<F>>::$fn($self, $($ident),*));
    ($fn:ident(&$($lt:tt)? mut $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<F>>::$fn($self, $($ident),*));
}

macro_rules! type_match {
    (
        $(
            fn $fn:ident$(<$($generic:tt),*>)?($($param:tt)*) $(-> $ret:ty)? {
                default => $default:expr,
                specialized($($bounds:tt)+) => $spec:expr
            }
        )+
    ) => {$(
        fn $fn$(<$($generic),*>)?($($param)*) $(-> $ret)? {
            trait TypeMatch<F>
                where F: RenderFrame
            {
                fn type_match$(<$($generic),*>)?($($param)*) $(-> $ret)?;
            }
            impl<F, W> TypeMatch<F> for W
                where F: RenderFrame
            {
                #[inline(always)]
                #[allow(unused_variables)]
                default fn type_match$(<$($generic),*>)?($($param)*) $(-> $ret)? {
                    $default
                }
            }
            impl<F, W> TypeMatch<F> for W
                where F: RenderFrame,
                      W: $($bounds)+
            {
                #[inline(always)]
                fn type_match$(<$($generic),*>)?($($param)*) $(-> $ret)? {
                    $spec
                }
            }

            isolate_params!(type_match($($param)*))
        }
    )+};
}

impl<W, F> WidgetDyn<F> for W
    where W: Widget,
          F: RenderFrame
{
    fn widget_tag(&self) -> &WidgetTag {
        <Self as Widget>::widget_tag(self)
    }
    fn widget_id(&self) -> WidgetID {
        <Self as Widget>::widget_id(self)
    }

    fn rect(&self) -> BoundBox<D2, i32> {
        <Self as Widget>::rect(self)
    }
    fn rect_mut(&mut self) -> &mut BoundBox<D2, i32> {
        <Self as Widget>::rect_mut(self)
    }
    fn on_widget_event(&mut self, event: WidgetEventSourced<'_>, input_state: InputState) -> EventOps {
        <Self as Widget>::on_widget_event(self, event, input_state)
    }

    fn size_bounds(&self) -> SizeBounds {
        <Self as Widget>::size_bounds(self)
    }
    fn dispatch_message(&mut self, message: &Any) {
        <Self as Widget>::dispatch_message(self, message)
    }

    type_match!{
        fn num_children(&self) -> usize {
            default => 0,
            specialized(Parent) => <Self as Parent>::num_children(self)
        }
        fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, F>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child::<F>(self, widget_ident)
        }
        fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, F>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_mut::<F>(self, widget_ident)
        }
        fn child_by_index(&self, index: usize) -> Option<WidgetInfo<'_, F>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_by_index::<F>(self, index)
        }
        fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetInfoMut<'_, F>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_by_index_mut::<F>(self, index)
        }
        fn children<'a>(&'a self, for_each: ForEachSummary<WidgetInfo<'a, F>>) {
            default => (),
            specialized(Parent) => {
                let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

                <Self as Parent>::framed_children::<F, _>(self, |summary| {
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
        fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<WidgetInfoMut<'a, F>>) {
            default => (),
            specialized(Parent) => {
                let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

                <Self as Parent>::framed_children_mut::<F, _>(self, |summary| {
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

        fn render(&mut self, frame: &mut RenderFrameClipped<F>) {
            default => (),
            specialized(WidgetRender<F>) => self.render(frame)
        }
        fn update_layout(&mut self, theme: &F::Theme) {
            default => (),
            specialized(WidgetRender<F>) => self.update_layout(theme)
        }
    }

    fn get_type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn to_widget(&self) -> &Widget {
        self
    }

    fn to_widget_mut(&mut self) -> &mut Widget {
        self
    }
}

impl<F> dyn WidgetDyn<F>
    where F: RenderFrame
{
    pub(crate) fn new<W: Widget>(widget: &W) -> &'_ WidgetDyn<F> {
        trait AsWidget<'a, F>
            where F: RenderFrame
        {
            fn as_widget_dyn(self) -> &'a WidgetDyn<F>;
        }
        impl<'a, F, W> AsWidget<'a, F> for &'a W
            where F: RenderFrame,
                  W: WidgetDyn<F>
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a WidgetDyn<F> {
                self
            }
        }
        impl<'a, F> AsWidget<'a, F> for &'a WidgetDyn<F>
            where F: RenderFrame
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a WidgetDyn<F> {
                self
            }
        }

        widget.as_widget_dyn()
    }

    pub(crate) fn new_mut<W: Widget>(widget: &mut W) -> &'_ mut WidgetDyn<F> {
        trait AsWidget<'a, F>
            where F: RenderFrame
        {
            fn as_widget_dyn(self) -> &'a mut WidgetDyn<F>;
        }
        impl<'a, F, W> AsWidget<'a, F> for &'a mut W
            where F: RenderFrame,
                  W: WidgetDyn<F>
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a mut WidgetDyn<F> {
                self
            }
        }
        impl<'a, F> AsWidget<'a, F> for &'a mut WidgetDyn<F>
            where F: RenderFrame
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a mut WidgetDyn<F> {
                self
            }
        }

        widget.as_widget_dyn()
    }
}


pub(crate) fn to_any<W>(widget: &mut W, f: impl FnOnce(&mut Any))
    where W: Widget + ?Sized
{
    trait AsWidgetDyn {
        fn as_widget_sized<G: FnOnce(&mut Any)>(&mut self, f: G);
    }
    impl<W> AsWidgetDyn for W
        where W: Widget + ?Sized
    {
        default fn as_widget_sized<G: FnOnce(&mut Any)>(&mut self, _f: G) {
            panic!("Invalid")
        }
    }
    impl<W> AsWidgetDyn for W
        where W: Widget
    {
        fn as_widget_sized<G: FnOnce(&mut Any)>(&mut self, f: G) {
            f(self);
        }
    }

    widget.as_widget_sized(f);
}
