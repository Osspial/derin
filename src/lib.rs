#![feature(specialization, never_type)]

extern crate dle;
extern crate dct;
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

#[cfg(target_os="windows")]
extern crate dww;
#[cfg(target_os="windows")]
#[cfg_attr(target_os="windows", macro_use)]
extern crate dww_macros;

// Switch between two different token trees, using the first token tree if the parenthesis contain
// tokens and the second tree if the parenthesis don't contain tokens.
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

pub mod native;
pub mod ui;
