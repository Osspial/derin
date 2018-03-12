//! The core set of widgets provided by Derin to create GUIs.

mod button;
mod direct_render;
mod edit_box;
mod group;
mod label;
mod slider;

pub use self::button::*;
pub use self::direct_render::*;
pub use self::edit_box::*;
pub use self::group::*;
pub use self::label::*;
pub use self::slider::*;

use gl_render::{Prim, ThemedPrim, RenderString, RelPoint};
use cgmath::Point2;
use cgmath_geometry::{BoundBox, DimsBox};
use theme::Theme;
use core::render::Theme as CoreTheme;

/// The `Widget` trait, as well as associated types used to create custom widgets.
pub mod custom {
    pub use core::timer::TimerRegister;
    pub use core::tree::{UpdateTag, Widget, WidgetSummary, WidgetIdent, WidgetSubtrait, WidgetSubtraitMut};
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
    fn to_prim<D>(&mut self, background_name: &str, rect_px_out: Option<&mut BoundBox<Point2<i32>>>) -> ThemedPrim<D> {
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
                rect_px_out: rect_px_out.map(|r| r as *mut BoundBox<Point2<i32>>)
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
                rect_px_out: rect_px_out.map(|r| r as *mut BoundBox<Point2<i32>>)
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

    fn min_size(&self, theme: &Theme) -> DimsBox<Point2<i32>> {
        match *self {
            ContentsInner::Text(ref s) => s.min_size(),
            ContentsInner::Image(ref i) => theme.widget_theme(&**i).image.as_ref().and_then(|img| img.dims.cast()).unwrap_or(DimsBox::new2(0, 0))
        }
    }
}
