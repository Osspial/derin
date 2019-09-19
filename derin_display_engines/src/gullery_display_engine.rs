use cgmath_geometry::{
    D2 as CD2,
    cgmath::Point2,
    rect::{BoundBox, DimsBox, GeoBox},
};
use crate::{
    Content,
    LayoutContent,
    LayoutResult,
    RenderContent,
    rect_layout::{
        self,
        Rect,
        ImageManager,
        ImageLayout,
        ImageLayoutData,
        TextLayoutData,
        text::{FaceManager, StringLayoutData},
        theme::{Color, ImageId, WidgetStyle},
    },
    rect_to_triangles::VertexRect,
    theme::Theme,
};
use derin_core::{
    render::{DisplayEngine, DisplayEngineLayoutRender},
    widget::{WidgetId, WidgetPathEntry},
};
use derin_common_types::layout::SizeBounds;
use glutin::{
    ContextWrapper, PossiblyCurrent,
    dpi::PhysicalSize,
    window::Window,
};
use gullery::{
    ContextState,
    buffer::{Buffer, BufferUsage},
    framebuffer::{
        DrawMode, Framebuffer, FramebufferDefault,
        render_state::RenderState,
    },
    geometry::D2,
    glsl::{GLInt, GLVec2, GLVec3, GLVec4, GLMat4r4c, Normalized},
    program::{Program, Shader},
    texture::{
        Texture,
        types::ArrayTex
    },
    image_format::{Rgba, SRgba},
    vertex::VertexArrayObject,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    iter,
};

pub trait HasLifetimeIterator<'a, Item> {
    type Iter: 'a + Iterator<Item=Item>;
}
pub trait FaceRasterizer: FaceManager + for<'a> HasLifetimeIterator<'a, Color> {
    fn rasterize(&mut self, image: ImageId) -> Option<(DimsBox<CD2, u32>, <Self as HasLifetimeIterator<'_, Color>>::Iter)>;
}
pub trait ImageRasterizer: ImageManager + for<'a> HasLifetimeIterator<'a, Color> {
    fn rasterize(&mut self, image: ImageId) -> Option<(DimsBox<CD2, u32>, <Self as HasLifetimeIterator<'_, Color>>::Iter)>;
}
pub trait LayeredImageAtlas: 'static + for<'a> HasLifetimeIterator<'a, &'a [Color]> {
    fn add_image(&mut self, image: ImageId, dims: DimsBox<CD2, u32>, pixels: impl Iterator<Item=Color>) -> ImageAtlasCoords;
    fn image_coords(&self, image: ImageId) -> Option<ImageAtlasCoords>;
    fn layers(&self) -> <Self as HasLifetimeIterator<'_, &'_ [Color]>>::Iter;
    fn num_layers(&self) -> u16;
    fn layer_dims(&self) -> DimsBox<CD2, u32>;
    fn clean(&mut self);
    fn updated_since_clean(&self) -> bool;

