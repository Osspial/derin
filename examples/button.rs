extern crate tint;
extern crate glutin;

use tint::draw::{Drawable, Shadable, Shader, Surface, Color, Complex, Rect, Composite};
use tint::draw::primitive::ColorRect;
use tint::draw::gl::{Facade, BufferData};

struct DrawableRect {
    rect: ColorRect,
    buffers: BufferData
}

impl Shadable for DrawableRect {
    type Composite = ();

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

struct CompositeRects {
    rect: Rect,
    front: ColorRect,
    fill: ColorRect,
    buffers: BufferData
}

impl<'a> Composite for &'a CompositeRects {
    type Foreground = &'a ColorRect;
    type Fill = &'a ColorRect;
    type Backdrop = ();

    fn rect(&self) -> Rect {
        self.rect
    }

    fn foreground(&self) -> &'a ColorRect {
        &self.front
    }

    fn fill(&self) -> &'a ColorRect {
        &self.fill
    }

    fn backdrop(&self) -> () {
        ()
    }
}

impl<'b> Shadable for &'b CompositeRects {
    type Composite = &'b CompositeRects;

    fn shader_data<'a>(&'a self) -> Shader<'a, &'b CompositeRects> {
        Shader::Composite {
            rect: self.rect(),
            border: self.border(),
            foreground: self.foreground(),
            fill: self.fill(),
            backdrop: self.backdrop()
        }
    }

    fn num_updates(&self) -> u64 {
        self.front.num_updates() +
        self.fill.num_updates()
    }
}

impl<'a> Drawable for &'a CompositeRects {
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

    let composite = CompositeRects {
        rect: Rect::new(
                Complex::new_rel(-1.0, 1.0),
                Complex::new_rel( 0.8, 0.0)
            ),
        front: ColorRect::new(
                Color::new(255, 0, 0, 255),
                Rect::new(
                    Complex::new(-1.0,  1.0,  12.0, -12.0),
                    Complex::new( 1.0, -1.0, -12.0,  12.0)
                )
            ),
        fill: ColorRect::new(
                Color::new(255, 255, 0, 128),
                Rect::new(
                    Complex::new_rel(-1.0,  1.0),
                    Complex::new_rel( 1.0, -1.0)
                )
            ),
        buffers: BufferData::new()
    };

    'main: loop {
        for event in window.poll_events() {
            use glutin::Event::*;

            match event {
                Closed => break 'main,
                Resized(x, y) => display.resize(x, y),
                _ => ()
            }
        }

        let mut surface = display.surface();
        surface.draw(&rect);
        surface.draw(&&composite);

        window.swap_buffers().unwrap();
    }
}
