use super::{Complex, Point, Color, ColorVert, Shadable, Surface, Rect, Widget};
use super::font::{Font, CharVert};

use std::collections::HashMap;
use std::os::raw::c_void;
use std::hash::BuildHasherDefault;
use std::mem;

use fnv::FnvHasher;

use gl;
use gl::types::*;
use gl_raii::*;

use cgmath::{Matrix3, Vector2, Vector3};
use cgmath::prelude::*;

type HasherType = BuildHasherDefault<FnvHasher>;

static mut ID_COUNTER: u64 = 0;

pub fn get_unique_id() -> u64 {
    let id = unsafe{ ID_COUNTER };
    unsafe{ ID_COUNTER += 1 };
    id
}

pub struct BufferData {
    id: u64,
    verts: GLVertexBuffer<ColorVertGpu>,
    vert_indices: GLIndexBuffer,
    verts_vao: GLVertexArray<ColorVertGpu>,
    chars: GLVertexBuffer<CharVertDepth>,
    chars_vao: GLVertexArray<CharVertDepth>
}

impl BufferData {
    pub fn new() -> BufferData {
        let verts = GLVertexBuffer::new(0, BufferUsage::Static);
        let vert_indices = GLIndexBuffer::new(0, BufferUsage::Static);
        let verts_vao = GLVertexArray::new(&verts, Some(&vert_indices));

        let chars = GLVertexBuffer::new(0, BufferUsage::Dynamic);
        let chars_vao = GLVertexArray::new(&chars, None);

        BufferData {
            id: get_unique_id(),
            verts: verts,
            vert_indices: vert_indices,
            verts_vao: verts_vao,
            chars: chars,
            chars_vao: chars_vao
        }
    }
}

impl Complex {
    fn mul_matrix(self, pts_rat_scale: Vector2<f32>, matrix: Matrix3<f32>) -> Vector3<f32> {
        let pure_rat = Vector2::new(self.rat.x, self.rat.y) +
                       Vector2::new(pts_rat_scale.x * self.pts.x, pts_rat_scale.y * self.pts.y);

        matrix * pure_rat.extend(1.0)
    }
}

/// The GPU representation of a `ColorVert`. The reason this exists is that it has the matrix
/// already performed, allowing us to batch the draw calls of multiple objects.
#[repr(packed)]
#[derive(Debug, Clone, Copy)]
struct ColorVertGpu {
    pos: Vector3<f32>,
    color: Color
}

impl ColorVert {
    fn to_gpu(self, depth: i16, pts_rat_scale: Vector2<f32>, matrix: Matrix3<f32>) -> ColorVertGpu {
        ColorVertGpu {
            pos: Vector3 {
                z: depth as f32 / i16::max_value() as f32,
                ..self.pos.mul_matrix(pts_rat_scale, matrix)
            },
            color: self.color
        }
    }
}

unsafe impl GLVertex for ColorVertGpu {
    unsafe fn vertex_attrib_data() -> &'static [VertexAttribData] {
        const VAD: &'static [VertexAttribData] = &[
            // Position
            VertexAttribData {
                index: 0,
                glsl_type: GLSLType::Vec3(GLPrim::Float),
                offset: 0
            },

            // Color
            VertexAttribData {
                index: 1,
                glsl_type: GLSLType::Vec4(GLPrim::NUByte),
                offset: 12
            }
        ];

        VAD
    }
}

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
struct CharVertDepth {
    char_vert: CharVert,
    depth: f32
}

impl CharVert {
    fn with_depth(self, depth: i16) -> CharVertDepth {
        CharVertDepth {
            char_vert: self,
            depth: depth as f32 / i16::max_value() as f32
        }
    }
}

unsafe impl GLVertex for CharVertDepth {
    unsafe fn vertex_attrib_data() -> &'static [VertexAttribData] {
        const VAD: &'static [VertexAttribData] = &[
            // Rect upper left
            VertexAttribData {
                index: 0,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 0
            },

            // Rect lower right
            VertexAttribData {
                index: 1,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 8
            },

            // Offset
            VertexAttribData {
                index: 2,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 16
            },

            // Size
            VertexAttribData {
                index: 3,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 24
            },

            // Depth
            VertexAttribData {
                index: 4,
                glsl_type: GLSLType::Single(GLPrim::Float),
                offset: 32
            }
        ];

        VAD
    }
}

