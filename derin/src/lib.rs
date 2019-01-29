// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![feature(nll, range_contains, specialization, never_type)]
//! # The Derin User Interface Library
//! The Derin User Interface Library aims to be a simple, albeit powerful, set of widgets and
//! containers that makes it easy to design and compose your own complex desktop GUI applications.
//! Included are the aforementioned [widgets], a [desktop window builder][window], an [OpenGL-powered
//! renderer][renderer], and utilities for [creating your own widgets and renderers][custom].
//!
//! ## Installing External Libraries
//! Derin currently relies on two external libraries to handle text rendering: Freetype and Harfbuzz.
//! Harfbuzz is built and statically linked by `rustc`, and should lead to little trouble while
//! building (although Windows users getting build errors are encouraged to use the MSVC toolchain
//! over the GCC toolchain). The other, Freetype, is dynamically linked, and may require extra steps
//! to use on Windows. Documentation on getting it working can be found [here][freetype-build].
//!
//! ## Reading This Documentation
//! There are three key parts of Derin that are essential for getting started creating UIs: [window
//! creation][window], [widgets], and [UI hierarchy creation][container]. New users are advised to
//! look at those pages first, before browsing other sections of the documentation.
//!
//! [widgets]: widgets/index.html
//! [window]: struct.Window.html
//! [renderer]: gl_render/struct.GLRenderer.html
//! [custom]: widgets/custom/index.html
//! [freetype-build]: https://github.com/PistonDevelopers/freetype-sys/blob/master/README.md
//! [container]: container/trait.WidgetContainer.html

extern crate derin_common_types;
extern crate derin_atlas;
extern crate derin_layout_engine;
extern crate derin_core as core;
use cgmath_geometry::cgmath;
extern crate cgmath_geometry;
extern crate gullery;
#[macro_use]
extern crate gullery_macros;
extern crate glutin;
extern crate arrayvec;
extern crate glyphydog;
extern crate itertools;
extern crate unicode_segmentation;
extern crate clipboard;
extern crate png;
extern crate parking_lot;
#[macro_use]
extern crate lazy_static;

pub mod container;
pub mod gl_render;
mod glutin_window;
pub mod layout;
pub mod theme;
pub mod widgets;

pub use crate::glutin_window::{GlutinWindow as Window, WindowConfig};
pub use glutin::WindowAttributes;
pub use crate::core::LoopFlow;

/// `WidgetEvent` type and associated helpers.
pub mod event {
    pub use crate::core::event::{EventOps, InputState, MouseDown, FocusChange, WidgetEvent, WidgetEventSourced, MouseHoverChange};
    pub use derin_common_types::buttons::{ModifierKeys, Key, MouseButton};
}

/// Types used to assemble widget geometry.
///
/// The types within this module are all re-exported, either from `cgmath` or `cgmath-geometry`.
pub mod geometry {
    pub use crate::cgmath::{Point2, Vector2};
    pub use cgmath_geometry::{D2, rect, line};
}
