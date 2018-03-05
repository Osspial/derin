mod atlas;
mod font_cache;
mod translate;

use std::rc::Rc;
use dct::cursor::CursorIcon;
use dct::layout::SizeBounds;

use cgmath::{Bounded, Point2, Vector2, EuclideanSpace};

use gullery::ContextState;
use gullery::render_state::{RenderState, BlendFunc, BlendFuncs};
use gullery::program::{Shader, Program};
use gullery::textures::Texture;
use gullery::textures::targets::SimpleTex;
use gullery::framebuffer::{DrawMode, Framebuffer, DefaultFramebuffer};
use gullery::buffers::{Buffer, BufferUsage};
use gullery::vao::VertexArrayObj;
use gullery::glsl::Nu8;
use gullery::colors::Rgba;

use glyphydog::DPI;

use cgmath_geometry::{BoundBox, OffsetBox, DimsBox, GeoBox};

use glutin::*;

use theme::Theme;
pub use core::render::{Renderer, RenderFrame};
use core::tree::WidgetIdent;

use self::atlas::Atlas;
use self::font_cache::FontCache;
use self::translate::Translator;
pub use self::translate::{EditString, Prim, ThemedPrim, RelPoint, RenderString};

pub struct GLRenderer {
    window: GlWindow,
    client_size_bounds: SizeBounds,
    frame: GLFrame,
}

pub struct GLFrame {
    poly_translator: Translator,
    draw: FrameDraw
}

struct FrameDraw {
    vertices: Vec<GLVertex>,
    atlas: Atlas,
    font_cache: FontCache,

    // OpenGL structs
    context_state: Rc<ContextState>,
    gl_tex_atlas: Texture<Rgba<Nu8>, SimpleTex<DimsBox<Point2<u32>>>>,
    render_state: RenderState,
    fb: DefaultFramebuffer,
    program: Program<GLVertex, GLUniforms<'static>>,
    vao: VertexArrayObj<GLVertex, ()>,
}

#[derive(TypeGroup, Debug, Clone, Copy)]
struct GLVertex {
    loc: Point2<i32>,
    color: Rgba<Nu8>,
    tex_coord: Point2<f32>
}

#[derive(Uniforms, Clone, Copy)]
struct GLUniforms<'a> {
    atlas_size: Vector2<u32>,
    window_size: Point2<u32>,
    tex_atlas: &'a Texture<Rgba<Nu8>, SimpleTex<DimsBox<Point2<u32>>>>
}

pub trait PrimFrame: RenderFrame<Primitive=ThemedPrim<<Self as PrimFrame>::DirectRender>, Theme=Theme> {
    type DirectRender;
}


impl GLRenderer {
    pub unsafe fn new(events_loop: &EventsLoop, window_builder: WindowBuilder) -> Result<GLRenderer, CreationError> {
        let window = {
            let context_builder = ContextBuilder::new()
                // .with_gl_profile(GlProfile::Core)
                .with_gl(GlRequest::GlThenGles {
                    opengl_version: (3, 1),
                    opengles_version: (3, 0)
                });
            GlWindow::new(window_builder, context_builder, events_loop)?
        };

        window.context().make_current().unwrap();
        let context_state = ContextState::new(|f| window.context().get_proc_address(f));

        let vert_shader = Shader::new(VERT_SHADER, context_state.clone()).unwrap();
        let frag_shader = Shader::new(FRAG_SHADER, context_state.clone()).unwrap();

        let program = Program::new(&vert_shader, None, &frag_shader).unwrap_werr();

        let gl_tex_atlas = Texture::new(DimsBox::new2(1024, 1024), 1, context_state.clone()).unwrap();

        Ok(GLRenderer {
            frame: GLFrame {
                poly_translator: Translator::new(),
                draw: FrameDraw {
                    vertices: Vec::new(),
                    atlas: Atlas::new(),
                    font_cache: FontCache::new(),
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
                }
            },
            client_size_bounds: SizeBounds::default(),
            window,
        })
    }

    pub fn dims(&self) -> DimsBox<Point2<u32>> {
        let (width, height) = self.window.get_inner_size().unwrap();
        DimsBox::new2(width, height)
    }

    #[inline]
    pub fn window(&self) -> &GlWindow {
        &self.window
    }

    pub fn context_state(&self) -> Rc<ContextState> {
        self.frame.draw.context_state.clone()
    }
}

impl Renderer for GLRenderer {
    type Frame = GLFrame;
    fn force_full_redraw(&self) -> bool {true}