struct ColorVertexProgram {
    program: GLProgram
}

impl ColorVertexProgram {
    fn new() -> ColorVertexProgram {
        use std::fs::File;
        use std::io::Read;

        let mut colored_vertex_string = String::new();
        File::open("./src/shaders/colored_vertex.vert").unwrap().read_to_string(&mut colored_vertex_string).unwrap();
        let colored_vertex_vert = GLShader::new(ShaderType::Vertex, &colored_vertex_string).unwrap();

        let mut color_passthrough_string = String::new();
        File::open("./src/shaders/color_passthrough.frag").unwrap().read_to_string(&mut color_passthrough_string).unwrap();
        let color_passthrough_frag = GLShader::new(ShaderType::Fragment, &color_passthrough_string).unwrap();

        let program = GLProgram::new(&colored_vertex_vert, &color_passthrough_frag).unwrap();

        ColorVertexProgram {
            program: program
        }
    }
}

struct CharVertexProgram {
    program: GLProgram,
    base_location_uniform: GLint,
    viewport_size_px_uniform: GLint,
    color_uniform: GLint,

    // font_image_uniform: GLint,
    font_image_tex_unit: GLint
}

impl CharVertexProgram {
    fn new() -> CharVertexProgram {
        use std::fs::File;
        use std::io::Read;

        let mut char_vertex_vert_string = String::new();
        File::open("./src/shaders/char_vertex.vert").unwrap().read_to_string(&mut char_vertex_vert_string).unwrap();
        let char_vertex_vert = GLShader::new(ShaderType::Vertex, &char_vertex_vert_string).unwrap();

        let mut char_vertex_geom_string = String::new();
        File::open("./src/shaders/char_vertex.geom").unwrap().read_to_string(&mut char_vertex_geom_string).unwrap();
        let char_vertex_geom = GLShader::new(ShaderType::Geometry, &char_vertex_geom_string).unwrap();

        let mut char_vertex_frag_string = String::new();
        File::open("./src/shaders/char_vertex.frag").unwrap().read_to_string(&mut char_vertex_frag_string).unwrap();
        let char_vertex_frag = GLShader::new(ShaderType::Fragment, &char_vertex_frag_string).unwrap();

        let program = GLProgram::new_geometry(&char_vertex_vert, &char_vertex_geom, &char_vertex_frag).unwrap();

        let base_location_uniform = unsafe{ gl::GetUniformLocation(program.handle, "base_location\0".as_ptr() as *const GLchar) };
        let viewport_size_px_uniform = unsafe{ gl::GetUniformLocation(program.handle, "viewport_size_px\0".as_ptr() as *const GLchar) };
        let color_uniform = unsafe{ gl::GetUniformLocation(program.handle, "color\0".as_ptr() as *const GLchar) };

        let font_image_uniform = unsafe{ gl::GetUniformLocation(program.handle, "tex\0".as_ptr() as *const GLchar) };
        let font_image_tex_unit = 1;

        // Set the font image uniform to use the proper texture unit
        program.with(|_| unsafe {
            gl::Uniform1i(font_image_uniform, font_image_tex_unit);
        });

        CharVertexProgram {
            program: program,
            base_location_uniform: base_location_uniform,
            viewport_size_px_uniform: viewport_size_px_uniform,
            color_uniform: color_uniform,

            // font_image_uniform: font_image_uniform,
            font_image_tex_unit: font_image_tex_unit
        }
    }
}

struct IDMapEntry {
    num_updates: u64,
    render_data_vec: Vec<RenderData>,

    mask_verts: Vec<ColorVertGpu>,
    mask_indices: Vec<u16>,
    mask_offset: GLint,
    mask_base_vertex: GLint,
    depth_offset: f64
}

pub struct Facade {
    pub dpi: u32,
    id_map: HashMap<u64, IDMapEntry, HasherType>,
    font_id_map: HashMap<u64, GLTexture, HasherType>,

    // The rendering programs
    color_passthrough: ColorVertexProgram,
    char_vertex: CharVertexProgram,

    sampler: GLSampler,

    viewport_size: (GLint, GLint),
    viewport_size_changed: bool
}

