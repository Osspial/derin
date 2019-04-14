// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate derin;
extern crate png;

use derin::{LoopFlow, Window, WindowConfig};
use derin::layout::{Align, Align2, Margins, SizeBounds, LayoutHorizontal};
use derin::container::SingleContainer;
use derin::widgets::{Content, Group, Label};
use derin::theme::{ThemeWidget, Image, RescaleRules};
use derin::theme::color::Rgba;
use derin::geometry::rect::DimsBox;

use std::rc::Rc;

fn main() {
    let group = Group::new(
        SingleContainer::new(Label::new(Content::Image("AddIcon".to_string()))),
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let mut theme = derin::theme::Theme::default();
    theme.insert_widget(
        "AddIcon".to_string(),
        ThemeWidget {
            text: None,
            image: Some(Rc::new(Image {
                pixels: {
                    let image_png = png::Decoder::new(::std::io::Cursor::new(&include_bytes!("plus_icon.png")[..]));
                    let (info, mut reader) = image_png.read_info().unwrap();
                    // Allocate the output buffer.
                    let mut image = vec![0; info.buffer_size()];
                    reader.next_frame(&mut image).unwrap();
                    Rgba::slice_from_raw(&image).to_vec()
                },
                dims: DimsBox::new2(32, 32),
                rescale: RescaleRules::Align(Align2::new(Align::Center, Align::Center)),
                size_bounds: SizeBounds::default()
            }))
        }
    );

    let window_config = WindowConfig {
        dimensions: Some(DimsBox::new2(64, 64)),
        title: "Custom Icon".to_string(),
        ..WindowConfig::default()
    };

    let mut window = unsafe{ Window::new(window_config, group, theme).unwrap() };
    window.run_forever(
        |_: (), _, _| {
            LoopFlow::Continue
        },
        |_, _| None
    );
}