    fn image_coords_or_else<F, I>(&mut self, image: ImageId, or_else: F) -> ImageAtlasCoords
        where F: FnOnce() -> (DimsBox<CD2, u32>, I),
              I: Iterator<Item=Color>
    {
        match self.image_coords(image) {
            Some(coords) => coords,
            None => {
                let (dims, pixels) = or_else();
                self.add_image(image, dims, pixels)
            }
        }
    }
}

#[derive(Vertex, Debug, Clone, Copy)]
struct Vertex {
    pixel: GLVec2<u16>,
    depth: GLInt<u16, Normalized>,
    texture_coordinate: GLVec2<u16>,
    texture_layer: u16,
    color: Rgba,
}

#[derive(Uniforms, Clone, Copy)]
struct Uniforms<'a> {
    pos_matrix: GLMat4r4c<f32>,
    texture_array: &'a Texture<D2, ArrayTex<SRgba>>,
}

const VERTEX_SHADER: &str = r#"
#version 330
in uvec2 pixel;
in float depth;
in uvec2 texture_coordinate;
in uint texture_layer;
in vec4 color;

uniform mat4 pos_matrix;
uniform sampler2DArray texture_array;

out vec4 s_color;
out vec3 s_texture_coordinate;

void main() {
    gl_Position = pos_matrix * vec4(vec3(pos, depth), 1.0);

    s_color = color;

    vec2 layer_size = vec2(textureSize(texture_array).xy);
    s_texture_coordinate = vec3(vec2(texture_coordinate) / layer_size, texture_layer);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 330
in vec4 s_color;
in vec3 s_texture_coordinate;

uniform sampler2DArray texture_array;

out vec4 frag_color;
void main() {
    frag_color = s_color * texture(texture_array, s_texture_coordinate);
}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageAtlasCoords {
    pub layer: u16,
    pub rect: BoundBox<CD2, u16>,
}

pub struct GulleryDisplayEngine<T, L, F, I>
    where T: 'static + Theme<Style=WidgetStyle>,
          L: LayeredImageAtlas,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    pub theme: T,
    pub atlas: L,
    pub face_rasterizer: F,
    pub image_rasterizer: I,
    window: ContextWrapper<PossiblyCurrent, Window>,
    white_image: ImageId,
    widget_rects: HashMap<WidgetId, Vec<Rect>>,
    cpu_buffers: CpuBuffers,

    context_state: Rc<ContextState>,
    texture_array: Texture<D2, ArrayTex<SRgba>>,
    vao: VertexArrayObject<Vertex, u16>,
    program: Program<Vertex, Uniforms<'static>>,
    framebuffer: FramebufferDefault,
    render_state: RenderState,
}

struct CpuBuffers {
    vertices_index: u16,
    indices_index: u16,
    vertices: Box<[Vertex]>,
    indices: Box<[u16]>,
}

impl CpuBuffers {
    fn new_rect_count(rect_count: u16) -> CpuBuffers {
        let base_size = rect_count as usize * 4;
        // We have 4 vertices for every 6 indices, so allocate
        // 1.5x more indices than vertices.
        let vertices_size = base_size;
        let indices_size = base_size * 3 / 2;
        let default_vertex = Vertex {
            pixel: GLVec2::new(0, 0),
            depth: GLInt::new(0),
            texture_coordinate: GLVec2::new(0, 0),
            texture_layer: 0,
            color: Rgba::new(0, 0, 0, 0),
        };
        CpuBuffers {
            vertices_index: 0,
            indices_index: 0,
            vertices: vec![default_vertex; vertices_size].into_boxed_slice(),
            indices: vec![0; indices_size].into_boxed_slice(),
        }
    }

    #[must_use]
    fn push_rect(&mut self, vertices: [Vertex; 4]) -> bool {
        let indices = VertexRect::indices_clockwise_offset(self.vertices_index);
        let new_vertices_index = self.vertices_index as usize + vertices.len();
        let new_indices_index = self.indices_index as usize + indices.len();
        if new_vertices_index > self.vertices.len() {
            return false;
        }
        self.vertices[self.vertices_index as usize..new_vertices_index].copy_from_slice(&vertices);
        self.indices[self.indices_index as usize..new_indices_index].copy_from_slice(&indices);
        self.vertices_index = new_vertices_index as u16;
        self.indices_index = new_indices_index as u16;

        true
    }

    fn clear(&mut self) {
        self.vertices_index = 0;
        self.indices_index = 0;
    }
}

impl<T, L, F, I> GulleryDisplayEngine<T, L, F, I>
    where T: 'static + Theme<Style=WidgetStyle>,
          L: LayeredImageAtlas,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    pub fn new(
        theme: T,
        atlas: L,
        face_rasterizer: F,
        image_rasterizer: I,
        window: ContextWrapper<PossiblyCurrent, Window>,
    ) -> GulleryDisplayEngine<T, L, F, I>
    {
        let white_image = ImageId::new();
        let widget_rects = HashMap::new();
        let cpu_buffers = CpuBuffers::new_rect_count(256);

        let context_state = unsafe{ ContextState::new(|name| window.get_proc_address(name) ) };
        let layer_dims = atlas.layer_dims().dims;
        let texture_array = Texture::with_mip_count(GLVec3::new(layer_dims.x, layer_dims.y, atlas.num_layers() as _), 1, context_state.clone()).unwrap();
        let vao = VertexArrayObject::new(
            Buffer::with_size(BufferUsage::DynamicDraw, cpu_buffers.vertices.len(), context_state.clone()),
            Some(Buffer::with_size(BufferUsage::DynamicDraw, cpu_buffers.indices.len(), context_state.clone())),
        );
        let (program, warnings) = Program::new(
            &Shader::new(VERTEX_SHADER, context_state.clone()).unwrap(),
            None,
            &Shader::new(FRAGMENT_SHADER, context_state.clone()).unwrap(),
        ).unwrap();
        assert_eq!(&warnings[..], &[][..]);

        let framebuffer = FramebufferDefault::new(context_state.clone()).unwrap();
        let window_size = window.window().inner_size();
        let render_state = RenderState {
            viewport:
                GLVec2::new(0, 0)..=
                GLVec2::new(window_size.width as u32, window_size.height as u32),
            ..RenderState::default()
        };

        GulleryDisplayEngine {
            theme,
            atlas,
            face_rasterizer,
            image_rasterizer,

            window,
            white_image,
            widget_rects,
            cpu_buffers,

            context_state,
            texture_array,
            vao,
            program,
            framebuffer,
            render_state,
        }
    }