impl Facade {
    pub fn new<F: Fn(&str) -> *const c_void>(load_with: F) -> Facade {
        gl::load_with(load_with);

        let mut viewport_info = [0; 4];

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::GetIntegerv(gl::VIEWPORT, viewport_info.as_mut_ptr());
            gl::Enable(gl::FRAMEBUFFER_SRGB);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LEQUAL);
            gl::DepthRange(0.0, 1.0);
        }

        Facade {
            dpi: 72,
            id_map: HashMap::default(),
            font_id_map: HashMap::default(),
            
            color_passthrough: ColorVertexProgram::new(),
            char_vertex: CharVertexProgram::new(),

            sampler: GLSampler::new(),

            viewport_size: (viewport_info[2], viewport_info[3]),
            viewport_size_changed: false
        }
    }

    pub fn resize(&mut self, x: u32, y: u32) {
        self.viewport_size = (x as GLint, y as GLint);
        self.viewport_size_changed = true;

        unsafe{ gl::Viewport(0, 0, self.viewport_size.0, self.viewport_size.1) };
    }

    pub fn surface(&mut self) -> GLSurface {
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::ClearDepth(0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        GLSurface {
            facade: self,
            depth_offset: 0.0
        }
    }
}

pub struct GLSurface<'a> {
    facade: &'a mut Facade,
    depth_offset: f64
}

