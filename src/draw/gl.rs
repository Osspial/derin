use super::{Point, ColorVert, Shader, Shadable, Drawable, Surface};
use super::font::{Font, CharVert};

use std::collections::HashMap;
use std::os::raw::c_void;

use gl;
use gl::types::*;
use gl_raii::*;

use cgmath::{Matrix3, Vector2, Vector3};
use cgmath::prelude::*;

static mut ID_COUNTER: u64 = 0;

pub fn get_unique_id() -> u64 {
    let id = unsafe{ ID_COUNTER };
    unsafe{ ID_COUNTER += 1 };
    id
}

pub struct BufferData {
    id: u64,
    verts: GLVertexBuffer<ColorVert>,
    vert_indices: GLIndexBuffer,
    verts_vao: GLVertexArray<ColorVert>,
    chars: GLVertexBuffer<CharVert>,
    chars_vao: GLVertexArray<CharVert>
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

unsafe impl GLVertex for ColorVert {
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
            }
        ];

        VAD
    }
}

unsafe impl GLVertex for CharVert {
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
            // font_image_uniform: font_image_uniform,
            font_image_tex_unit: font_image_tex_unit
        }
    }
}

struct IDMapEntry {
    num_updates: u64,
    base_vertex_vec: Vec<BaseVertexData>,
    char_vertex_vec: Vec<CharVertexData>
}

pub struct Facade {
    pub dpi: u32,
    id_map: HashMap<u64, IDMapEntry>,
    font_id_map: HashMap<u64, GLTexture>,

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
        }

        Facade {
            dpi: 72,
            id_map: HashMap::with_capacity(32),
            font_id_map: HashMap::with_capacity(8),
            
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
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        GLSurface {
            facade: self
        }
    }
}

pub struct GLSurface<'a> {
    facade: &'a mut Facade
}