    fn dpi(&self) -> u32 {
        (self.window.window().hidpi_factor() * 72.0).round() as u32
    }

    fn pos_matrix(&self) -> GLMat4r4c<f32> {
        let dims = self.dims().dims;
        let x_scale = 2.0 / dims.x as f32;
        let y_scale = -2.0 / dims.y as f32;

        #[cfg_attr(rustfmt, rustfmt_skip)]
        GLMat4r4c {
            x: GLVec4::new(x_scale, 0.0    , 0.0, 0.0),
            y: GLVec4::new(0.0,     y_scale, 0.0, 0.0),
            z: GLVec4::new(0.0,     0.0    , 1.0, 0.0),
            w: GLVec4::new(-0.5,   -0.5    , 0.0, 1.0),
        }
    }
}

impl<T, L, F, I> DisplayEngine for GulleryDisplayEngine<T, L, F, I>
    where T: 'static + Theme<Style=WidgetStyle>,
          L: LayeredImageAtlas,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    fn resized(&mut self, new_size: DimsBox<CD2, u32>) {
        self.window.resize(PhysicalSize::new(new_size.width() as f64, new_size.height() as f64));
    }
    fn dims(&self) -> DimsBox<CD2, u32> {
        let size = self.window.window().inner_size();
        DimsBox::new2(size.width as u32, size.height as u32)
    }
    fn widget_removed(&mut self, widget_id: WidgetId) {
        self.widget_rects.remove(&widget_id);
    }
    fn start_frame(&mut self) {
        self.framebuffer.clear_color_all(Rgba::new(0.0, 0.0, 0.0, 1.0));
        self.atlas.add_image(self.white_image, DimsBox::new2(1, 1), iter::once(Color::new(255, 255, 255, 255)));
    }
    fn finish_frame(&mut self) {
        let pos_matrix = self.pos_matrix();
        let GulleryDisplayEngine {
            ref mut vao,
            program,
            framebuffer,
            cpu_buffers,
            render_state,
            ..
        } = self;

        vao.vertex_buffer_mut().sub_data(cpu_buffers.vertices.len(), &cpu_buffers.vertices);
        vao.index_buffer_mut().as_mut().unwrap().sub_data(cpu_buffers.indices.len(), &cpu_buffers.indices);
        let uniforms = Uniforms {
            pos_matrix,
            texture_array: &self.texture_array,
        };
        framebuffer.draw(
            DrawMode::Triangles,
            ..,
            vao,
            program,
            uniforms,
            render_state,
        );

        cpu_buffers.clear();
        self.window.swap_buffers().unwrap();
    }
}
impl<'d, T, L, F, I> DisplayEngineLayoutRender<'d> for GulleryDisplayEngine<T, L, F, I>
    where T: 'static + Theme<Style=WidgetStyle>,
          L: LayeredImageAtlas,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    type Layout = GulleryLayout<'d, T, F, I>;
    type Renderer = GulleryRenderer<'d, L, F, I>;

    fn layout(
        &'d mut self,
        widget_path: &'d [WidgetPathEntry],
        dims: DimsBox<CD2, i32>
    ) -> Self::Layout {
        GulleryLayout {
            path: widget_path,
            dpi: self.dpi(),
            theme: &mut self.theme,
            face_rasterizer: &mut self.face_rasterizer,
            image_rasterizer: &mut self.image_rasterizer,
            widget_rects: &mut self.widget_rects,
            widget_dims: dims,
        }
    }

    fn render(
        &'d mut self,
        widget_id: WidgetId,
        widget_pos: Point2<i32>,
        clip_rect: BoundBox<CD2, i32>,
        depth: u16,
    ) -> Self::Renderer {
        let pos_matrix = self.pos_matrix();
        GulleryRenderer {
            face_rasterizer: &mut self.face_rasterizer,
            image_rasterizer: &mut self.image_rasterizer,

            cpu_buffers: &mut self.cpu_buffers,
            vao: &mut self.vao,
            program: &self.program,
            framebuffer: &mut self.framebuffer,
            texture_array: &mut self.texture_array,
            render_state: self.render_state.clone(),

            atlas: &mut self.atlas,
            white_image: self.white_image,

            rects: self.widget_rects.get(&widget_id).map(|r| &r[..]).unwrap_or(&[]),
            widget_pos,
            clip_rect,
            depth,

            pos_matrix,
        }
    }
}

