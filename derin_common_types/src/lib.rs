// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use cgmath_geometry::cgmath;
extern crate num_traits;
extern crate cgmath_geometry;
#[macro_use]
extern crate bitflags;
#[cfg(feature = "serde")]
#[cfg_attr(feature = "serde", macro_use)]
extern crate serde;

pub type Px = i32;

#[macro_use]
mod macros;
pub mod buttons;
pub mod layout;
pub mod cursor;
