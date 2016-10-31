#![feature(specialization)]

extern crate gl;
extern crate gl_raii;
extern crate cgmath;
extern crate freetype;
extern crate fnv;
extern crate glutin;
#[macro_use]
extern crate bitflags;

#[cfg(target_os="windows")]
extern crate user32;
#[cfg(target_os="windows")]
extern crate kernel32;
#[cfg(target_os="windows")]
extern crate dwmapi;
#[cfg(target_os="windows")]
extern crate winapi;

use fnv::FnvHasher;

use std::hash::BuildHasherDefault;
use std::collections::HashMap;

use draw::{Shadable, Widget};

pub mod draw;
pub mod native;
pub mod ui;


static mut ID_COUNTER: u64 = 0;

fn get_unique_id() -> u64 {
    let id = unsafe{ ID_COUNTER };
    unsafe{ ID_COUNTER += 1 };
    id
}

type HasherType = BuildHasherDefault<FnvHasher>;

pub trait Renderer {
    type Processor: DataProcessor;

    fn processor(&mut self) -> Self::Processor;
}

pub trait DataProcessor {
    type DispData: Default;

    fn render_flags(&self) -> RenderFlags;
    fn update_data<S: Shadable>(&mut self, &S, &mut Self::DispData);
    fn render_data(&mut self, &Self::DispData);
}

bitflags! {
    pub flags RenderFlags: u64 {
        const FORCE_UPDATE = 0b1
    }
}

#[derive(Default)]
pub struct Display<R: Renderer> {
    id_map: HashMap<u64, IDMapEntry<<R::Processor as DataProcessor>::DispData>, HasherType>,
    renderer: R
}

impl<R: Renderer> Display<R> {
    pub fn new(renderer: R) -> Display<R> {
        Display {
            id_map: HashMap::default(),
            renderer: renderer
        }
    }

    pub fn dispatcher(&mut self) -> Dispatcher<R::Processor> {
        Dispatcher {
            id_map: &mut self.id_map,
            data_processor: self.renderer.processor()
        }
    }
}


struct IDMapEntry<D> {
    num_updates: u64,
    data: D
}

pub struct Dispatcher<'a, D: 'a + DataProcessor> {
    id_map: &'a mut HashMap<u64, IDMapEntry<D::DispData>, HasherType>,
    data_processor: D
}

impl<'a, D: 'a + DataProcessor> Dispatcher<'a, D> {
    pub fn draw<S: Shadable>(&mut self, widget: &Widget<S>) {
        use std::collections::hash_map::Entry;

        let render_flags = self.data_processor.render_flags();
        {
            // Whether or not to re-upload any data to the GPU buffers
            let update_buffers: bool;
            let id_map_entry_mut: &mut IDMapEntry<D::DispData>;

            match self.id_map.entry(widget.id()) {
                Entry::Occupied(mut entry) => {
                    update_buffers = !(widget.num_updates() == entry.get().num_updates);
                    entry.get_mut().num_updates = widget.num_updates();
                    id_map_entry_mut = entry.into_mut();
                }
                Entry::Vacant(entry)   => {
                    update_buffers = true;
                    id_map_entry_mut = entry.insert(IDMapEntry {
                        num_updates: widget.num_updates(),
                        data: Default::default()
                    });
                }
            }
            
            if render_flags.contains(FORCE_UPDATE) || update_buffers {
                self.data_processor.update_data(widget.as_ref(), &mut id_map_entry_mut.data);
            }
        }

        // Unfortunately, we can't just re-use the mutable reference to the id_map_entry, as we also need
        // to borrow the struct owning the entry as immutable. This workaround has a slight runtime cost,
        // so it's in the program's best interest to have this hack removed.
        let id_map_entry = self.id_map.get(&widget.id()).unwrap();
        self.data_processor.render_data(&id_map_entry.data);
    }
}