impl<'a> Surface for GLSurface<'a> {
    fn draw<S: Shadable>(&mut self, drawable: &Widget<S>) {
        use std::collections::hash_map::Entry;

        let buffers = &drawable.buffer_data;
        // Whether or not to re-upload any data to the GPU buffers
        let update_buffers: bool;

        {
            let id_map_entry_mut: &mut IDMapEntry;
            match self.facade.id_map.entry(buffers.id) {
                Entry::Occupied(mut entry) => {
                    update_buffers = !(drawable.num_updates == entry.get().num_updates);
                    entry.get_mut().num_updates = drawable.num_updates;
                    id_map_entry_mut = entry.into_mut();
                }
                Entry::Vacant(entry)   => {
                    update_buffers = true;
                    id_map_entry_mut = entry.insert(
                        IDMapEntry {
                            num_updates: drawable.num_updates,
                            render_data_vec: Vec::new(),

                            mask_verts: Vec::new(),
                            mask_indices: Vec::new(),
                            mask_offset: 0,
                            mask_base_vertex: 0,
                            depth_offset: 0.0
                        }
                    );
                }
            }
            
            if update_buffers || self.facade.viewport_size_changed {
                // Annoyingly, we can't borrow these variables directly inside of the closure because
                // that throws an error. Binding them through these works. Probably a bug, and should be reported to
                // the rust compiler.
                let font_id_map = &mut self.facade.font_id_map;
                let dpi = self.facade.dpi;
                let viewport_size = self.facade.viewport_size;

                buffers.verts.with(|mut vert_modder|
                buffers.vert_indices.with(|mut index_modder|
                buffers.chars.with(|mut char_modder| {
                    vert_modder.buffer_vec().clear();
                    index_modder.buffer_vec().clear();
                    char_modder.buffer_vec().clear();
                    id_map_entry_mut.render_data_vec.clear();
                    id_map_entry_mut.mask_verts.clear();
                    id_map_entry_mut.mask_indices.clear();

                    let mut index_offset = 0;
                    let mut index_bias = 0;
                    let mut max_depth = -32767;

                    {
                        let mut sdc = ShaderDataCollector {
                            matrix: One::one(),
                            pts_rat_scale: Vector2::new(
                                2.0 * dpi as f32 / (viewport_size.0 as f32 * 72.0),
                                2.0 * dpi as f32 / (viewport_size.1 as f32 * 72.0)
                            ),

                            vert_vec: vert_modder.buffer_vec(),
                            index_vec: index_modder.buffer_vec(),
                            char_vec: char_modder.buffer_vec(),
                            index_offset: &mut index_offset,
                            index_bias: &mut index_bias,

                            mask_verts: &mut id_map_entry_mut.mask_verts,
                            mask_indices: &mut id_map_entry_mut.mask_indices,

                            render_data_vec: &mut id_map_entry_mut.render_data_vec,

                            font_id_map:font_id_map,

                            // We use -32767 instead of -32768 (i16's actual minimum value) because -32767 is
                            // the same distance from zero as i16's max value, 32767. This makes the math easier.
                            depth: -32767,
                            max_depth: &mut max_depth,

                            dpi: dpi,
                            viewport_size: viewport_size
                        };
                        drawable.shader_data(sdc.take());
                        sdc.push_to_render_data_vec();
                    }

                    id_map_entry_mut.depth_offset = (max_depth as i32 + 32767) as f64 / 65535.0;

                    id_map_entry_mut.mask_offset = index_modder.buffer_vec().len() as GLint;
                    id_map_entry_mut.mask_base_vertex = vert_modder.buffer_vec().len() as GLint;
                    vert_modder.buffer_vec().extend_from_slice(&id_map_entry_mut.mask_verts);
                    index_modder.buffer_vec().extend_from_slice(&id_map_entry_mut.mask_indices);
                })));
            }
        }

        // Unfortunately, we can't just re-use the mutable reference to the id_map_entry, as we also need
        // to borrow the struct owning the entry as immutable. This workaround has a slight runtime cost,
        // so it's in the program's best interest to have this hack removed.
        let id_map_entry = self.facade.id_map.get(&buffers.id).unwrap();

        if 0 < id_map_entry.mask_indices.len() {unsafe {
            gl::DepthFunc(gl::ALWAYS);
            gl::DepthRange(self.depth_offset, 1.0 + self.depth_offset);
            self.facade.color_passthrough.program.with(|_|
                buffers.verts_vao.with(|_| {
                    gl::DrawElementsBaseVertex(
                        gl::TRIANGLES,
                        id_map_entry.mask_indices.len() as GLsizei,
                        gl::UNSIGNED_SHORT,
                        (id_map_entry.mask_offset * mem::size_of::<u16>() as GLint) as *const _,
                        id_map_entry.mask_base_vertex
                    );
                }));
            gl::DepthFunc(gl::LEQUAL);
        }} else {unsafe{
            gl::DepthRange(0.0, 1.0);
        }}

        self.depth_offset += id_map_entry.depth_offset;

        for render_data in &id_map_entry.render_data_vec {unsafe{
            match *render_data {
                RenderData::ColorVerts{offset, count} =>
                    self.facade.color_passthrough.program.with(|_|
                        buffers.verts_vao.with(|_| {
                            gl::DrawElementsBaseVertex(
                                gl::TRIANGLES, 
                                count as GLsizei, 
                                gl::UNSIGNED_SHORT, 
                                (offset * mem::size_of::<u16>()) as *const _,
                                0
                            );
                        })
                    ),
                RenderData::CharVerts{offset, count, base_location, color, reupload_font_image, ref font} =>
                    self.facade.char_vertex.program.with(|_| 
                        buffers.chars_vao.with(|_| {
                            let font_texture = self.facade.font_id_map.get(&font.id())
                                .expect("Dangling Font ID; should never happen");

                            if reupload_font_image {
                                let raw_font = font.raw_font().borrow();
                                let atlas_image = raw_font.atlas_image();

                                font_texture.swap_data(
                                    atlas_image.width,
                                    atlas_image.height,
                                    atlas_image.pixels,
                                    TextureFormat::R8
                                );
                            }
                            self.facade.sampler.with_texture(
                                self.facade.char_vertex.font_image_tex_unit as GLuint,
                                font_texture
                            );

                            gl::Uniform2f(
                                self.facade.char_vertex.base_location_uniform,
                                base_location.x, base_location.y
                            );
                            gl::Uniform2f(
                                self.facade.char_vertex.viewport_size_px_uniform,
                                self.facade.viewport_size.0 as f32, self.facade.viewport_size.1 as f32
                            );
                            gl::Uniform4f(
                                self.facade.char_vertex.color_uniform,
                                color.r as f32 / 255.0, 
                                color.g as f32 / 255.0, 
                                color.b as f32 / 255.0, 
                                color.a as f32 / 255.0
                            );

                            gl::DrawArrays(
                                gl::POINTS,
                                offset as GLint,
                                count as GLsizei
                            );
                        })
                    )
            }
        }}
    }
}

impl<'a> Drop for GLSurface<'a> {
    fn drop(&mut self) {
        self.facade.viewport_size_changed = false;
    }
}

enum RenderData {
    ColorVerts {
        offset: usize,
        count: usize
    },
    CharVerts {
        offset: usize,
        count: usize,
        base_location: Point,
        color: Color,
        reupload_font_image: bool,
        font: Font
    }
}

