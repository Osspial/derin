// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Implements the default OpenGL renderer.
//!
//! Unless you're creating your own widgets, you generally shouldn't have to look at this module.

mod atlas;
mod font_cache;
mod translate;

use std::rc::Rc;
use derin_common_types::cursor::CursorIcon;
use derin_common_types::layout::SizeBounds;

use crate::cgmath::{Array, Bounded, Point2, Vector2, EuclideanSpace};

use gullery::ContextState;
use gullery::framebuffer::render_state::{RenderState, BlendFunc, BlendFuncs};
use gullery::program::{Shader, Program};
use gullery::texture::Texture;
use gullery::framebuffer::{DrawMode, Framebuffer, FramebufferDefault};
use gullery::buffer::{Buffer, BufferUsage};
use gullery::vertex::VertexArrayObject;
use gullery::image_format::Rgba;

use glyphydog::DPI;

use cgmath_geometry::{D2, rect::{BoundBox, OffsetBox, DimsBox, GeoBox}};

use glutin::*;

use crate::theme::Theme;
pub use crate::core::render::{Renderer, RenderFrame};

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
    gl_tex_atlas: Texture<D2, Rgba<u8>>,
    render_state: RenderState,
    fb: FramebufferDefault,
    program: Program<GLVertex, GLUniforms<'static>>,
    vao: VertexArrayObject<GLVertex, !>,
    window_dims: DimsBox<D2, u32>,
    scale_factor: f32
}

#[derive(Vertex, Debug, Clone, Copy)]
struct GLVertex {
    loc: Point2<f32>,
    color: Rgba<u8>,
    tex_coord: Point2<f32>
}

#[derive(Uniforms, Clone, Copy)]
struct GLUniforms<'a> {
    atlas_size: Vector2<u32>,
    window_size: Point2<f32>,
    tex_atlas: &'a Texture<D2, Rgba<u8>>
}

pub trait PrimFrame: RenderFrame<Primitive=ThemedPrim<<Self as PrimFrame>::DirectRender>, Theme=Theme> {
    type DirectRender;
}


impl GLRenderer {
    pub unsafe fn new(events_loop: &EventsLoop, window_builder: WindowBuilder, gen_context_builder: impl Fn() -> ContextBuilder<'static>) -> Result<GLRenderer, CreationError> {
        let show_window = window_builder.window.visible;
        let window = {
            let window_builder_no_show = window_builder.with_visibility(false);
            let versions = [
                ((3, 3), Some(GlProfile::Core)),
                ((3, 3), Some(GlProfile::Compatibility)),
                ((3, 3), None),
                ((3, 2), Some(GlProfile::Core)),
                ((3, 2), Some(GlProfile::Compatibility)),
                ((3, 2), None),
                ((3, 1), Some(GlProfile::Core)),
                ((3, 1), Some(GlProfile::Compatibility)),
                ((3, 1), None)
            ];

            let mut window = None;
            for (version, profile_opt) in versions.iter().cloned() {
                let mut context_builder =
                    gen_context_builder()
                        .with_gl(GlRequest::GlThenGles {
                            opengl_version: version,
                            opengles_version: (3, 0)
                        });
                context_builder.gl_attr.profile = profile_opt;
                window = Some(GlWindow::new(
                    window_builder_no_show.clone(),
                    context_builder,
                    events_loop
                ));
                if let Some(Ok(_)) = window {
                    break;
                }
            }

            window.unwrap()?
        };
        if show_window {
            window.show();
        }

        window.context().make_current().unwrap();
        let context_state = ContextState::new(|f| window.context().get_proc_address(f));

        let vert_shader = Shader::new(VERT_SHADER, context_state.clone()).unwrap();
        let frag_shader = Shader::new(FRAG_SHADER, context_state.clone()).unwrap();

        let program = Program::new(&vert_shader, None, &frag_shader).unwrap().0;

        let gl_tex_atlas = Texture::new(DimsBox::new2(1024, 1024), 1, context_state.clone()).unwrap();

        let mut vertices = vec![
            GLVertex {
                loc: Point2::new(0., 0.),
                color: Rgba::new(0, 0, 0, 0),
                tex_coord: Point2::new(0., 0.)
            };
            2048 * 3
        ];
        let vao = VertexArrayObject::new(Buffer::with_data(BufferUsage::StreamDraw, &vertices, context_state.clone()), None);
        vertices.clear();

        Ok(GLRenderer {
            frame: GLFrame {
                poly_translator: Translator::new(),
                draw: FrameDraw {
                    vertices,
                    atlas: Atlas::new(),
                    font_cache: FontCache::new(),
                    fb: FramebufferDefault::new(context_state.clone()).expect("Could not access default framebuffer"),
                    vao,
                    render_state: RenderState {
                        blend: BlendFuncs {
                            src_rgb: BlendFunc::SrcAlpha,
                            dst_rgb: BlendFunc::OneMinusSrcAlpha,
                            src_alpha: BlendFunc::SrcAlpha,
                            dst_alpha: BlendFunc::OneMinusSrcAlpha
                        },
                        ..RenderState::default()
                    },
                    program,
                    gl_tex_atlas,
                    context_state,
                    window_dims: DimsBox::new2(0, 0),
                    scale_factor: 1.0
                }
            },
            client_size_bounds: SizeBounds::default(),
            window,
        })
    }