pub struct GulleryLayout<'d, T, F, I>
    where T: Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    path: &'d [WidgetPathEntry],
    theme: &'d mut T,
    face_rasterizer: &'d mut F,
    image_rasterizer: &'d mut I,
    widget_rects: &'d mut HashMap<WidgetId, Vec<Rect>>,
    dpi: u32,
    widget_dims: DimsBox<CD2, i32>,
}

pub struct GulleryRenderer<'d, L, F, I>
    where L: LayeredImageAtlas,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    face_rasterizer: &'d mut F,
    image_rasterizer: &'d mut I,

    cpu_buffers: &'d mut CpuBuffers,
    vao: &'d mut VertexArrayObject<Vertex, u16>,
    program: &'d Program<Vertex, Uniforms<'static>>,
    framebuffer: &'d mut FramebufferDefault,
    texture_array: &'d mut Texture<D2, ArrayTex<SRgba>>,
    render_state: RenderState,

    atlas: &'d mut L,
    white_image: ImageId,

    rects: &'d [Rect],
    widget_pos: Point2<i32>,
    clip_rect: BoundBox<CD2, i32>,
    depth: u16,
    pos_matrix: GLMat4r4c<f32>,
}

impl<'d, T, F, I> LayoutContent for GulleryLayout<'d, T, F, I>
    where T: Theme<Style=WidgetStyle>,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    fn layout_content<C: Content>(self, content: &C) -> LayoutResult {
        let widget_id = self.path.last().unwrap().widget_id;

        self.theme.set_widget_content(self.path, content);
        let WidgetStyle {
            background: style_background,
            text: style_text,
            content_margins,
            size_bounds: widget_size_bounds,
        } = self.theme.style(widget_id);
        let text_rect = style_text.margins.apply(self.widget_dims.into());

        let mut image_size_bounds = SizeBounds::default();
        let image_layout_data = try {
            let image_id = style_background?;
            let ImageLayout {
                rescale,
                dims,
                size_bounds,
                margins,
            } = self.image_rasterizer.image_layout(image_id);
            image_size_bounds = size_bounds;

            ImageLayoutData {
                image_id,
                rect: margins.apply(self.widget_dims.into()),
                rescale,
                dims,
            }
        };

        let mut text_size_bounds = SizeBounds::default();
        let string_layout: StringLayoutData;
        let text_layout_data: Option<TextLayoutData> = try {
            let render_string = content.string()?;
            string_layout = StringLayoutData::shape(
                render_string.string,
                self.widget_dims,
                self.dpi,
                style_text.layout,
                self.face_rasterizer
            );

            text_size_bounds.min = string_layout.min_size().unwrap_or(DimsBox::new2(0, 0));

            TextLayoutData {
                string_layout: &string_layout,
                decorations: render_string.decorations,
                render_style: style_text.render,
                offset: render_string.offset,
                clip_rect: text_rect,
            }
        };

        let rect_iter = rect_layout::layout_widget_rects(
            image_layout_data,
            text_layout_data,
            self.face_rasterizer,
        );
        self.widget_rects.entry(widget_id)
            .and_modify(|v| v.clear())
            .or_insert(Vec::new())
            .extend(rect_iter);

        LayoutResult {
            size_bounds: widget_size_bounds
                .union(image_size_bounds)
                .and_then(|sb| sb.union(text_size_bounds))
                .unwrap_or(SizeBounds::default()),
            content_rect: content_margins.apply(self.widget_dims.into()),
        }
    }
}

// impl<'d> LayoutString for GulleryLayout<'d> {
//     fn layout_string<C: Content>(
//         &mut self,
//         content: &C,
//         grapheme_clusters: &mut Vec<GraphemeCluster>
//     ) {
//         self.theme.set_widget_content(self.path, content);
//         unimplemented!()
//     }
// }

