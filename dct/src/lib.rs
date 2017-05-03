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

pub mod buttons;
pub mod geometry;
pub mod hints;
