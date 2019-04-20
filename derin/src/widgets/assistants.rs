// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use serde::{Serialize, Deserialize};

// mod slider;
// pub mod text_edit;
// pub mod toggle_button;

// pub use self::slider::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
    // Disabled,
    // Defaulted
}
