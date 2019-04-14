// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! The core set of widgets provided by Derin to create GUIs.

#[macro_use]
// pub mod assistants;
// mod button;
// mod check_box;
// mod clip;
// mod direct_render;
// mod edit_box;
// mod group;
pub mod label;
// mod progress_bar;
// mod radio_buttons;
// mod scroll_box;
// mod slider;
// mod tabs;

// pub use self::button::*;
// pub use self::check_box::*;
// pub use self::clip::*;
// pub use self::direct_render::*;
// pub use self::edit_box::*;
// pub use self::group::*;
#[doc(inline)]
pub use self::label::Label;
// pub use self::progress_bar::*;
// pub use self::radio_buttons::*;
// pub use self::scroll_box::*;
// pub use self::slider::*;
// pub use self::tabs::*;

/// The `Widget` trait, as well as associated types used to create custom widgets.
pub mod custom {
    pub use crate::core::widget::{WidgetTag, Widget, Parent, WidgetSubtype, WidgetInfo, WidgetInfoMut, WidgetIdent};
}
