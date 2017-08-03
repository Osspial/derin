#![feature(specialization, never_type)]

extern crate dle;
#[macro_use]
extern crate dct;

#[cfg(test)]
#[cfg_attr(test, macro_use)]
extern crate quickcheck;

#[cfg(target_os="windows")]
extern crate dww;

pub mod ui;
