extern crate derin;
#[macro_use]
extern crate derin_macros;
extern crate glutin;
extern crate gullery;

use derin::dct::hints::Margins;
use derin::{SingleContainer, Group, LayoutHorizontal, DirectRender, DirectRenderState};
use derin::core::LoopFlow;
use derin::geometry::{BoundBox, OffsetBox, Point2, Matrix3, GeoBox};

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
    program: Program<Vertex, Uniforms>
}

#[derive(TypeGroup, Debug, Clone, Copy)]
struct Vertex {
    pos: Point2<f32>
}

#[derive(Uniforms, Clone, Copy)]
struct Uniforms {
    transform_matrix: Matrix3<f32>
}

fn main() {
    let direct_draw_ui = Group::new(
        SingleContainer::new(DirectRender::new(DD(None))),
        LayoutHorizontal::new(Margins::new(8, 8, 8, 8), Default::default())
    );
    let theme = derin::theme::Theme::default();

    let window_builder = glutin::WindowBuilder::new()
        .with_dimensions(256, 256)
        .with_title("Direct Render Example");

    let mut window = unsafe{ derin::glutin_window::GlutinWindow::new(window_builder, direct_draw_ui, theme).unwrap() };
    let context_state = window.context_state();
    {
        let direct = &mut window.root_mut().container_mut().node.render_state_mut();
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
    type RenderType = (DefaultFramebuffer, BoundBox<Point2<f32>>, OffsetBox<Point2<u32>>, Rc<ContextState>);
    fn render(&self, &mut (ref mut fb, draw_box, viewport_rect, _): &mut Self::RenderType) {
        if let Some(ref draw_params) = self.0 {
            let render_state = RenderState {
                srgb: true,
                viewport: viewport_rect,
                ..RenderState::default()
            };
            let uniform = Uniforms {
                transform_matrix: Matrix3::new(
                    draw_box.width(), 0.0, draw_box.min().x,
                    0.0, draw_box.height(), draw_box.min().y,
                    0.0, 0.0, 0.0
                )
            };
            fb.draw(DrawMode::Triangles, .., &draw_params.vao, &draw_params.program, uniform, render_state);
        }
    }
}

const VERTEX_SHADER: &str = r#"
    #version 330
    in vec2 pos;
    uniform mat3 transform_matrix;

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