    fn set_cursor_pos(&mut self, pos: Point2<i32>) {
        self.window.set_cursor_position(pos.x, pos.y).ok();
    }
    fn set_cursor_icon(&mut self, icon: CursorIcon) {
        let glutin_icon = match icon {
            CursorIcon::Pointer => MouseCursor::Default,
            CursorIcon::Wait => MouseCursor::Wait,
            CursorIcon::Crosshair => MouseCursor::Crosshair,
            CursorIcon::Hand => MouseCursor::Hand,
            CursorIcon::NotAllowed => MouseCursor::NotAllowed,
            CursorIcon::Text => MouseCursor::Text,
            CursorIcon::Move => MouseCursor::Move,
            CursorIcon::SizeNS => MouseCursor::NsResize,
            CursorIcon::SizeWE => MouseCursor::EwResize,
            CursorIcon::SizeNeSw => MouseCursor::NeswResize,
            CursorIcon::SizeNwSe => MouseCursor::NwseResize,
            CursorIcon::SizeAll => MouseCursor::AllScroll,
            CursorIcon::Hide => {
                self.window.set_cursor_state(CursorState::Hide).ok();
                return;
            }
        };
        self.window.set_cursor_state(CursorState::Normal).ok();
        self.window.set_cursor(glutin_icon);
    }

    fn set_size_bounds(&mut self, client_size_bounds: SizeBounds) {
        if client_size_bounds != self.client_size_bounds {
            self.client_size_bounds = client_size_bounds;
            let outer_rect = self.window.get_outer_size().unwrap();
            let inner_rect = self.window.get_inner_size().unwrap();
            let x_expand = outer_rect.0 - inner_rect.0;
            let y_expand = outer_rect.1 - inner_rect.1;

            let min_dimensions = match client_size_bounds.min == DimsBox::new2(0, 0) {
                true => None,
                false => Some((client_size_bounds.min.width().max(0) as u32 + x_expand, client_size_bounds.min.height().max(0) as u32 + y_expand))
            };
            let max_dimensions = match client_size_bounds.max == DimsBox::max_value() {
                true => None,
                false => Some((client_size_bounds.max.width() as u32 + x_expand, client_size_bounds.max.height() as u32 + y_expand))
            };
            self.window.set_min_dimensions(min_dimensions);
            self.window.set_max_dimensions(max_dimensions);
        }
    }

    fn make_frame(&mut self) -> (&mut GLFrame, BoundBox<Point2<i32>>) {
        let (width, height) = self.window.get_inner_size().unwrap();
        self.frame.draw.render_state.viewport = DimsBox::new2(width, height).into();

        (&mut self.frame, BoundBox::new2(0, 0, width as i32, height as i32))
    }

    fn finish_frame(&mut self, _: &Theme) {
        self.frame.draw.draw_contents();
        self.window.swap_buffers().unwrap();
        self.frame.draw.atlas.bump_frame_count();
    }
}

impl FrameDraw {
    fn draw_contents(&mut self) {
        let atlas_dims = self.atlas.dims();
        if atlas_dims != self.gl_tex_atlas.dims() {
            self.gl_tex_atlas = Texture::new(atlas_dims, 1, self.context_state.clone()).unwrap();
        }
        self.gl_tex_atlas.sub_image(0, Vector2::new(0, 0), atlas_dims, self.atlas.pixels());

        let uniform = GLUniforms {
            atlas_size: self.gl_tex_atlas.dims().dims,
            window_size: Point2::from_vec(self.render_state.viewport.dims()),
            tex_atlas: &self.gl_tex_atlas
        };

        for verts in self.vertices.chunks(self.vao.vertex_buffer().size()) {
            self.vao.vertex_buffer_mut().sub_data(0, verts);
            self.fb.draw(DrawMode::Triangles, 0..verts.len(), &self.vao, &self.program, uniform, self.render_state);
        }
        self.vertices.clear();
    }
}

impl PrimFrame for GLFrame {
    type DirectRender = (DefaultFramebuffer, OffsetBox<Point2<u32>>, Rc<ContextState>);
}

impl RenderFrame for GLFrame {
    type Transform = BoundBox<Point2<i32>>;
    type Primitive = ThemedPrim<<Self as PrimFrame>::DirectRender>;
    type Theme = Theme;

    fn upload_primitives<I>(&mut self, _ident: &[WidgetIdent], theme: &Theme, transform: &BoundBox<Point2<i32>>, prim_iter: I)
        where I: Iterator<Item=ThemedPrim<<GLFrame as PrimFrame>::DirectRender>>
    {
        self.poly_translator.translate_prims(
            *transform,
            theme,
            DPI::new(72, 72), // TODO: REPLACE HARDCODED VALUE
            prim_iter,
            &mut self.draw
        );
    }

    #[inline]
    fn child_rect_transform(rect: &BoundBox<Point2<i32>>, child_rect: BoundBox<Point2<i32>>) -> BoundBox<Point2<i32>> {
        let trans = child_rect + rect.min().to_vec();
        trans
    }
}

const VERT_SHADER: &str = r#"
    #version 330
    in ivec2 loc;
    in vec4 color;
    in vec2 tex_coord;

    uniform uvec2 atlas_size;
    uniform uvec2 window_size;

    out vec2 tex_coord_out;
    out vec4 frag_color;

    void main() {
        gl_Position = vec4(vec2(1.0, -1.0) * (vec2(loc) / vec2(window_size) - 0.5) * 2.0, 1.0, 1.0);
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