impl<'a> Surface for GLSurface<'a> {
    fn draw<D: Drawable>(&mut self, drawable: &D) {
        use std::collections::hash_map::Entry;

        let buffers = drawable.buffer_data();
        // Whether or not to re-upload any data to the GPU buffers
        let update_buffers: bool;

        {
            let id_map_entry_mut: &mut IDMapEntry;
            match self.facade.id_map.entry(buffers.id) {
                Entry::Occupied(mut entry) => {
                    update_buffers = !drawable.num_updates() == entry.get().num_updates;
                    entry.get_mut().num_updates = drawable.num_updates();
                    id_map_entry_mut = entry.into_mut();
                }
                Entry::Vacant(entry)   => {
                    update_buffers = true;
                    id_map_entry_mut = entry.insert(
                        IDMapEntry {
                            num_updates: drawable.num_updates(),
                            base_vertex_vec: Vec::new(),
                            char_vertex_vec: Vec::new()
                        }
                    );
                }
            }
            
            let dpi = self.facade.dpi;
            let viewport_size = self.facade.viewport_size;

            if update_buffers || self.facade.viewport_size_changed {
                // Annoyingly, we can't borrow `self.facade.font_id_map` directly inside of the closure because
                // that throws an error. Doing it through this binding works. Probably a bug, and should be reported to
                // the rust compiler.
                let font_id_map = &mut self.facade.font_id_map;

                buffers.verts.with(|mut vert_modder|
                buffers.vert_indices.with(|mut index_modder|
                buffers.chars.with(|mut char_modder| {
                    vert_modder.buffer_vec().clear();
                    index_modder.buffer_vec().clear();
                    char_modder.buffer_vec().clear();

                    let mut bud = BufferUpdateData {
                        vert_offset: 0,
                        vert_vec: vert_modder.buffer_vec(),

                        index_offset: 0,
                        offsetted_indices: Vec::new(),
                        index_vec: index_modder.buffer_vec(),

                        char_offset: 0,
                        char_vec: char_modder.buffer_vec(),

                        base_vertex_vec: vec![Default::default(); 1],
                        char_vertex_vec: Vec::new(),
                        matrix_stack: vec![One::one(); 1],

                        font_id_map: font_id_map,

                        dpi: dpi,
                        viewport_size: viewport_size
                    };

                    bud.update_buffers(drawable);

                    // If `drawable` is a composite, then there will be one extra BaseVertexData pushed to the vector
                    // that we need to get rid of. This does that.
                    if let Shader::Composite{..} = drawable.shader_data() {
                        bud.base_vertex_vec.pop();
                    }
                    id_map_entry_mut.base_vertex_vec = bud.base_vertex_vec;
                    id_map_entry_mut.char_vertex_vec = bud.char_vertex_vec;
                })));
            }
        }

        // Unfortunately, we can't just re-use the mutable reference to the id_map_entry, as we also need
        // to borrow the struct owning the entry as immutable. This workaround has a slight runtime cost,
        // so it's in the program's best interest to have this hack removed.
        let id_map_entry = self.facade.id_map.get(&buffers.id).unwrap();

        let transform_matrix_uniform = self.facade.color_passthrough.transform_matrix_uniform;
        let pts_rat_scale_uniform = self.facade.color_passthrough.pts_rat_scale_uniform;

        self.facade.color_passthrough.program.with(|_| {
            buffers.verts_vao.with(|_| {
                for bvd in id_map_entry.base_vertex_vec.iter() {unsafe{
                    gl::UniformMatrix3fv(
                        transform_matrix_uniform,
                        1,
                        gl::FALSE,
                        bvd.matrix.as_ptr()
                    );

                    gl::Uniform2f(
                        pts_rat_scale_uniform,
                        bvd.pts_rat_scale.x, bvd.pts_rat_scale.y
                    );


                    gl::DrawElementsBaseVertex(
                        gl::TRIANGLES, 
                        bvd.count as GLsizei, 
                        gl::UNSIGNED_SHORT, 
                        bvd.offset as *const _,
                        bvd.base_vertex as GLint
                    );
                }}
            });
        });

        let base_location_uniform = self.facade.char_vertex.base_location_uniform;
        let viewport_size_px_uniform = self.facade.char_vertex.viewport_size_px_uniform;
        self.facade.char_vertex.program.with(|_| 
            buffers.chars_vao.with(|_| 
                for cvd in id_map_entry.char_vertex_vec.iter() {unsafe{
                    let font_texture = self.facade.font_id_map.get(&cvd.font.id())
                        .expect("Dangling Font ID; should never happen");

                    if cvd.reupload_font_image {
                        let raw_font = cvd.font.raw_font().borrow();
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
                        base_location_uniform,
                        cvd.base_location.x, cvd.base_location.y
                    );
                    gl::Uniform2f(
                        viewport_size_px_uniform,
                        self.facade.viewport_size.0 as f32, self.facade.viewport_size.1 as f32
                    );

                    gl::DrawArrays(
                        gl::POINTS,
                        cvd.offset as GLint,
                        cvd.count as GLsizei
                    );
                }}
            )
        );
    }
}

impl<'a> Drop for GLSurface<'a> {
    fn drop(&mut self) {
        self.facade.viewport_size_changed = false;
    }
}

#[derive(Debug, Clone, Copy)]
struct BaseVertexData {
    offset: usize,
    count: usize,
    base_vertex: usize,
    matrix: Matrix3<f32>,
    pts_rat_scale: Vector2<f32>
}

impl Default for BaseVertexData {
    fn default() -> BaseVertexData {
        BaseVertexData {
            offset: 0,
            count: 0,
            base_vertex: 0,
            matrix: One::one(),
            pts_rat_scale: Zero::zero()
        }
    }
}

struct CharVertexData {
    offset: usize,
    count: usize,
    base_location: Point,
    reupload_font_image: bool,
    font: Font
}

struct BufferUpdateData<'a> {
    vert_offset: usize,
    vert_vec: &'a mut Vec<ColorVert>,

    index_offset: usize,
    offsetted_indices: Vec<u16>,
    index_vec: &'a mut Vec<u16>,

    char_offset: usize,
    char_vec: &'a mut Vec<CharVert>,

    base_vertex_vec: Vec<BaseVertexData>,
    char_vertex_vec: Vec<CharVertexData>,
    matrix_stack: Vec<Matrix3<f32>>,

    /// A reference to the facade's font_id_map, which this struct's `update_buffers` function adds to
    /// in the event that the desired font is not in the map.
    font_id_map: &'a mut HashMap<u64, GLTexture>,

    dpi: u32,
    viewport_size: (GLint, GLint)
}

