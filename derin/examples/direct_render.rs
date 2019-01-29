// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
#![feature(never_type)]

extern crate derin;
extern crate gullery;
#[macro_use]
extern crate gullery_macros;

use derin::{LoopFlow, Window, WindowConfig};
use derin::layout::{Margins, LayoutHorizontal};
use derin::container::SingleContainer;
use derin::widgets::{Group, DirectRender, DirectRenderState};
use derin::geometry::{rect::{DimsBox, OffsetBox}, Point2, D2};

use gullery::ContextState;
use gullery::buffer::*;
use gullery::framebuffer::*;
use gullery::program::*;
use gullery::vertex::*;
use gullery::framebuffer::render_state::*;
use std::rc::Rc;

struct DD(Option<DirectDraw>);

struct DirectDraw {
    vao: VertexArrayObject<Vertex, !>,
    program: Program<Vertex, ()>
}

#[derive(Vertex, Debug, Clone, Copy)]
struct Vertex {
    pos: Point2<f32>
}

fn main() {
    let direct_draw_ui = Group::new(
        SingleContainer::new(DirectRender::new(DD(None), None)),
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_config = WindowConfig {
        dimensions: Some(DimsBox::new2(256, 256)),
        title: "Direct Render Example".to_string(),
        ..WindowConfig::default()
    };

    let mut window = unsafe{ Window::new(window_config, direct_draw_ui, theme).unwrap() };
    let context_state = window.context_state();
    {
        let direct = &mut window.root_mut().container_mut().widget.render_state_mut();
        direct.0 = Some(DirectDraw {
            vao: VertexArrayObject::new(
                Buffer::with_data(
                    BufferUsage::StaticDraw,
                    &[
                        Vertex{ pos: Point2::new(-1., -1.) },
                        Vertex{ pos: Point2::new( 1., -1.) },
                        Vertex{ pos: Point2::new( 0.,  1.) },
                    ],
                    context_state.clone()
                ),
                None
            ),
            program: {
                let vertex_shader = Shader::new(VERTEX_SHADER, context_state.clone()).unwrap();
                let fragment_shader = Shader::new(FRAGMENT_SHADER, context_state.clone()).unwrap();
                Program::new(&vertex_shader, None, &fragment_shader).unwrap().0
            }
        });
    }
    let _: Option<()> = window.run_forever(
        |_: (), _, _| {
            LoopFlow::Continue
        },
        |_, _| None
    );
}

impl<A> DirectRenderState<A> for DD
{
    type RenderType = (DefaultFramebuffer, OffsetBox<D2, u32>, Rc<ContextState>);
    fn render(&mut self, &mut (ref mut fb, viewport_rect, _): &mut Self::RenderType) {
        if let Some(ref draw_params) = self.0 {
            let render_state = RenderState {
                srgb: true,
                viewport: viewport_rect,
                ..RenderState::default()
            };
            fb.draw(DrawMode::Triangles, .., &draw_params.vao, &draw_params.program, (), render_state);
        }
    }
}

const VERTEX_SHADER: &str = r#"
    #version 330
    in vec2 pos;

    void main() {
        gl_Position = vec4(vec3(pos, 1.0), 1.0);
    }
"#;

const FRAGMENT_SHADER: &str = r#"
    #version 330
    out vec4 frag_color;
    void main() {
        frag_color = vec4(1.0, 0.0, 1.0, 1.0);
    }
"#;
