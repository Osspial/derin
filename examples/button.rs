extern crate tint;
extern crate glutin;

use tint::draw::{Drawable, Shadable, Shader, Surface, Color, Complex, Point, Rect};
use tint::draw::primitive::ColorRect;
use tint::draw::gl::{Facade, BufferData};

struct DrawableRect {
    rect: ColorRect,
    buffers: BufferData
}

impl Shadable for DrawableRect {
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        self.rect.shader_data()
    }

    fn num_updates(&self) -> u64 {
        self.rect.num_updates()
    }
}

impl Drawable for DrawableRect {
    fn buffer_data(&self) -> &BufferData {
        &self.buffers
    }
}

fn main() {
    let window = glutin::WindowBuilder::new()
        .with_dimensions(500, 500)
        .with_pixel_format(24, 8)
        .build().unwrap();

    unsafe{ window.make_current().unwrap() };

    let mut display = Facade::new(|s| window.get_proc_address(s) as *const _);

    let rect = DrawableRect {
        rect: ColorRect::new(
            Color::new(255, 255, 255, 255), 
            Rect::new(
                Complex::new_rel(-0.5,  0.5),
                Complex::new_rel( 0.5, -0.5)
            )
        ),
        buffers: BufferData::new()
    };

    'main: loop {
        for event in window.poll_events() {
            match event {
                glutin::Event::Closed => break 'main,
                _ => ()
            }
        }

        let mut surface = display.surface();
        surface.draw(&rect);

        window.swap_buffers().unwrap();
    }
}
