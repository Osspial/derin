// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

macro_rules! two_axis_type {
    () => {};
    ($(#[$attr:meta])* pub struct $name:ident (Into<$t:ty>); $($rest:tt)*) => {
        $(#[$attr])*
        pub struct $name {
            pub x: $t,
            pub y: $t
        }

        impl $name {
            #[inline]
            pub fn new<X, Y>(x: X, y: Y) -> $name
                    where X: Into<$t>,
                          Y: Into<$t> {
                $name {
                    x: x.into(),
                    y: y.into()
                }
            }
        }

        two_axis_type!($($rest)*);
    };
    ($(#[$attr:meta])* pub struct $name:ident ($t:ty); $($rest:tt)*) => {
        $(#[$attr])*
        pub struct $name {
            pub x: $t,
            pub y: $t
        }

        impl $name {
            #[inline]
            pub fn new(x: $t, y: $t) -> $name {
                $name {
                    x: x,
                    y: y
                }
            }
        }

        two_axis_type!($($rest)*);
    }
}

/// Switch between two different token trees, using the first token tree if the parenthesis contain
/// tokens and the second tree if the parenthesis don't contain tokens.
#[macro_export]
macro_rules! if_tokens {
    (
        ($($if_tokens:tt)+) {
            $($tokens_exist:tt)*
        } else {
            $($tokens_else:tt)*
        }
    ) => {
        $($tokens_exist)*
    };

    (
        () {
            $($tokens_exist:tt)*
        } else {
            $($tokens_else:tt)*
        }
    ) => {
        $($tokens_else)*
    };
}

#[macro_export]
macro_rules! id {
    ($vis:vis $Name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis struct $Name(std::num::NonZeroU32);

        impl $Name {
            #[inline]
            pub fn new() -> $Name {
                use std::sync::atomic::{AtomicUsize, Ordering};

                static ID_COUNTER: AtomicUsize = AtomicUsize::new(2);
                let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;

                $Name(std::num::NonZeroU32::new(id as u32).unwrap())
            }

            pub fn dummy() -> $Name {
                $Name(std::num::NonZeroU32::new(1).unwrap())
            }

            pub fn to_u32(self) -> u32 {
                self.0.get()
            }
        }
    }
}
