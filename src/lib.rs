#![feature(specialization, conservative_impl_trait)]

extern crate rand;
extern crate boolinator;
#[macro_use]
extern crate lazy_static;
extern crate smallvec;
extern crate dle;

#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

#[cfg(target_os="windows")]
extern crate user32;
#[cfg(target_os="windows")]
extern crate kernel32;
#[cfg(target_os="windows")]
extern crate dwmapi;
#[cfg(target_os="windows")]
extern crate winapi;
#[cfg(target_os="windows")]
extern crate comctl32;

pub mod native;
pub mod ui;