impl<'d, L, F, I> RenderContent for GulleryRenderer<'d, L, F, I>
    where L: LayeredImageAtlas,
          F: FaceRasterizer,
          I: ImageRasterizer,
{
    fn render_laid_out_content(self) {
        let GulleryRenderer {
            face_rasterizer,
            image_rasterizer,
            vao,
            program,
            framebuffer,
            texture_array,
            cpu_buffers,
            render_state,
            atlas,
            white_image,
            rects,
            widget_pos,
            clip_rect,
            depth,
            pos_matrix,
        } = self;
        let atlas = RefCell::new(atlas);
        let global_rects = rect_layout::transform_local_to_global_rects(
                rects.iter().cloned(),
                widget_pos,
                clip_rect,
            );
        let vertex_rects = global_rects
            .map(VertexRect::from_rect)
            .map(|r| (
                r.image_id().unwrap_or(white_image),
                r.map_unify(
                    |color_vertex| Vertex {
                        pixel: color_vertex.position.cast::<u16>().unwrap().into(),
                        depth: GLInt::new(depth),
                        // TODO: FIGURE OUT GAMMA CORRECTION
                        color: Rgba::new(
                            color_vertex.color.r,
                            color_vertex.color.g,
                            color_vertex.color.b,
                            color_vertex.color.a
                        ),

                        texture_layer: 0,
                        texture_coordinate: GLVec2::new(0, 0),
                    },
                    |_, texture_vertex| Vertex {
                        pixel: texture_vertex.position.cast::<u16>().unwrap().into(),
                        depth: GLInt::new(depth),
                        color: Rgba::new(255, 255, 255, 255),

                        texture_layer: 0,
                        texture_coordinate: GLVec2::new(0, 0),
                    })))
            .map(|(image_id, vertices)| (
                {
                    let mut atlas = atlas.borrow_mut();
                    macro_rules! add_image {
                        () => {{|(dims, pixels)| atlas.add_image(image_id, dims, pixels)}};
                    }
                    atlas.image_coords(image_id)
                        .or_else(|| image_rasterizer.rasterize(image_id).map(add_image!()))
                        .or_else(|| face_rasterizer.rasterize(image_id).map(add_image!()))
                        .unwrap_or_else(|| add_image!()(error_image()))
                },
                vertices))
            .map(|(ImageAtlasCoords{layer, rect}, [v0, v1, v2, v3])| [
                Vertex {
                    texture_layer: layer,
                    texture_coordinate: GLVec2::new(rect.min.x, rect.min.y),
                    ..v0
                },
                Vertex {
                    texture_layer: layer,
                    texture_coordinate: GLVec2::new(rect.max.x, rect.min.y),
                    ..v1
                },
                Vertex {
                    texture_layer: layer,
                    texture_coordinate: GLVec2::new(rect.min.x, rect.max.y),
                    ..v2
                },
                Vertex {
                    texture_layer: layer,
                    texture_coordinate: GLVec2::new(rect.max.x, rect.max.y),
                    ..v3
                }]);

        for vertices in vertex_rects {
            if !cpu_buffers.push_rect(vertices) {
                let atlas = atlas.borrow_mut();
                if atlas.updated_since_clean() {
                    let layer_dims = atlas.layer_dims();
                    let layer_dims = GLVec3::from(layer_dims.dims.extend(1));
                    for (i, layer) in atlas.layers().enumerate() {
                        let srgb_color = SRgba::from_raw_slice(Color::to_raw_slice(layer));
                        texture_array.sub_image(
                            0,
                            GLVec3::new(0, 0, i as u32),
                            layer_dims,
                            srgb_color
                        );
                    }
                }
                vao.vertex_buffer_mut().sub_data(cpu_buffers.vertices.len(), &cpu_buffers.vertices);
                vao.index_buffer_mut().as_mut().unwrap().sub_data(cpu_buffers.indices.len(), &cpu_buffers.indices);
                let uniforms = Uniforms {
                    pos_matrix,
                    texture_array,
                };
                framebuffer.draw(
                    DrawMode::Triangles,
                    ..,
                    vao,
                    program,
                    uniforms,
                    &render_state,
                );

                cpu_buffers.clear();
                let _ = cpu_buffers.push_rect(vertices);
            }
        }
    }
}

fn error_image() -> (DimsBox<CD2, u32>, impl Iterator<Item=Color>) {
    const BLACK: Color = Color::new(0, 0, 0, 255);
    /// Magenta
    const MAGEN: Color = Color::new(255, 0, 255, 255);
    const PIXELS: &[Color] = &[
        BLACK, BLACK, MAGEN, MAGEN,
        BLACK, BLACK, MAGEN, MAGEN,
        MAGEN, MAGEN, BLACK, BLACK,
        MAGEN, MAGEN, BLACK, BLACK,
    ];
    (DimsBox::new2(2, 2), PIXELS.iter().copied())
}