pub struct ShaderDataCollector<'a> {
    matrix: Matrix3<f32>,
    pts_rat_scale: Vector2<f32>,

    vert_vec: &'a mut Vec<ColorVertGpu>,
    index_vec: &'a mut Vec<u16>,
    char_vec: &'a mut Vec<CharVertDepth>,
    index_offset: &'a mut usize,
    index_bias: &'a mut u16,

    mask_verts: &'a mut Vec<ColorVertGpu>,
    mask_indices: &'a mut Vec<u16>,

    render_data_vec: &'a mut Vec<RenderData>,

    /// A reference to the facade's font_id_map, which this struct's `update_buffers` function adds to
    /// in the event that the desired font is not in the map.
    font_id_map: &'a mut HashMap<u64, GLTexture, HasherType>,

    // Why is depth an i16 and not a u16? Well, OpenGL has a depth range of -1 to 1, instead of 0 to 1.
    // This way -32767 maps nicely to -1.0, instead of us having to subtract stuff from a u16 after
    // converting it to a float.
    depth: i16,
    max_depth: &'a mut i16,

    dpi: u32,
    viewport_size: (GLint, GLint)
}

impl<'a> ShaderDataCollector<'a> {
    fn push_to_render_data_vec(&mut self) {
        if *self.index_offset < self.index_vec.len() {
            self.render_data_vec.push(RenderData::ColorVerts{
                offset: *self.index_offset,
                count: self.index_vec.len() - *self.index_offset as usize
            });

            *self.index_offset = self.index_vec.len();
            *self.index_bias = self.vert_vec.len() as u16;
        }
    }

    pub fn take(&mut self) -> ShaderDataCollector {
        *self.index_bias = self.vert_vec.len() as u16;

        ShaderDataCollector {
            matrix: self.matrix,
            pts_rat_scale: self.pts_rat_scale,

            vert_vec: self.vert_vec,
            index_vec: self.index_vec,
            char_vec: self.char_vec,
            index_offset: self.index_offset,
            index_bias: self.index_bias,

            mask_verts: self.mask_verts,
            mask_indices: self.mask_indices,

            render_data_vec: self.render_data_vec,

            font_id_map: self.font_id_map,

            depth: self.depth,
            max_depth: self.max_depth,

            dpi: self.dpi,
            viewport_size: self.viewport_size
        }
    }

    pub fn push_vert(&mut self, vert: ColorVert) {
        self.vert_vec.push(vert.to_gpu(self.depth, self.pts_rat_scale, self.matrix));
    }

    pub fn verts_extend_from_slice(&mut self, verts: &[ColorVert]) {
        let depth = self.depth;
        let pts_rat_scale = self.pts_rat_scale;
        let matrix = self.matrix;
        self.vert_vec.extend(verts.iter().map(|v| v.to_gpu(depth, pts_rat_scale, matrix)));
    }

    pub fn push_indices(&mut self, indices: [u16; 3]) {
        let index_bias = *self.index_bias;
        self.index_vec.extend(indices.iter().map(|i| *i + index_bias));
    }

    pub fn indices_extend_from_slice(&mut self, indices: &[[u16; 3]]) {
        use std::slice;

        let index_bias = *self.index_bias;

        let collapsed_slice = unsafe{ slice::from_raw_parts(indices.as_ptr() as *const u16, indices.len() * 3) };
        self.index_vec.extend(collapsed_slice.iter().map(|i| *i + index_bias));
    }

