mod atlas;
mod font_cache;
mod translate;

use std::rc::Rc;

use cgmath::{Point2, Vector2, EuclideanSpace};

use gl_raii::ContextState;
use gl_raii::render_state::{RenderState, BlendFunc, BlendFuncs};
use gl_raii::program::{Shader, Program};
use gl_raii::textures::{Dims2D, Texture};
use gl_raii::textures::targets::SimpleTex;
use gl_raii::framebuffer::{DrawMode, Framebuffer, DefaultFramebuffer};
use gl_raii::buffers::{Buffer, BufferUsage};
use gl_raii::vao::VertexArrayObj;
use gl_raii::glsl::Nu8;
use gl_raii::colors::Rgba;

use glyphydog::DPI;

use cgmath_geometry::{BoundRect, DimsRect, Rectangle};

use glutin::{GlWindow, GlContext, EventsLoop, WindowBuilder, ContextBuilder, GlRequest, CreationError};

use theme::Theme;
use core::render::{Renderer, RenderFrame};
use core::tree::NodeIdent;

use self::atlas::Atlas;
use self::font_cache::FontCache;
use self::translate::Translator;
pub use self::translate::{Prim, ThemedPrim, RelPoint};

pub struct GLRenderer {
    window: GlWindow,
    frame: GLFrame,

    // OpenGL structs
    context_state: Rc<ContextState>,
    fb: DefaultFramebuffer,
    program: Program<GLVertex, GLUniforms<'static>>,
    vao: VertexArrayObj<GLVertex, ()>,
    gl_tex_atlas: Texture<Rgba<Nu8>, SimpleTex<Dims2D>>,
    render_state: RenderState
}

pub struct GLFrame {
    // render_queue: RenderQueue,
    poly_translator: Translator,
    vertices: Vec<GLVertex>,
    atlas: Atlas,
    font_cache: FontCache,
}

#[derive(TypeGroup, Debug, Clone, Copy)]
struct GLVertex {
    loc: Point2<u32>,
    color: Rgba<Nu8>,
    tex_coord: Point2<f32>
}

#[derive(Uniforms, Clone, Copy)]
struct GLUniforms<'a> {
    atlas_size: Point2<u32>,
    window_size: Point2<u32>,
    tex_atlas: &'a Texture<Rgba<Nu8>, SimpleTex<Dims2D>>
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

        let program = Program::new(&vert_shader, None, &frag_shader).unwrap_discard();

        let gl_tex_atlas = Texture::new(Dims2D::new(1024, 1024), 1, context_state.clone()).unwrap();

        Ok(GLRenderer {
            frame: GLFrame {
                poly_translator: Translator::new(),
                vertices: Vec::new(),
                atlas: Atlas::new(),
                font_cache: FontCache::new()
            },
            window,


            fb: DefaultFramebuffer::new(context_state.clone()),
            vao: VertexArrayObj::new_noindex(Buffer::with_size(BufferUsage::StreamDraw, 2048 * 3, context_state.clone())),
            render_state: RenderState {
                blend: Some(BlendFuncs {
                    src_rgb: BlendFunc::SrcAlpha,
                    dst_rgb: BlendFunc::OneMinusSrcAlpha,
                    src_alpha: BlendFunc::SrcAlpha,
                    dst_alpha: BlendFunc::OneMinusSrcAlpha
                }),
                ..RenderState::default()
            },
            program,
            gl_tex_atlas,
            context_state
        })
    }
}

impl Renderer for GLRenderer {
    type Frame = GLFrame;
    fn force_full_redraw(&self) -> bool {true}
    fn make_frame(&mut self) -> (&mut GLFrame, BoundRect<u32>) {
        let (width, height) = self.window.get_inner_size().unwrap();
        self.render_state.viewport = DimsRect::new(width, height).into();

        (&mut self.frame, BoundRect::new(0, 0, width, height))
    }

    fn finish_frame(&mut self, _: &Theme) {
        let atlas_dims = Dims2D::new(self.frame.atlas.dims().width(), self.frame.atlas.dims().height());
        if atlas_dims != self.gl_tex_atlas.dims() {
            self.gl_tex_atlas = Texture::new(atlas_dims, 1, self.context_state.clone()).unwrap();
        }
        self.gl_tex_atlas.sub_image(0, Vector2::new(0, 0), atlas_dims, self.frame.atlas.pixels());

        let (window_width, window_height) = self.window.get_inner_size().unwrap();
        let uniform = GLUniforms {
            atlas_size: Point2::new(self.gl_tex_atlas.dims().width, self.gl_tex_atlas.dims().height),
            window_size: Point2::new(window_width, window_height),
            tex_atlas: &self.gl_tex_atlas
        };

        for verts in self.frame.vertices.chunks(self.vao.vertex_buffer().size()) {
            self.vao.vertex_buffer_mut().sub_data(0, verts);
            self.fb.draw(DrawMode::Triangles, 0..verts.len(), &self.vao, &self.program, uniform, self.render_state);
        }
        self.window.swap_buffers().ok();
        self.frame.atlas.bump_frame_count();
        self.frame.vertices.clear();
    }
}

impl RenderFrame for GLFrame {
    type Transform = BoundRect<u32>;
    type Primitive = ThemedPrim;
    type Theme = Theme;

    fn upload_primitives<I>(&mut self, _ident: &[NodeIdent], theme: &Theme, transform: &BoundRect<u32>, prim_iter: I)
        where I: Iterator<Item=ThemedPrim>
    {
        self.poly_translator.translate_prims(
            *transform,
            theme,
            &mut self.atlas,
            &mut self.font_cache,
            DPI::new(72, 72), // TODO: REPLACE HARDCODED VALUE
            prim_iter,
            &mut self.vertices
        );
    }

    #[inline]
    fn child_rect_transform(rect: &BoundRect<u32>, child_rect: BoundRect<u32>) -> BoundRect<u32> {
        let trans = child_rect + rect.min().to_vec();
        trans
    }
}

const VERT_SHADER: &str = r#"
    #version 330
    in uvec2 loc;
    in vec4 color;
    in vec2 tex_coord;

    uniform uvec2 atlas_size;
    uniform uvec2 window_size;

    out vec2 tex_coord_out;
    out vec4 frag_color;

    void main() {
        gl_Position = vec4(vec2(1.0, -1.0) * (vec2(loc) / vec2(window_size) - 0.5) * 2.0, 0.0, 1.0);
        frag_color = color;
        tex_coord_out = (tex_coord) / vec2(atlas_size);
    }
"#;

const FRAG_SHADER: &str = r#"
    #version 330
    in vec4 frag_color;
    in vec2 tex_coord_out;

    uniform sampler2D tex_atlas;

    out vec4 out_color;

    void main() {
        out_color = frag_color * texture(tex_atlas, tex_coord_out);
    }
"#;