impl<'a> BufferUpdateData<'a> {
    fn update_buffers<S: Shadable>(&mut self, shadable: &S) {
        match shadable.shader_data() {
            Shader::Verts {verts, indices} => {
                self.vert_vec.extend_from_slice(verts);

                let bvd = self.base_vertex_vec.last_mut().unwrap();
                bvd.count += indices.len();

                if self.index_offset > 0 {
                    self.offsetted_indices.clear();

                    // Because every vertex is being stored in one vertex buffer, we need to offset the indices so that
                    // they all get drawn properly
                    let vert_offset = self.vert_offset as u16;
                    self.offsetted_indices.extend(indices.iter().map(|i| *i + vert_offset));

                    self.index_vec.extend_from_slice(&self.offsetted_indices);
                } else {
                    self.index_vec.extend_from_slice(indices);
                }

                self.vert_offset += verts.len();
                self.index_offset += indices.len();
            }

            Shader::Text{rect, text, font, font_size} => {
                let mut raw_font = font.raw_font().borrow_mut();
                let font_height = raw_font.height(font_size, self.dpi) as f32 / (self.viewport_size.1 as f32 / 2.0);

                let (char_vert_iter, mut reupload_font_image) = raw_font.char_vert_iter(text, font_size, self.dpi);

                // If the facade doesn't have a texture created for this font, create one. If we do need to create
                // one, then we'll need to reupload the texture so force that to true. The actual texture gets
                // filled in right before the draw calls happen.
                self.font_id_map.entry(font.id()).or_insert_with(|| {
                    reupload_font_image = true;
                    GLTexture::empty()
                });

                let mut count = 0;
                for v in char_vert_iter {
                    self.char_vec.push(v);
                    count += 1;
                }

                // Tranform the base location into window-space coordinates
                let pts_rat_scale = self.base_vertex_vec.last().unwrap().pts_rat_scale;
                let matrix = self.base_vertex_vec.last().unwrap().matrix;
                let base_location_point = rect.upleft.rat + rect.upleft.pts * pts_rat_scale;
                let base_location_vec3 = matrix * Vector3::new(base_location_point.x, base_location_point.y, 1.0);

                self.char_vertex_vec.push(
                    CharVertexData {
                        offset: self.char_offset,
                        count: count,
                        // Because the base location specifies the upper-left coordinate of the font renderer, we need to
                        // shift it downwards by the height of the font so that the font appears inside of the text box
                        // instead of above it.
                        base_location: Point::new(base_location_vec3.x, base_location_vec3.y - font_height),
                        reupload_font_image: reupload_font_image,
                        font: font.clone()
                    });
            }

            Shader::Composite{foreground, fill, backdrop, rect, ..} => {
                let last_matrix = self.matrix_stack.last().unwrap().clone();

                let (rat_width, rat_height) =
                    (
                        last_matrix.x.x * (rect.lowright.rat.x - rect.upleft.rat.x),
                        last_matrix.y.y * (rect.upleft.rat.y - rect.lowright.rat.y)
                    );

                let pts_rat_scale = Vector2::new(
                    (4.0 / rat_width) * self.dpi as f32 / (self.viewport_size.0 as f32 * 72.0),
                    (4.0 / rat_height) * self.dpi as f32 / (self.viewport_size.1 as f32 * 72.0)
                );

                let complex_center = rect.center();
                let center = 
                    complex_center.rat + 
                    Point::new(
                        complex_center.pts.x * pts_rat_scale.x,
                        complex_center.pts.y * pts_rat_scale.y
                    );

                let width = rat_width + (rect.lowright.pts.x - rect.upleft.rat.y) * pts_rat_scale.x;
                let height = rat_height + (rect.upleft.pts.y - rect.lowright.pts.y) * pts_rat_scale.y;

                let new_matrix = last_matrix * Matrix3::new(
                    width/2.0,        0.0, 0.0,
                          0.0, height/2.0, 0.0,
                     center.x,   center.y, 1.0
                );

                self.base_vertex_vec.last_mut().unwrap().matrix = new_matrix;
                self.base_vertex_vec.last_mut().unwrap().pts_rat_scale = pts_rat_scale;
                self.matrix_stack.push(new_matrix);

                // We order the vertices so that OpenGL draws them back-to-front
                self.update_buffers(&backdrop);
                self.update_buffers(&fill);
                self.update_buffers(&foreground);

                self.base_vertex_vec.push(Default::default());
            }

            Shader::None => ()
        }
    }
}
