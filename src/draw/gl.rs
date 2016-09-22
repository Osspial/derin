use super::{Point, Color, ColorVert, Shadable, Surface, Rect, Widget};
use super::font::{Font, CharVert};

use std::collections::HashMap;
use std::os::raw::c_void;
use std::hash::BuildHasherDefault;

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
    verts: GLVertexBuffer<ColorVertDepth>,
    vert_indices: GLIndexBuffer,
    verts_vao: GLVertexArray<ColorVertDepth>,
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

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
struct ColorVertDepth {
    color_vert: ColorVert,
    depth: f32
}

impl ColorVert {
    fn with_depth(self, depth: u16) -> ColorVertDepth {
        ColorVertDepth {
            color_vert: self,
            depth: depth as f32 / 65536.0
        }
    }
}

unsafe impl GLVertex for ColorVertDepth {
    unsafe fn vertex_attrib_data() -> &'static [VertexAttribData] {
        const VAD: &'static [VertexAttribData] = &[
            // Ratio Vec2
            VertexAttribData {
                index: 0,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 0
            },

            // Points Vec2
            VertexAttribData {
                index: 1,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 8
            },

            // Normal
            VertexAttribData {
                index: 2,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 16
            },

            // Color
            VertexAttribData {
                index: 3,
                glsl_type: GLSLType::Vec4(GLPrim::NUByte),
                offset: 24
            },