    pub fn push_text(&mut self, rect: Rect, color: Color, text: &str, font: &Font, font_size: u32) {
        self.push_to_render_data_vec();

        let mut raw_font = font.raw_font().borrow_mut();
        let font_height_px = raw_font.height(font_size, self.dpi) as f32;
        let font_height_gl = font_height_px / (self.viewport_size.1 as f32 / 2.0);

        let char_offset = self.char_vec.len();

        // Tranform the base location into window-space coordinates
        let base_location_point = rect.upleft().rat + rect.upleft().pts * self.pts_rat_scale;
        let base_location_vec3 = self.matrix * Vector3::new(base_location_point.x, base_location_point.y, 1.0);

        let rect_width_px = 
            (self.dpi as f32 / 72.0) * (rect.upright.pts.x - rect.lowleft.pts.x) +
            self.viewport_size.0 as f32 * self.matrix.x.x * (rect.upright.rat.x - rect.lowleft.rat.x) / 2.0;

        let (word_iter, mut reupload_font_image) = raw_font.word_iter(text, font_size, self.dpi);

        // If the facade doesn't have a texture created for this font, create one. If we do need to create
        // one, then we'll need to reupload the texture so force that to true. The actual texture gets
        // uploaded right before the draw calls happen.
        self.font_id_map.entry(font.id()).or_insert_with(|| {
            reupload_font_image = true;
            GLTexture::empty()
        });

        let mut count = 0;
        let mut line_offset = Point::new(0.0, 0.0);
        for w in word_iter {
            // If the length of the word causes the line length to exceed the length of the text box,
            // wrap the word and move down one line.
            if w.offset().x + w.word_len_px() + line_offset.x > rect_width_px {
                line_offset.x = -w.offset().x;
                line_offset.y -= font_height_px;
            }

            for v_result in w.char_vert_iter() {
                match v_result {
                    Ok(mut v) => {
                        v.offset = v.offset + w.offset() + line_offset;
                        self.char_vec.push(v.with_depth(self.depth));
                        count += 1;
                    }
                    Err(ci) => match ci.character {
                        '\n' => {
                            line_offset.x = -w.offset().x - ci.offset.x;
                            line_offset.y -= font_height_px;
                        }
                        _ => ()
                    }
                }
            }
        }

        self.render_data_vec.push(RenderData::CharVerts {
            offset: char_offset,
            count: count,
            // Because the base location specifies the upper-left coordinate of the font renderer, we need to
            // shift it downwards by the height of the font so that the font appears inside of the text box
            // instead of above it.
            base_location: Point::new(base_location_vec3.x, base_location_vec3.y - font_height_gl),
            color: color,
            reupload_font_image: reupload_font_image,
            font: font.clone()
        });
    }

    pub fn with_rect(&mut self, scale: Rect) {
        // Create the new matrix and new pts_rat_scale
        let (rat_width, rat_height) =
            (
                self.matrix.x.x * scale.width().rat,
                self.matrix.y.y * scale.height().rat
            );

        let complex_center = scale.center();
        let center = 
            complex_center.rat + 
            Point::new(
                complex_center.pts.x * self.pts_rat_scale.x,
                complex_center.pts.y * self.pts_rat_scale.y
            );

        let width = rat_width + scale.width().pts * self.pts_rat_scale.x;
        let height = rat_height + scale.height().pts * self.pts_rat_scale.y;

        let new_matrix = self.matrix * Matrix3::new(
            width/2.0,        0.0, 0.0,
                  0.0, height/2.0, 0.0,
             center.x,   center.y, 1.0
        );

        // If the rat width or height is zero, then the pts_rat_scale will end up being infinity due to a
        // divide by zero error. This fixes that, allowing pts to be used even with rats of zero.
        let pts_rat_scale_x_div = if rat_width == 0.0 {2.0} else {rat_width};
        let pts_rat_scale_y_div = if rat_height == 0.0 {2.0} else {rat_height};

        let pts_rat_scale = Vector2::new(
            (4.0 / pts_rat_scale_x_div) * self.dpi as f32 / (self.viewport_size.0 as f32 * 72.0),
            (4.0 / pts_rat_scale_y_div) * self.dpi as f32 / (self.viewport_size.1 as f32 * 72.0)
        );
        
        self.matrix = new_matrix;
        self.pts_rat_scale = pts_rat_scale;
    }

    pub fn with_matrix(&mut self, matrix: Matrix3<f32>) {
        self.matrix = self.matrix * matrix;
    }

    pub fn with_mask<'b, VI, II>(&mut self, verts: VI, indices: II)
            where VI: IntoIterator<Item = &'b Complex>, 
                  II: IntoIterator<Item = &'b [u16; 3]> {
        use std::cmp;

        self.depth += 1;
        *self.max_depth = cmp::max(*self.max_depth, self.depth);

        let pts_rat_scale = self.pts_rat_scale;
        let matrix = self.matrix;

        let depth = self.depth;
        self.mask_verts.extend(verts.into_iter().map(|v| ColorVertGpu {
            pos: Vector3{
                z: depth as f32 / i16::max_value() as f32,
                ..v.mul_matrix(pts_rat_scale, matrix)
            },
            color: Color::new(0, 0, 0, 0)
        }));

        for ins in indices {
            self.mask_indices.extend(ins.iter().cloned());
        }
    }
}

impl<'a> Drop for ShaderDataCollector<'a> {
    fn drop(&mut self) {
        *self.index_bias = self.vert_vec.len() as u16;
    }
}
