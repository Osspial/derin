#![feature(slice_rotate, nll, range_contains, conservative_impl_trait, universal_impl_trait, clone_closures)]

pub extern crate dct;
extern crate dat;
extern crate dle;
pub extern crate derin_core as core;
extern crate cgmath;
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

pub mod container;
pub mod gl_render;
mod glutin_window;
pub mod layout;
pub mod theme;
pub mod widgets;

pub mod geometry {
    pub use cgmath::*;
    pub use cgmath_geometry::*;
}

pub use glutin_window::GlutinWindow as Window;
pub use glutin::WindowAttributes;