            // Depth
            VertexAttribData {
                index: 4,
                glsl_type: GLSLType::Single(GLPrim::Float),
                offset: 28
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
    fn with_depth(self, depth: u16) -> CharVertDepth {
        CharVertDepth {
            char_vert: self,
            depth: depth as f32 / 65536.0
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
    program: GLProgram,
    transform_matrix_uniform: GLint,
    pts_rat_scale_uniform: GLint
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

        let transform_matrix_uniform = unsafe{ gl::GetUniformLocation(program.handle, "transform_matrix\0".as_ptr() as *const GLchar) };
        let pts_rat_scale_uniform = unsafe{ gl::GetUniformLocation(program.handle, "pts_rat_scale\0".as_ptr() as *const GLchar) };

        ColorVertexProgram {
            program: program,
            transform_matrix_uniform: transform_matrix_uniform,
            pts_rat_scale_uniform: pts_rat_scale_uniform
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
    render_data_vec: Vec<RenderData>
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
            gl::DepthFunc(gl::GEQUAL);
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

    pub fn surface<'a>(&'a mut self) -> GLSurface {
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::ClearDepth(0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        GLSurface {
            facade: self,
            depth_accumulator: 0
        }
    }
}

pub struct GLSurface<'a> {
    facade: &'a mut Facade,
    /// An accumulator that increments every time a shader is drawn. Used to prevent shaders that are executed
    /// after other shaders from drawing above a shader when the programmer told the first shader to draw before
    /// the second. As an example, text is drawn after vertices, so if the programmer instructed the program to
    /// draw the vertices after the text, the depth test is used to make it appear as though that is what's happening.
    depth_accumulator: u16
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
                            render_data_vec: Vec::new()
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
                let mut depth_accumulator = &mut self.depth_accumulator;

                buffers.verts.with(|mut vert_modder|
                buffers.vert_indices.with(|mut index_modder|
                buffers.chars.with(|mut char_modder| {
                    vert_modder.buffer_vec().clear();
                    index_modder.buffer_vec().clear();
                    char_modder.buffer_vec().clear();
                    id_map_entry_mut.render_data_vec.clear();

                    let mut vert_offset = 0;
                    let mut index_offset = 0;

                    drawable.shader_data(&mut ShaderDataCollector {
                        matrix: One::one(),
                        pts_rat_scale: Vector2::new(
                            2.0 * dpi as f32 / (viewport_size.0 as f32 * 72.0),
                            2.0 * dpi as f32 / (viewport_size.1 as f32 * 72.0)
                        ),

                        vert_vec: vert_modder.buffer_vec(),
                        index_vec: index_modder.buffer_vec(),
                        char_vec: char_modder.buffer_vec(),
                        vert_offset: &mut vert_offset,
                        index_offset: &mut index_offset,

                        render_data_vec: &mut id_map_entry_mut.render_data_vec,

                        font_id_map:font_id_map,

                        depth: &mut depth_accumulator,

                        dpi: dpi,
                        viewport_size: viewport_size
                    });
                })));
            }
        }

        // Unfortunately, we can't just re-use the mutable reference to the id_map_entry, as we also need
        // to borrow the struct owning the entry as immutable. This workaround has a slight runtime cost,
        // so it's in the program's best interest to have this hack removed.
        let id_map_entry = self.facade.id_map.get(&buffers.id).unwrap();

        for render_data in id_map_entry.render_data_vec.iter() {unsafe{
            match *render_data {
                RenderData::ColorVerts{offset, count, matrix, pts_rat_scale} =>
                    self.facade.color_passthrough.program.with(|_|
                        buffers.verts_vao.with(|_| {
                            gl::UniformMatrix3fv(
                                self.facade.color_passthrough.transform_matrix_uniform,
                                1,
                                gl::FALSE,
                                matrix.as_ptr()
                            );
                            gl::Uniform2f(
                                self.facade.color_passthrough.pts_rat_scale_uniform,
                                pts_rat_scale.x, pts_rat_scale.y
                            );

                            gl::DrawElementsBaseVertex(
                                gl::TRIANGLES, 
                                count as GLsizei, 
                                gl::UNSIGNED_SHORT, 
                                offset as *const _,
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
        count: usize,
        matrix: Matrix3<f32>,
        pts_rat_scale: Vector2<f32>
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

    vert_vec: &'a mut Vec<ColorVertDepth>,
    index_vec: &'a mut Vec<u16>,
    char_vec: &'a mut Vec<CharVertDepth>,
    vert_offset: &'a mut usize,
    index_offset: &'a mut usize,

    render_data_vec: &'a mut Vec<RenderData>,

    /// A reference to the facade's font_id_map, which this struct's `update_buffers` function adds to
    /// in the event that the desired font is not in the map.
    font_id_map: &'a mut HashMap<u64, GLTexture, HasherType>,

    depth: &'a mut u16,

    dpi: u32,
    viewport_size: (GLint, GLint)
}

impl<'a> ShaderDataCollector<'a> {
    fn push_to_render_data_vec(&mut self) {
        if *self.vert_offset < self.vert_vec.len() {
            self.render_data_vec.push(RenderData::ColorVerts{
                offset: *self.index_offset,
                count: self.index_vec.len() - *self.index_offset,
                matrix: self.matrix,
                pts_rat_scale: self.pts_rat_scale
            });

            *self.depth += 1;
            *self.vert_offset = self.vert_vec.len();
            *self.index_offset = self.index_vec.len();
        }
    }

    pub fn push_vert(&mut self, vert: ColorVert) {
        self.vert_vec.push(vert.with_depth(*self.depth));
    }

    pub fn verts_extend_from_slice(&mut self, verts: &[ColorVert]) {
        let depth = *self.depth;
        self.vert_vec.extend(verts.iter().map(|v| v.with_depth(depth)));
    }

    pub fn push_indices(&mut self, indices: [u16; 3]) {
        self.index_vec.extend_from_slice(&indices);
    }

    pub fn indices_extend_from_slice(&mut self, indices: &[[u16; 3]]) {
        use std::slice;

        let collapsed_slice = unsafe{ slice::from_raw_parts(indices.as_ptr() as *const u16, indices.len() * 3) };
        self.index_vec.extend_from_slice(collapsed_slice);
    }

    pub fn push_text(&mut self, rect: Rect, text: &str, color: Color, font: &Font, font_size: u32) {
        self.push_to_render_data_vec();

        let mut raw_font = font.raw_font().borrow_mut();
        let font_height_px = raw_font.height(font_size, self.dpi) as f32;
        let font_height_gl = font_height_px / (self.viewport_size.1 as f32 / 2.0);

        let char_offset = self.char_vec.len();

        // Tranform the base location into window-space coordinates
        let base_location_point = rect.upleft.rat + rect.upleft.pts * self.pts_rat_scale;
        let base_location_vec3 = self.matrix * Vector3::new(base_location_point.x, base_location_point.y, 1.0);

        let rect_width_px =
            (rect.lowright.pts.x + rect.lowright.rat.x / self.pts_rat_scale.x) -
            (rect.upleft.pts.x + rect.upleft.rat.x / self.pts_rat_scale.x);

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
                        self.char_vec.push(v.with_depth(*self.depth));
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
        *self.depth += 1;
    }

    pub fn with_transform<'b>(&'b mut self, scale: Rect) -> ShaderDataCollector<'b> {
        self.push_to_render_data_vec();

        // Create the new matrix and new pts_rat_scale
        let (rat_width, rat_height) =
            (
                self.matrix.x.x * (scale.lowright.rat.x - scale.upleft.rat.x),
                self.matrix.y.y * (scale.upleft.rat.y - scale.lowright.rat.y)
            );

        let pts_rat_scale = Vector2::new(
            (4.0 / rat_width) * self.dpi as f32 / (self.viewport_size.0 as f32 * 72.0),
            (4.0 / rat_height) * self.dpi as f32 / (self.viewport_size.1 as f32 * 72.0)
        );

        let complex_center = scale.center();
        let center = 
            complex_center.rat + 
            Point::new(
                complex_center.pts.x * pts_rat_scale.x,
                complex_center.pts.y * pts_rat_scale.y
            );

        let width = rat_width + (scale.lowright.pts.x - scale.upleft.rat.y) * pts_rat_scale.x;
        let height = rat_height + (scale.upleft.pts.y - scale.lowright.pts.y) * pts_rat_scale.y;

        let new_matrix = self.matrix * Matrix3::new(
            width/2.0,        0.0, 0.0,
                  0.0, height/2.0, 0.0,
             center.x,   center.y, 1.0
        );

        ShaderDataCollector {
            matrix: new_matrix,
            pts_rat_scale: pts_rat_scale,

            vert_vec: self.vert_vec,
            index_vec: self.index_vec,
            char_vec: self.char_vec,
            vert_offset: self.vert_offset,
            index_offset: self.index_offset,

            render_data_vec: self.render_data_vec,

            font_id_map: self.font_id_map,

            depth: self.depth,
            dpi: self.dpi,
            viewport_size: self.viewport_size
        }
    }

    // pub fn with_clip<'b, VI, II>(&'b mut self, verts: VI, indices: II) -> ShaderDataCollector<'b> 
    //         where VI: IntoIterator<Item = Complex>, 
    //               II: IntoIterator<Item = [u16; 3]> {
    // }
}

impl<'a> Drop for ShaderDataCollector<'a> {
    fn drop(&mut self) {
        self.push_to_render_data_vec();
    }
}
