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

#![feature(slice_rotate, nll, range_contains, conservative_impl_trait, universal_impl_trait, clone_closures, specialization)]
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

pub use glutin_window::{GlutinWindow as Window, WindowConfig};
pub use glutin::WindowAttributes;
pub use core::LoopFlow;

/// `WidgetEvent` type and associated helpers.
pub mod event {
    pub use core::event::{EventOps, InputState, MouseDown, FocusChange, WidgetEvent};
    pub use derin_common_types::buttons::{ModifierKeys, Key, MouseButton};
}

/// Types used to assemble widget geometry.
///
/// The types within this module are all re-exported, either from `cgmath` or `cgmath-geometry`.
pub mod geometry {
    pub use cgmath::{Point2, Vector2};
    pub use cgmath_geometry::{GeoBox, DimsBox, BoundBox, OffsetBox, Line, Ray, Segment, Linear, Intersection};
}
