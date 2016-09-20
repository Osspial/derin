extern crate tint;
extern crate glutin;

use tint::draw::*;
use tint::draw::primitive::*;
use tint::draw::gl::{Facade, ShaderDataCollector};
use tint::draw::font::{Font, FontInfo};

struct CompositeRects {
    rect: Rect,
    text: TextBox<&'static str>,
    front: ColorRect
}

impl Shadable for CompositeRects {
    fn shader_data(&self, data: &mut ShaderDataCollector) {
        let mut data_trans = data.push_transform(self.rect);
        self.front.shader_data(&mut data_trans);
        self.text.shader_data(&mut data_trans);
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

    let rect = Widget::new(LinearGradient::new(
            Rect::new(
                Complex::new(-0.5,  0.5, 0.0, 144.0),
                Complex::new_rat( 0.5, -0.5)
            ),
            vec![
                GradientNode::new(-0.5, Color::new(255, 255, 255, 255)),
                GradientNode::new( 0.0, Color::new(0, 255, 0, 255)),
                GradientNode::new( 1.0, Color::new(255, 0, 0, 255)),
            ]
        )
    );

    let composite = Widget::new(CompositeRects {
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
        text: TextBox::new(
                Rect::new(
                    Complex::new(-1.0,  1.0,  12.0, -12.0),
                    Complex::new( 1.0, -1.0, -12.0,  12.0)
                ),
                "Greetings, you glorious bastards. Oh shit, word wrapping works correctly? Cool beans.\nDo new\n\nlines work?",
                Color::new(0, 127, 127, 255),
                font,
                16
            )
    });

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
        surface.draw(&composite);

        window.swap_buffers().unwrap();
    }
}
