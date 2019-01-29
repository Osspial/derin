// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CursorIcon {
    Pointer,
    Wait,
    Crosshair,
    Hand,
    NotAllowed,
    Text,
    Move,
    SizeNS,
    SizeWE,
    SizeNeSw,
    SizeNwSe,
    SizeAll,
    Hide
}

impl Default for CursorIcon {
    #[inline]
    fn default() -> CursorIcon {
        CursorIcon::Pointer
    }
}
