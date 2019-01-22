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

//! The core set of widgets provided by Derin to create GUIs.

pub mod assistants;
mod button;
mod check_box;
mod clip;
mod direct_render;
mod edit_box;
mod group;
mod label;
mod progress_bar;
mod radio_buttons;
mod scroll_box;
mod slider;
mod tabs;

pub use self::button::*;
pub use self::check_box::*;
pub use self::clip::*;
pub use self::direct_render::*;
pub use self::edit_box::*;
pub use self::group::*;
pub use self::label::*;
pub use self::progress_bar::*;
pub use self::radio_buttons::*;
pub use self::scroll_box::*;
pub use self::slider::*;
pub use self::tabs::*;

use crate::gl_render::{Prim, ThemedPrim, RenderString, RelPoint};
use crate::cgmath::Point2;
use cgmath_geometry::{D2, rect::{BoundBox, DimsBox}};
use crate::theme::Theme;
use crate::core::render::Theme as CoreTheme;

/// The `Widget` trait, as well as associated types used to create custom widgets.
pub mod custom {
    pub use crate::core::tree::{WidgetTag, Widget, WidgetSummary, WidgetIdent};
}

/// Determines which action, if any, should be taken in response to a button toggle.
pub trait ToggleHandler: 'static {
    fn change_state(&mut self, enabled: bool);
}

impl<A: 'static + Clone> ToggleHandler for Option<A> {
    /// Returns the stored action when the toggle is enabled. Otherwise, returns `None`.
    #[inline]
    fn change_state(&mut self, enabled: bool) {
        unimplemented!()
    }
}

impl ToggleHandler for () {
    /// Always returns `None`.
    #[inline]
    fn change_state(&mut self, _: bool) {
        unimplemented!()
    }
}

/// What should be drawn inside of a label, or other widgets that contains a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Contents<C=String> {
    /// Draw the text given in the string field.
    Text(C),
    /// Draw the theme image with the given name.
    Image(C)
}

#[derive(Debug, Clone)]
enum ContentsInner {
    Text(RenderString),
    Image(String)
}

impl<C> Contents<C> {
    pub fn as_text(self) -> Option<C> {
        match self {
            Contents::Text(c) => Some(c),
            _ => None
        }
    }

    pub fn as_image(self) -> Option<C> {
        match self {
            Contents::Image(c) => Some(c),
            _ => None
        }
    }
}

impl Contents<String> {
    fn to_inner(self) -> ContentsInner {
        match self {
            Contents::Text(t) => ContentsInner::Text(RenderString::new(t)),
            Contents::Image(i) => ContentsInner::Image(i)
        }
    }
}

impl ContentsInner {
    fn to_prim<D>(&mut self, background_name: &str, rect_px_out: Option<&mut BoundBox<D2, i32>>) -> ThemedPrim<D> {
        match *self {
            ContentsInner::Text(ref mut s) => ThemedPrim {
                theme_path: background_name,
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::String(s),
                rect_px_out: rect_px_out.map(|r| r as *mut BoundBox<D2, i32>)
            },
            ContentsInner::Image(ref i) => ThemedPrim {
                theme_path: &**i,
                min: Point2::new(
                    RelPoint::new(-1.0, 0),
                    RelPoint::new(-1.0, 0),
                ),
                max: Point2::new(
                    RelPoint::new( 1.0, 0),
                    RelPoint::new( 1.0, 0)
                ),
                prim: Prim::Image,
                rect_px_out: rect_px_out.map(|r| r as *mut BoundBox<D2, i32>)
            }
        }
    }

    fn borrow(&self) -> Contents<&str> {
        match *self {
            ContentsInner::Text(ref t) => Contents::Text(t.string()),
            ContentsInner::Image(ref s) => Contents::Image(s)
        }
    }

    fn borrow_mut(&mut self) -> Contents<&mut String> {
        match *self {
            ContentsInner::Text(ref mut t) => Contents::Text(t.string_mut()),
            ContentsInner::Image(ref mut s) => Contents::Image(s)
        }
    }

    fn min_size(&self, theme: &Theme) -> DimsBox<D2, i32> {
        match *self {
            ContentsInner::Text(ref s) => s.min_size(),
            ContentsInner::Image(ref i) => theme.widget_theme(&**i).image.as_ref().and_then(|img| img.dims.cast()).unwrap_or(DimsBox::new2(0, 0))
        }
    }
}
