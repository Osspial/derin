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
use gl_raii::glsl::{Nu8, Ni8};
use gl_raii::colors::Rgba;

use cgmath_geometry::{BoundRect, DimsRect, OffsetRect, Rectangle};

use glutin::{GlWindow, GlContext, EventsLoop, WindowBuilder, ContextBuilder, GlRequest, CreationError};

use dat::SkylineAtlas;
use std::cmp;
use std::cell::Cell;
use std::collections::HashMap;
use core::tree::{Renderer, RenderFrame, Theme, FrameRectStack};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Border<T> {
    pub left: T,
    pub top: T,
    pub right: T,
    pub bottom: T
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasImage {
    /// Border values used for nine-slicing the underlying image.
    slice_border: Border<u16>,
    image_rect: OffsetRect<u32>
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawImage {
    pub atlas_image: AtlasImage,
    pub rect_frac: BoundRect<f32>,
    pub tint: Rgba<Nu8>,
    pub margins_px: Border<i16>
}

pub struct GLRenderer {
    window: GlWindow,
    frame: GLFrame,
    context_state: Rc<ContextState>,
    fb: DefaultFramebuffer,
    program: Program<GLVertex, GLUniforms<'static>>,
    vao: VertexArrayObj<GLVertex, ()>,
    gl_tex_atlas: Texture<Rgba<Nu8>, SimpleTex<Dims2D>>,
    render_state: RenderState
}

pub struct GLFrame {
    vertices: Vec<GLVertex>,
    window_dims: DimsRect<f32>
}

#[derive(Debug, Clone)]
pub struct IconAtlas {
    icon_atlas: SkylineAtlas<Rgba<Nu8>>,
    icon_rects: HashMap<String, AtlasImage>,
    atlas_updated: Cell<bool>
}

#[derive(TypeGroup, Debug, Clone, Copy)]
struct GLVertex {
    loc: Point2<f32>,
    color: Rgba<Nu8>,
    tex_coord: Point2<u16>,
    tex_subpixel_bias: Point2<Ni8>
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
        let window_size = window.get_inner_size().unwrap();

        let gl_tex_atlas = Texture::new(Dims2D::new(1024, 1024), 1, context_state.clone()).unwrap();

        Ok(GLRenderer {
            frame: GLFrame {
                vertices: Vec::new(),
                window_dims: DimsRect::new(window_size.0 as f32, window_size.1 as f32)
            },
            window,
            fb: DefaultFramebuffer::new(context_state.clone()),
            vao: VertexArrayObj::new_noindex(Buffer::with_size(BufferUsage::StreamDraw, 1024 * 3, context_state.clone())),
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
    fn make_frame(&mut self) -> FrameRectStack<GLFrame> {
        self.frame.vertices.clear();
        let (width, height) = self.window.get_inner_size().unwrap();
        self.render_state.viewport = DimsRect::new(width, height).into();
        self.frame.window_dims = DimsRect::new(width as f32, height as f32);
        FrameRectStack::new(&mut self.frame, BoundRect::new(0, 0, width, height))
    }

    fn finish_frame(&mut self, atlas: &IconAtlas) {
        if atlas.atlas_updated.get() == true {
            atlas.atlas_updated.set(false);
            let atlas_dims = Dims2D::new(atlas.icon_atlas.dims().width(), atlas.icon_atlas.dims().height());
            if atlas_dims != self.gl_tex_atlas.dims() {
                self.gl_tex_atlas = Texture::new(atlas_dims, 1, self.context_state.clone()).unwrap();
            }
            self.gl_tex_atlas.sub_image(0, Vector2::new(0, 0), atlas_dims, atlas.icon_atlas.pixels());
        }

        let (window_width, window_height) = self.window.get_inner_size().unwrap();
        let uniform = GLUniforms {
            atlas_size: Point2::new(self.gl_tex_atlas.dims().width, self.gl_tex_atlas.dims().height),
            window_size: Point2::new(window_width, window_height),
            tex_atlas: &self.gl_tex_atlas
        };
        self.fb.clear_color(Rgba::new(0.0, 0.0, 0.0, 1.0));
        for verts in self.frame.vertices.chunks(self.vao.vertex_buffer().size()) {
            self.vao.vertex_buffer_mut().sub_data(0, verts);
            self.fb.draw(DrawMode::Triangles, 0..verts.len(), &self.vao, &self.program, uniform, self.render_state);
        }
        self.window.swap_buffers().ok();
    }
}

impl RenderFrame for GLFrame {
    type Transform = BoundRect<u32>;
    type Primitive = DrawImage;
    type Theme = IconAtlas;

    fn upload_primitives<I>(&mut self, transform: &BoundRect<u32>, prim_iter: I)
        where I: Iterator<Item=DrawImage>
    {
        let iter_mapped = prim_iter
            .flat_map(|draw_image| {
                let min = Point2 {
                    x: (transform.min().x as f32 + (transform.width() as f32 * draw_image.rect_frac.min().x) + draw_image.margins_px.left as f32),
                    y: (transform.min().y as f32 + (transform.height() as f32 * draw_image.rect_frac.min().y) + draw_image.margins_px.top as f32)
                };
                let max = Point2 {
                    x: (transform.min().x as f32 + (transform.width() as f32 * draw_image.rect_frac.max().x) - draw_image.margins_px.right as f32),
                    y: (transform.min().y as f32 + (transform.height() as f32 * draw_image.rect_frac.max().y) - draw_image.margins_px.bottom as f32)
                };
                let color = draw_image.tint;
                let slice_border = draw_image.atlas_image.slice_border;

                let image_rect = draw_image.atlas_image.image_rect.cast::<u16>().unwrap();

                let tl_out = GLVertex {
                    loc: min,
                    color,
                    tex_coord: image_rect.min(),
                    tex_subpixel_bias: Point2::new(Ni8(63), Ni8(63))
                };
                let tr_out = GLVertex {
                    loc: Point2::new(max.x, min.y),
                    color,
                    tex_coord: Point2::new(image_rect.max().x, image_rect.min().y),
                    tex_subpixel_bias: Point2::new(Ni8(-63), Ni8(63))
                };
                let br_out = GLVertex {
                    loc: max,
                    color,
                    tex_coord: image_rect.max(),
                    tex_subpixel_bias: Point2::new(Ni8(-63), Ni8(-63))
                };
                let bl_out = GLVertex {
                    loc: Point2::new(min.x, max.y),
                    color,
                    tex_coord: Point2::new(image_rect.min().x, image_rect.max().y),
                    tex_subpixel_bias: Point2::new(Ni8(63), Ni8(-63))
                };

                macro_rules! derived_verts {
                    ($base:expr, $sign_x:tt $slice_x:expr, $sign_y:tt $slice_y:expr) => {{
                        [
                            $base,
                            GLVertex {
                                loc: Point2::new($base.loc.x $sign_x $slice_x as f32, $base.loc.y),
                                tex_coord: Point2::new($base.tex_coord.x $sign_x $slice_x, $base.tex_coord.y),
                                tex_subpixel_bias: Point2::new($base.tex_subpixel_bias.x, Ni8(0)),
                                ..$base
                            },
                            GLVertex {
                                loc: Point2::new($base.loc.x $sign_x $slice_x as f32, $base.loc.y $sign_y $slice_y as f32),
                                tex_coord: Point2::new($base.tex_coord.x $sign_x $slice_x, $base.tex_coord.y $sign_y $slice_y),
                                ..$base
                            },
                            GLVertex {
                                loc: Point2::new($base.loc.x, $base.loc.y $sign_y $slice_y as f32),
                                tex_coord: Point2::new($base.tex_coord.x, $base.tex_coord.y $sign_y $slice_y),
                                tex_subpixel_bias: Point2::new(Ni8(0), $base.tex_subpixel_bias.y),
                                ..$base
                            }
                        ]
                    }}
                }

                let tl = derived_verts!(tl_out, +slice_border.left, +slice_border.top);
                let tr = derived_verts!(tr_out, -slice_border.right, +slice_border.top);
                let br = derived_verts!(br_out, -slice_border.right, -slice_border.bottom);
                let bl = derived_verts!(bl_out, +slice_border.left, -slice_border.bottom);

                // // Vertex and index arrangements for indexed rendering
                // let vertices = [
                //     tl[0], tl[1], tl[2], tl[3],
                //     tr[0], tr[1], tr[2], tr[3],
                //     br[0], br[1], br[2], br[3],
                //     bl[0], bl[1], bl[2], bl[3],
                // ];
                // let indices = [
                //     0, 1, 2,
                //     2, 3, 0,
                //         1, 5, 6,
                //         6, 2, 1,
                //     4, 5, 6,
                //     6, 7, 4,
                //         7, 11, 10,
                //         10, 6, 7,
                //     8, 9, 10,
                //     10, 11, 8,
                //         9, 13, 15,
                //         15, 10, 9,
                //     12, 13, 14,
                //     14, 15, 12,
                //         14, 3, 2,
                //         2, 15, 14,
                //     2, 6, 10,
                //     10, 15, 2
                // ];

                let vertices = [
                    tl[0], tl[1], tl[2],
                    tl[2], tl[3], tl[0],

                        tl[1], tr[1], tr[2],
                        tr[3], tl[2], tl[1],

                    tr[0], tr[1], tr[2],
                    tr[2], tr[3], tr[0],

                        tr[3], br[3], br[2],
                        br[2], tr[2], tr[3],

                    br[0], br[1], br[2],
                    br[2], br[3], br[0],

                        br[1], bl[1], bl[2],
                        bl[3], br[2], br[1],

                    bl[0], bl[1], bl[2],
                    bl[2], bl[3], bl[0],

                        tl[3], bl[3], bl[2],
                        bl[2], tl[2], tl[3],

                    tl[2], tr[2], br[2],
                    br[2], bl[2], tl[2],
                ];
                (0..vertices.len()).map(move |i| unsafe{ vertices.get_unchecked(i).clone() })
            });
        self.vertices.extend(iter_mapped);
    }

    #[inline]
    fn child_rect_transform(rect: &BoundRect<u32>, child_rect: BoundRect<u32>) -> BoundRect<u32> {
        let trans = child_rect + rect.min().to_vec();
        trans
    }
}

impl Theme for IconAtlas {
    type Key = str;
    type ThemeValue = AtlasImage;

    /// Get the rectangle of the image associated with the theme in the atlas.
    fn node_theme(&self, key: &str) -> AtlasImage {
        self.icon_rects.get(key).or_else(|| self.icon_rects.get("white")).cloned().unwrap()
    }
}

impl IconAtlas {
    pub fn new() -> IconAtlas {
        let mut atlas = IconAtlas {
            icon_atlas: SkylineAtlas::new(Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(0)), DimsRect::new(1024, 1024)),
            icon_rects: HashMap::new(),
            atlas_updated: Cell::new(true)
        };
        let white_rect = {
            let w = Rgba::new(Nu8(255), Nu8(255), Nu8(255), Nu8(255));
            let b = Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(255));

            [
                b, b, b,
                b, w, b,
                b, b, b
            ]
        };
        atlas.upload_icon(
            "white".to_owned(),
            DimsRect::new(3, 3),
            Border::new(1, 1, 1, 1),
            &white_rect
        );
        atlas
    }

    pub fn upload_icon(&mut self, icon_name: String, icon_dims: DimsRect<u32>, slice_border: Border<u16>, icon_px: &[Rgba<Nu8>]) {
        assert_ne!(0, icon_dims.width());
        assert_ne!(0, icon_dims.height());

        self.atlas_updated.set(true);
        let insert_new: bool;
        match self.icon_rects.get_mut(&icon_name) {
            Some(occ) => {
                let occupied_dims = DimsRect{ dims: occ.image_rect.dims() };
                match occupied_dims.contains(icon_dims.max()) {
                    true => {
                        self.icon_atlas.blit(icon_dims, icon_dims.into(), occ.image_rect.min().to_vec(), icon_px);

                        occ.image_rect = OffsetRect{ origin: occ.image_rect.min(), dims: icon_dims.dims() };
                        occ.slice_border = slice_border;

                        insert_new = false;
                    },
                    false => insert_new = true
                }
            },
            None => insert_new = true
        }

        if insert_new {
            let insert_rect = self.icon_atlas.add_image(icon_dims, icon_dims.into(), &icon_px)
                .or_else(|| {
                    self.icon_atlas.compact(self.icon_rects.iter_mut().map(|(_, i)| &mut i.image_rect));
                    self.icon_atlas.add_image(icon_dims, icon_dims.into(), &icon_px)
                })
                .or_else(|| {
                    let mut expanded_dims = self.icon_atlas.dims();
                    expanded_dims.dims.y = icon_dims.height() + expanded_dims.height() * 2;
                    expanded_dims.dims.x = cmp::max(icon_dims.width(), expanded_dims.width());
                    self.icon_atlas.set_dims(Rgba::new(Nu8(0), Nu8(0), Nu8(0), Nu8(0)), expanded_dims);
                    self.icon_atlas.add_image(icon_dims, icon_dims.into(), &icon_px)
                })
                .unwrap();

            self.icon_rects.insert(icon_name, AtlasImage {
                slice_border,
                image_rect: insert_rect
            });
        }
    }
}

impl<T> Border<T> {
    #[inline]
    pub fn new(left: T, top: T, right: T, bottom: T) -> Border<T> {
        Border{ left, top, right, bottom }
    }
}

const VERT_SHADER: &str = r#"
    #version 330
    in vec2 loc;
    in vec4 color;
    in uvec2 tex_coord;
    in vec2 tex_subpixel_bias;

    uniform uvec2 atlas_size;
    uniform uvec2 window_size;

    out vec2 tex_coord_f32;
    out vec4 frag_color;

    void main() {
        gl_Position = vec4(vec2(1.0, -1.0) * (loc / window_size - 0.5) * 2.0, 0.0, 1.0);
        frag_color = color;
        tex_coord_f32 = (vec2(tex_coord) + tex_subpixel_bias) / vec2(atlas_size);
    }
"#;

const FRAG_SHADER: &str = r#"
    #version 330
    in vec4 frag_color;
    in vec2 tex_coord_f32;

    uniform sampler2D tex_atlas;

    out vec4 out_color;

    void main() {
        out_color = frag_color * texture(tex_atlas, tex_coord_f32);
    }
"#;
