// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! This module is Rust at its ugliest.

use crate::{
    LoopFlow,
    event::{EventOps, InputState, WidgetEventSourced},
    render::{Renderer},
    widget::{Parent, WidgetIdent, Widget, WidgetRender, WidgetId, WidgetTag, WidgetInfo, WidgetInfoMut},
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
pub(crate) trait WidgetDyn<R: Renderer>: 'static {
    fn widget_tag(&self) -> &WidgetTag;
    fn widget_id(&self) -> WidgetId;

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
    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, R>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, R>>;
    fn child_by_index(&self, index: usize) -> Option<WidgetInfo<'_, R>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetInfoMut<'_, R>>;
    fn children<'a>(&'a self, for_each: ForEachSummary<WidgetInfo<'a, R>>);
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<WidgetInfoMut<'a, R>>);

    // WidgetRender methods
    fn render(&mut self, frame: &mut R::SubFrame);
    fn update_layout(&mut self, layout: &mut R::Layout);

    fn type_id(&self) -> TypeId;
    fn to_widget(&self) -> &Widget;
    fn to_widget_mut(&mut self) -> &mut Widget;
}

macro_rules! isolate_params {
    ($fn:ident(&               $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<R>>::$fn($self, $($ident),*));
    ($fn:ident(&mut            $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<R>>::$fn($self, $($ident),*));
    ($fn:ident(&$($lt:tt)?     $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<R>>::$fn($self, $($ident),*));
    ($fn:ident(&$($lt:tt)? mut $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<R>>::$fn($self, $($ident),*));
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
            trait TypeMatch<R>
                where R: Renderer
            {
                fn type_match$(<$($generic),*>)?($($param)*) $(-> $ret)?;
            }
            impl<R, W> TypeMatch<R> for W
                where R: Renderer
            {
                #[inline(always)]
                #[allow(unused_variables)]
                default fn type_match$(<$($generic),*>)?($($param)*) $(-> $ret)? {
                    $default
                }
            }
            impl<R, W> TypeMatch<R> for W
                where R: Renderer,
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

impl<W, R> WidgetDyn<R> for W
    where W: Widget,
          R: Renderer
{
    fn widget_tag(&self) -> &WidgetTag {
        <Self as Widget>::widget_tag(self)
    }
    fn widget_id(&self) -> WidgetId {
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
        fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, R>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child::<R>(self, widget_ident)
        }
        fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, R>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_mut::<R>(self, widget_ident)
        }
        fn child_by_index(&self, index: usize) -> Option<WidgetInfo<'_, R>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_by_index::<R>(self, index)
        }
        fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetInfoMut<'_, R>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_by_index_mut::<R>(self, index)
        }
        fn children<'a>(&'a self, for_each: ForEachSummary<WidgetInfo<'a, R>>) {
            default => (),
            specialized(Parent) => {
                let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

                <Self as Parent>::framed_children::<R, _>(self, |summary| {
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
        fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<WidgetInfoMut<'a, R>>) {
            default => (),
            specialized(Parent) => {
                let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

                <Self as Parent>::framed_children_mut::<R, _>(self, |summary| {
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

        fn render(&mut self, frame: &mut R::SubFrame) {
            default => (),
            specialized(WidgetRender<R>) => self.render(frame)
        }
        fn update_layout(&mut self, layout: &mut R::Layout) {
            default => (),
            specialized(WidgetRender<R>) => self.update_layout(layout)
        }
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn to_widget(&self) -> &Widget {
        self
    }

    fn to_widget_mut(&mut self) -> &mut Widget {
        self
    }
}

impl<R> dyn WidgetDyn<R>
    where R: Renderer
{
    pub(crate) fn new<W: Widget>(widget: &W) -> &'_ WidgetDyn<R> {
        trait AsWidget<'a, R>
            where R: Renderer
        {
            fn as_widget_dyn(self) -> &'a WidgetDyn<R>;
        }
        impl<'a, R, W> AsWidget<'a, R> for &'a W
            where R: Renderer,
                  W: WidgetDyn<R>
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a WidgetDyn<R> {
                self
            }
        }
        impl<'a, R> AsWidget<'a, R> for &'a WidgetDyn<R>
            where R: Renderer
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a WidgetDyn<R> {
                self
            }
        }

        widget.as_widget_dyn()
    }

    pub(crate) fn new_mut<W: Widget>(widget: &mut W) -> &'_ mut WidgetDyn<R> {
        trait AsWidget<'a, R>
            where R: Renderer
        {
            fn as_widget_dyn(self) -> &'a mut WidgetDyn<R>;
        }
        impl<'a, R, W> AsWidget<'a, R> for &'a mut W
            where R: Renderer,
                  W: WidgetDyn<R>
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a mut WidgetDyn<R> {
                self
            }
        }
        impl<'a, R> AsWidget<'a, R> for &'a mut WidgetDyn<R>
            where R: Renderer
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a mut WidgetDyn<R> {
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