    #[inline]
    pub fn window(&self) -> &GlWindow {
        &self.window
    }

    pub fn context_state(&self) -> Rc<ContextState> {
        self.frame.draw.context_state.clone()
    }

    pub(crate) fn set_size_bounds(&mut self, client_size_bounds: SizeBounds) {
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

    pub(crate) fn set_cursor_pos(&mut self, pos: Point2<i32>) {
        self.window.set_cursor_position(pos.x, pos.y).ok();
    }
    pub(crate) fn set_cursor_icon(&mut self, icon: CursorIcon) {
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
}

impl Renderer for GLRenderer {
    type Frame = GLFrame;
    fn resized(&mut self, new_size: DimsBox<D2, u32>) {
        self.window.context().resize(new_size.width(), new_size.height());
    }

    fn dims(&self) -> DimsBox<D2, u32> {
        let (width, height) = self.window.get_inner_size().unwrap();
        DimsBox::new2(width, height)
    }

    fn render(
        &mut self,
        _: &<Self::Frame as RenderFrame>::Theme,
        draw_to_frame: impl FnOnce(&mut Self::Frame)
    ) {
        let (width, height) = self.window.get_inner_size().unwrap();
        let scale_factor = self.window.hidpi_factor();
        self.frame.draw.window_dims = DimsBox::new2(width, height);
        self.frame.draw.scale_factor = scale_factor;
        let width_scaled = (width as f32 * scale_factor) as u32;
        let height_scaled = (height as f32 * scale_factor) as u32;
        self.frame.draw.render_state.viewport = DimsBox::new2(width_scaled, height_scaled).into();
        self.frame.draw.fb.clear_color_all(Rgba::new(1., 1., 1., 1.));
        self.frame.draw.fb.clear_depth(1.0);
        self.frame.draw.fb.clear_stencil(0);

        draw_to_frame(&mut self.frame);

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
            window_size: Point2::from_vec(self.window_dims.dims.cast::<f32>().unwrap_or(Vector2::from_value(f32::max_value()))),
            tex_atlas: &self.gl_tex_atlas
        };

        for verts in self.vertices.chunks(self.vao.vertex_buffer().len()) {
            self.vao.vertex_buffer_mut().sub_data(0, verts);
            self.fb.draw(DrawMode::Triangles, 0..verts.len(), &self.vao, &self.program, uniform, self.render_state);
        }
        self.vertices.clear();
    }
}

impl PrimFrame for GLFrame {
    type DirectRender = (FramebufferDefault, OffsetBox<D2, u32>, Rc<ContextState>);
}

impl RenderFrame for GLFrame {
    type Primitive = ThemedPrim<<Self as PrimFrame>::DirectRender>;
    type Theme = Theme;

    fn upload_primitives<I>(&mut self, theme: &Theme, transform: BoundBox<D2, i32>, clip_rect: BoundBox<D2, i32>, prim_iter: I)
        where I: Iterator<Item=ThemedPrim<<GLFrame as PrimFrame>::DirectRender>>
    {
        let dpi_axis = 72;// (72. * self.draw.scale_factor) as u32;
        self.poly_translator.translate_prims(
            transform,
            clip_rect,
            theme,
            DPI::new(dpi_axis, dpi_axis), // TODO: REPLACE HARDCODED VALUE
            prim_iter,
            &mut self.draw
        );
    }
}

const VERT_SHADER: &str = r#"
    #version 140
    in vec2 loc;
    in vec4 color;
    in vec2 tex_coord;

    uniform uvec2 atlas_size;
    uniform vec2 window_size;

    out vec2 tex_coord_out;
    out vec4 frag_color;

    void main() {
        gl_Position = vec4(vec2(1.0, -1.0) * (vec2(loc) / window_size - 0.5) * 2.0, 1.0, 1.0);
        frag_color = color;
        tex_coord_out = tex_coord / vec2(atlas_size);
    }
"#;

const FRAG_SHADER: &str = r#"
    #version 140
    in vec4 frag_color;
    in vec2 tex_coord_out;

    uniform sampler2D tex_atlas;

    out vec4 out_color;

    void main() {
        out_color = frag_color * texture(tex_atlas, tex_coord_out);
    }
"#;
