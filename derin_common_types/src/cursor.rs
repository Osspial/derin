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
