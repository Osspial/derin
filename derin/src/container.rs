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

//! Types used to specify children of container widgets.
//!
//! This module's primary functionality is in the `WidgetContainer` trait, and an implementation
//! which contains a single widget is provided with the `SingleContainer` struct.
use std::marker::PhantomData;

use core::LoopFlow;
use core::render::RenderFrame;
use core::tree::{WidgetIdent, WidgetSummary, Widget};

/// Designates a struct that contains other widgets.
///
/// This is used in conjunction with container widgets, such as [`Group`]. This usually shouldn't be
/// directly implemented; you're encouraged to derive it with the macro included in `derin_macros`.
/// Using this macro properly requires a few extra annotations within the type:
/// * `#[derin(action = "$action_type")]` is placed on the struct itself, and is used to set the
///   `Action` type.
/// * `#[derin(collection = "$type_in_collection")]` is placed on fields within the struct which aren't
///   themselves widgets, but are instead collections of widgets, such as `Vec`.
///
/// # Example
/// ```
/// pub struct SimpleAction;
///
/// #[derive(WidgetContainer)]
/// #[derin(action = "SimpleAction")]
/// struct Container {
///     label: Label,
///     edit_box: EditBox,
///     #[derin(collection = "Button<Option<GalleryEvent>>")]
///     buttons: Vec<Button<Option<GalleryEvent>>>
/// }
/// ```
pub trait WidgetContainer<A: 'static, F: RenderFrame>: 'static {
    type Widget: ?Sized + Widget<A, F>;

    /// Get the number of children stored within the container.
    fn num_children(&self) -> usize;

    /// Perform internal, immutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn children<'a, G, R>(&'a self, for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a Self::Widget>) -> LoopFlow<R>,
              A: 'a,
              F: 'a;

    /// Perform internal, mutable iteration over each child widget stored within the container,
    /// calling `for_each_child` on each child.
    fn children_mut<'a, G, R>(&'a mut self, for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a mut Self::Widget>) -> LoopFlow<R>,
              A: 'a,
              F: 'a;

    /// Get the child with the specified name.
    fn child(&self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&Self::Widget>> {
        self.children(|summary| {
            if summary.ident == widget_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    /// Mutably get the child with the specified name.
    fn child_mut(&mut self, widget_ident: WidgetIdent) -> Option<WidgetSummary<&mut Self::Widget>> {
        self.children_mut(|summary| {
            if summary.ident == widget_ident {
                LoopFlow::Break(summary)
            } else {
                LoopFlow::Continue
            }
        })
    }

    /// Get the child at the specified index.
    ///
    /// The index of a child is generally assumed to correspond with the order in which the children
    /// are defined within the container.
    fn child_by_index(&self, mut index: usize) -> Option<WidgetSummary<&Self::Widget>> {
        self.children(|summary| {
            if index == 0 {
                LoopFlow::Break(summary)
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        })
    }
    /// Mutably get the child at the specified index.
    ///
    /// The index of a child is generally assumed to correspond with the order in which the children
    /// are defined within the container.
    fn child_by_index_mut(&mut self, mut index: usize) -> Option<WidgetSummary<&mut Self::Widget>> {
        self.children_mut(|summary| {
            if index == 0 {
                LoopFlow::Break(summary)
            } else {
                index -= 1;
                LoopFlow::Continue
            }
        })
    }
}

/// A container that contains a single widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SingleContainer<A, F: RenderFrame, N: Widget<A, F>> {
    /// A widget.
    pub widget: N,
    _marker: PhantomData<(A, F)>
}

impl<A, F: RenderFrame, N: Widget<A, F>> SingleContainer<A, F, N> {
    /// Creates a new container containing the given widget.
    #[inline(always)]
    pub fn new(widget: N) -> SingleContainer<A, F, N> {
        SingleContainer{ widget, _marker: PhantomData }
    }
}

impl<A, F, N> WidgetContainer<A, F> for SingleContainer<A, F, N>
    where A: 'static,
          F: RenderFrame,
          N: 'static + Widget<A, F>
{
    type Widget = N;

    #[inline(always)]
    fn num_children(&self) -> usize {1}

    fn children<'a, G, R>(&'a self, mut for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a N>) -> LoopFlow<R>,
              A: 'a,
              F: 'a
    {
        match for_each_child(WidgetSummary::new(WidgetIdent::Num(0), 0, &self.widget)) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a mut N>) -> LoopFlow<R>,
              A: 'a,
              F: 'a
    {
        match for_each_child(WidgetSummary::new_mut(WidgetIdent::Num(0), 0, &mut self.widget)) {
            LoopFlow::Continue => None,
            LoopFlow::Break(r) => Some(r)
        }
    }
}

impl<A, F, W> WidgetContainer<A, F> for Vec<W>
    where A: 'static,
          F: RenderFrame,
          W: 'static + Widget<A, F>
{
    type Widget = W;

    #[inline(always)]
    fn num_children(&self) -> usize {
        self.len()
    }

    fn children<'a, G, R>(&'a self, mut for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a W>) -> LoopFlow<R>,
              A: 'a,
              F: 'a
    {
        for (index, widget) in self.iter().enumerate() {
            match for_each_child(WidgetSummary::new(WidgetIdent::Num(index as u32), index, widget)) {
                LoopFlow::Continue => (),
                LoopFlow::Break(r) => return Some(r)
            }
        }

        None
    }

    fn children_mut<'a, G, R>(&'a mut self, mut for_each_child: G) -> Option<R>
        where G: FnMut(WidgetSummary<&'a mut W>) -> LoopFlow<R>,
              A: 'a,
              F: 'a
    {
        for (index, widget) in self.iter_mut().enumerate() {
            match for_each_child(WidgetSummary::new_mut(WidgetIdent::Num(index as u32), index, widget)) {
                LoopFlow::Continue => (),
                LoopFlow::Break(r) => return Some(r)
            }
        }

        None
    }
}
