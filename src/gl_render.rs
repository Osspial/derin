use std::rc::Rc;

use cgmath::{Point2, Vector2, EuclideanSpace};

use gl_raii::ContextState;
use gl_raii::render_state::RenderState;
use gl_raii::program::{Shader, Program};
use gl_raii::framebuffer::{DrawMode, Framebuffer, DefaultFramebuffer};
use gl_raii::buffers::{Buffer, BufferUsage};
use gl_raii::vao::VertexArrayObj;
use gl_raii::glsl::{Nu8, Nu16, Nu32};
use gl_raii::colors::Rgba;

use cgmath_geometry::{BoundRect, DimsRect, Rectangle};

use glutin::{GlWindow, GlContext, EventsLoop, WindowBuilder, ContextBuilder, GlRequest, CreationError};

use derin_core::tree::{Renderer, RenderFrame, FrameRectStack};

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub loc: Point2<i16>,
    pub offset: Vector2<Nu16>,
    pub color: Rgba<Nu8>
}

pub struct GLRenderer {
    window: GlWindow,
    frame: GLFrame,
    context_state: Rc<ContextState>,
    fb: DefaultFramebuffer,
    program: Program<GLVertex, ()>,
    vao: VertexArrayObj<GLVertex, ()>,
    render_state: RenderState
}

pub struct GLFrame {
    vertices: Vec<GLVertex>,
    window_dims: DimsRect<f32>,
}

#[repr(C)]
#[derive(TypeGroup, Debug, Clone, Copy)]
struct GLVertex {
    loc: Point2<f32>,
    color: Rgba<Nu8>
}

impl GLRenderer {
    pub unsafe fn new(events_loop: &EventsLoop, window_builder: WindowBuilder) -> Result<GLRenderer, CreationError> {
        let window = {
            let context_builder = ContextBuilder::new()
                .with_gl(GlRequest::GlThenGles {
                    opengl_version: (3, 3),
                    opengles_version: (3, 0)
                });
            GlWindow::new(window_builder, context_builder, events_loop)?
        };

        window.context().make_current().unwrap();
        let context_state = ContextState::new(|f| window.context().get_proc_address(f));

        let vert_shader = Shader::new(VERT_SHADER, context_state.clone()).unwrap();
        let frag_shader = Shader::new(FRAG_SHADER, context_state.clone()).unwrap();

        let program = Program::new(&vert_shader, None, &frag_shader).unwrap_werr();
        let window_size = window.get_inner_size().unwrap();

        Ok(GLRenderer {
            frame: GLFrame {
                vertices: Vec::new(),
                window_dims: DimsRect::new(window_size.0 as f32, window_size.1 as f32)
            },
            window,
            fb: DefaultFramebuffer::new(context_state.clone()),
            vao: VertexArrayObj::new_noindex(Buffer::with_size(BufferUsage::StreamDraw, 128 * 3, context_state.clone())),
            render_state: RenderState::default(),
            program,
            context_state
        })
    }
}

impl Renderer for GLRenderer {
    type Frame = GLFrame;
    fn make_frame(&mut self) -> FrameRectStack<GLFrame> {
        self.frame.vertices.clear();
        let (width, height) = self.window.get_inner_size().unwrap();
        self.render_state.viewport = DimsRect::new(width, height).into();
        self.frame.window_dims = DimsRect::new(width as f32, height as f32);
        FrameRectStack::new(&mut self.frame, BoundRect::new(0, 0, width, height))
    }

    fn finish_frame(&mut self) {
        for verts in self.frame.vertices.chunks(self.vao.vertex_buffer().size()) {
            self.vao.vertex_buffer_mut().sub_data(0, verts);
            self.fb.draw(DrawMode::Triangles, 0..verts.len(), &self.vao, &self.program, (), self.render_state);
        }
        self.window.swap_buffers().ok();
    }
}

impl RenderFrame for GLFrame {
    type Transform = BoundRect<u32>;
    type Primitive = [Vertex; 3];
    fn upload_primitives<I>(&mut self, transform: &BoundRect<u32>, prim_iter: I)
        where I: Iterator<Item=[Vertex; 3]>
    {
        let window_dims = self.window_dims;
        let iter_mapped = prim_iter.flat_map(|v| (0..v.len()).map(move |i| unsafe{ *v.get_unchecked(i) }))
            .map(|v| GLVertex {
                loc: {
                    let offset_nu32 = v.offset.cast::<Nu32>().unwrap();
                    let offset_px = Vector2 {
                        x: transform.width() * offset_nu32.x + transform.min().x,
                        y: transform.height() * offset_nu32.y + transform.min().y,
                    };
                    Point2 {
                        x: (v.loc.x as i32 + offset_px.x as i32) as f32 / window_dims.width(),
                        y: (v.loc.y as i32 + offset_px.y as i32) as f32 / window_dims.height()
                    }
                },
                color: v.color
            });
        self.vertices.extend(iter_mapped);
    }

    #[inline]
    fn child_rect_transform(rect: &BoundRect<u32>, child_rect: BoundRect<u32>) -> BoundRect<u32> {
        let trans = child_rect + rect.min().to_vec();
        trans
    }
}

impl Vertex {
    #[inline]
    pub fn new(loc: Point2<i16>, offset: Vector2<Nu16>, color: Rgba<Nu8>) -> Vertex {
        Vertex{ loc, offset, color }
    }
}

const VERT_SHADER: &str = r#"
    #version 330
    in vec2 loc;
    in vec4 color;

    out vec4 frag_color;

    void main() {
        gl_Position = vec4(vec2(1.0, -1.0) * (loc - 0.5) * 2.0, 0.0, 1.0);
        frag_color = color;
    }
"#;

const FRAG_SHADER: &str = r#"
    #version 330
    in vec4 frag_color;

    out vec4 out_color;

    void main() {
        out_color = frag_color;
    }
"#;
