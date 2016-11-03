#![feature(specialization)]

extern crate fnv;
extern crate rand;
#[macro_use]
extern crate bitflags;
extern crate boolinator;
#[macro_use]
extern crate lazy_static;

#[cfg(feature="gl_ui")]
extern crate gl;
#[cfg(feature="gl_ui")]
extern crate gl_raii;
#[cfg(feature="gl_ui")]
extern crate cgmath;
#[cfg(feature="gl_ui")]
extern crate freetype;
#[cfg(feature="gl_ui")]
extern crate glutin;

#[cfg(target_os="windows")]
extern crate user32;
#[cfg(target_os="windows")]
extern crate kernel32;
#[cfg(target_os="windows")]
extern crate dwmapi;
#[cfg(target_os="windows")]
extern crate winapi;

#[cfg(feature="gl_ui")]
pub mod draw;
pub mod native;
pub mod ui;


#[cfg(feature="gl_ui")]
static mut ID_COUNTER: u64 = 0;
#[cfg(feature="gl_ui")]
fn get_unique_id() -> u64 {
    let id = unsafe{ ID_COUNTER };
    unsafe{ ID_COUNTER += 1 };
    id
}
