//! The core set of widgets provided by Derin to create GUIs.

mod button;
mod direct_render;
mod edit_box;
mod group;
mod label;

pub use self::button::*;
pub use self::direct_render::*;
pub use self::edit_box::*;
pub use self::group::*;
pub use self::label::*;

/// The `Widget` trait, as well as associated types used to create custom widgets.
pub mod custom {
    pub use core::timer::TimerRegister;
    pub use core::tree::{UpdateTag, Widget, WidgetSummary, WidgetIdent, WidgetSubtrait, WidgetSubtraitMut};
}
