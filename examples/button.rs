extern crate tint;
extern crate glutin;

use tint::draw::*;
use tint::draw::primitive::*;
use tint::draw::gl::{Facade, BufferData};
use tint::draw::font::{Font, FontInfo};

struct DrawableRect {
    rect: LinearGradient<Vec<GradientNode>>,
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
    fill: TextBox<&'static str>,
    buffers: BufferData
}

impl<'a> Composite for &'a CompositeRects {
    type Foreground = &'a ColorRect;
    type Fill = &'a TextBox<&'static str>;
    type Backdrop = ();

    fn rect(&self) -> Rect {
        self.rect
    }

    fn foreground(&self) -> &'a ColorRect {
        &self.front
    }

    fn fill(&self) -> &'a TextBox<&'static str> {
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
        .with_depth_buffer(24)
        .build().unwrap();

    unsafe{ window.make_current().unwrap() };

    let mut display = Facade::new(|s| window.get_proc_address(s) as *const _);
    let font = Font::new(&FontInfo {
        regular: "/usr/share/fonts/OTF/NimbusSans-Regular.otf".into(),
        italic: None,
        bold: None,
        bold_italic: None
    });

    let rect = DrawableRect {
        rect: LinearGradient::new(
            Rect::new(
                Complex::new_rat(-0.5,  0.5),
                Complex::new_rat( 0.5, -0.5)
            ),
            vec![
                GradientNode::new(LinearComplex::new_rat( 0.5), Color::new(255, 0, 0, 255)),
                GradientNode::new(LinearComplex::new_rat( 0.0), Color::new(0, 255, 0, 255)),
                GradientNode::new(LinearComplex::new_rat(-0.5), Color::new(255, 255, 255, 255)),
            ]
        ),
        buffers: BufferData::new()
    };

    let composite = CompositeRects {
        rect: Rect::new(
                Complex::new_rat(-1.0, 1.0),
                Complex::new( 0.0, 0.0, 0.0, 0.0)
            ),
        front: ColorRect::new(
                Color::new(255, 0, 0, 255),
                Rect::new(
                    Complex::new(-1.0,  1.0,  12.0, -12.0),
                    Complex::new( 0.0, -1.0, -12.0,  12.0)
                )
            ),
        fill: TextBox::new(
                Rect::new(
                    Complex::new(-1.0,  1.0,  12.0, -12.0),
                    Complex::new( 1.0, -1.0, -12.0,  12.0)
                ),
                "Greetings, you glorious bastards. Oh shit, word wrapping works correctly? Cool beans.\nDo new\n\nlines work?",
                Color::new(0, 127, 127, 255),
                font,
                16
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
