// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! This module is Rust at its ugliest.

use crate::{
    LoopFlow,
    event::{EventOps, InputState, WidgetEventSourced},
    render::{DisplayEngine, DisplayEngineLayoutRender},
    widget::{Parent, WidgetIdent, Widget, WidgetRenderable, WidgetId, WidgetTag, WidgetInfo, WidgetInfoMut},
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

pub type ForEachSummary<'a, W> = &'a mut dyn FnMut(ArrayVec<[W; CHILD_BATCH_SIZE]>) -> LoopFlow;

/// Trait used internally to aid with dynamic dispatch.
pub(crate) trait WidgetDyn<D>: 'static
    where D: DisplayEngine
{
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
    fn dispatch_message(&mut self, message: &dyn Any);

    // Parent methods
    fn num_children(&self) -> usize;
    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, D>>;
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, D>>;
    fn child_by_index(&self, index: usize) -> Option<WidgetInfo<'_, D>>;
    fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetInfoMut<'_, D>>;
    fn children<'a>(&'a self, for_each: ForEachSummary<WidgetInfo<'a, D>>);
    fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<WidgetInfoMut<'a, D>>);

    // WidgetRenderable methods
    fn render(&mut self, renderer: <D as DisplayEngineLayoutRender<'_>>::Renderer);
    fn update_layout(&mut self, layout: <D as DisplayEngineLayoutRender<'_>>::Layout);

    fn type_id(&self) -> TypeId;
    fn to_widget(&self) -> &dyn Widget;
    fn to_widget_mut(&mut self) -> &mut dyn Widget;
}

macro_rules! isolate_params {
    ($fn:ident(&               $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<D>>::$fn($self, $($ident),*));
    ($fn:ident(&mut            $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<D>>::$fn($self, $($ident),*));
    ($fn:ident(&$($lt:tt)?     $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<D>>::$fn($self, $($ident),*));
    ($fn:ident(&$($lt:tt)? mut $self:ident $(, $ident:ident: $ty:ty)*)) => (<Self as TypeMatch<D>>::$fn($self, $($ident),*));
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
            trait TypeMatch<D>
                where D: DisplayEngine
            {
                fn type_match$(<$($generic),*>)?($($param)*) $(-> $ret)?;
            }
            impl<D, W> TypeMatch<D> for W
                where D: DisplayEngine
            {
                #[inline(always)]
                #[allow(unused_variables)]
                default fn type_match$(<$($generic),*>)?($($param)*) $(-> $ret)? {
                    $default
                }
            }
            impl<D, W> TypeMatch<D> for W
                where D: DisplayEngine,
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

impl<W, D> WidgetDyn<D> for W
    where W: Widget,
          D: DisplayEngine
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
    fn dispatch_message(&mut self, message: &dyn Any) {
        <Self as Widget>::dispatch_message(self, message)
    }

    type_match!{
        fn num_children(&self) -> usize {
            default => 0,
            specialized(Parent) => <Self as Parent>::num_children(self)
        }
        fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetInfo<'_, D>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child::<D>(self, widget_ident)
        }
        fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetInfoMut<'_, D>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_mut::<D>(self, widget_ident)
        }
        fn child_by_index(&self, index: usize) -> Option<WidgetInfo<'_, D>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_by_index::<D>(self, index)
        }
        fn child_by_index_mut(&mut self, index: usize) -> Option<WidgetInfoMut<'_, D>> {
            default => None,
            specialized(Parent) => <Self as Parent>::framed_child_by_index_mut::<D>(self, index)
        }
        fn children<'a>(&'a self, for_each: ForEachSummary<WidgetInfo<'a, D>>) {
            default => (),
            specialized(Parent) => {
                let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

                <Self as Parent>::framed_children::<D, _>(self, |summary| {
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
        fn children_mut<'a>(&'a mut self, for_each: ForEachSummary<WidgetInfoMut<'a, D>>) {
            default => (),
            specialized(Parent) => {
                let mut child_avec: ArrayVec<[_; CHILD_BATCH_SIZE]> = ArrayVec::new();

                <Self as Parent>::framed_children_mut::<D, _>(self, |summary| {
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

        fn render(&mut self, renderer: <D as DisplayEngineLayoutRender<'_>>::Renderer) {
            default => (),
            specialized(WidgetRenderable<D>) => self.render(renderer)
        }
        fn update_layout(&mut self, layout: <D as DisplayEngineLayoutRender<'_>>::Layout) {
            default => (),
            specialized(WidgetRenderable<D>) => self.update_layout(layout)
        }
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<W>()
    }

    fn to_widget(&self) -> &dyn Widget {
        self
    }

    fn to_widget_mut(&mut self) -> &mut dyn Widget {
        self
    }
}

impl<D> dyn WidgetDyn<D>
    where D: DisplayEngine
{
    pub(crate) fn new<W: Widget>(widget: &W) -> &'_ dyn WidgetDyn<D> {
        trait AsWidget<'a, D>
            where D: DisplayEngine
        {
            fn as_widget_dyn(self) -> &'a dyn WidgetDyn<D>;
        }
        impl<'a, D, W> AsWidget<'a, D> for &'a W
            where D: DisplayEngine,
                  W: WidgetDyn<D>
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a dyn WidgetDyn<D> {
                self
            }
        }
        impl<'a, D> AsWidget<'a, D> for &'a dyn WidgetDyn<D>
            where D: DisplayEngine
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a dyn WidgetDyn<D> {
                self
            }
        }

        widget.as_widget_dyn()
    }

    pub(crate) fn new_mut<W: Widget>(widget: &mut W) -> &'_ mut dyn WidgetDyn<D> {
        trait AsWidget<'a, D>
            where D: DisplayEngine
        {
            fn as_widget_dyn(self) -> &'a mut dyn WidgetDyn<D>;
        }
        impl<'a, D, W> AsWidget<'a, D> for &'a mut W
            where D: DisplayEngine,
                  W: WidgetDyn<D>
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a mut dyn WidgetDyn<D> {
                self
            }
        }
        impl<'a, D> AsWidget<'a, D> for &'a mut dyn WidgetDyn<D>
            where D: DisplayEngine
        {
            #[inline(always)]
            fn as_widget_dyn(self) -> &'a mut dyn WidgetDyn<D> {
                self
            }
        }

        widget.as_widget_dyn()
    }
}


pub(crate) fn to_any<W>(widget: &mut W, f: impl FnOnce(&mut dyn Any))
    where W: Widget + ?Sized
{
    trait AsWidgetDyn {
        fn as_widget_sized<G: FnOnce(&mut dyn Any)>(&mut self, f: G);
    }
    impl<W> AsWidgetDyn for W
        where W: Widget + ?Sized
    {
        default fn as_widget_sized<G: FnOnce(&mut dyn Any)>(&mut self, _f: G) {
            panic!("Invalid")
        }
    }
    impl<W> AsWidgetDyn for W
        where W: Widget
    {
        fn as_widget_sized<G: FnOnce(&mut dyn Any)>(&mut self, f: G) {
            f(self);
        }
    }

    widget.as_widget_sized(f);
}
