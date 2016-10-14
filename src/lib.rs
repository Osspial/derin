extern crate gl;
extern crate gl_raii;
extern crate cgmath;
extern crate freetype;
extern crate fnv;

pub mod draw;
pub mod ui;

static mut ID_COUNTER: u64 = 0;

fn get_unique_id() -> u64 {
    let id = unsafe{ ID_COUNTER };
    unsafe{ ID_COUNTER += 1 };
    id
}
