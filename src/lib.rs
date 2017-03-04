#![feature(specialization, conservative_impl_trait)]

extern crate rand;
extern crate dle;
extern crate dct;

#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

#[cfg(target_os="windows")]
extern crate dww;
#[cfg(target_os="windows")]
#[cfg_attr(target_os="windows", macro_use)]
extern crate dww_macros;

pub mod native;
pub mod ui;
