// Copyright 2018 Osspial
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate derin;
extern crate gullery;
#[macro_use]
extern crate gullery_macros;

use derin::{LoopFlow, Window, WindowAttributes};
use derin::layout::{Margins, LayoutHorizontal};
use derin::container::SingleContainer;
use derin::widgets::{Group, DirectRender, DirectRenderState};
use derin::geometry::{OffsetBox, Point2};

use gullery::ContextState;
use gullery::buffers::*;
use gullery::framebuffer::*;
use gullery::program::*;
use gullery::vao::*;
use gullery::render_state::*;
use std::rc::Rc;

struct DD(Option<DirectDraw>);

struct DirectDraw {
    vao: VertexArrayObj<Vertex, ()>,
    program: Program<Vertex, ()>
}

#[derive(TypeGroup, Debug, Clone, Copy)]
struct Vertex {
    pos: Point2<f32>
}

fn main() {
    let direct_draw_ui = Group::new(
        SingleContainer::new(DirectRender::new(DD(None))),
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_attributes = WindowAttributes {
        dimensions: Some((256, 256)),
        title: "Direct Render Example".to_string(),
        ..WindowAttributes::default()
    };

    let mut window = unsafe{ Window::new(window_attributes, direct_draw_ui, theme).unwrap() };
    let context_state = window.context_state();
    {
        let direct = &mut window.root_mut().container_mut().widget.render_state_mut();
        direct.0 = Some(DirectDraw {
            vao: VertexArrayObj::new_noindex(
                Buffer::with_data(
                    BufferUsage::StaticDraw,
                    &[
                        Vertex{ pos: Point2::new(-1., -1.) },
                        Vertex{ pos: Point2::new( 1., -1.) },
                        Vertex{ pos: Point2::new( 0.,  1.) },
                    ],
                    context_state.clone()
                )
            ),
            program: {
                let vertex_shader = Shader::new(VERTEX_SHADER, context_state.clone()).unwrap();
                let fragment_shader = Shader::new(FRAGMENT_SHADER, context_state.clone()).unwrap();
                Program::new(&vertex_shader, None, &fragment_shader).unwrap_discard()
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

impl DirectRenderState for DD
{
    type RenderType = (DefaultFramebuffer, OffsetBox<Point2<u32>>, Rc<ContextState>);
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
