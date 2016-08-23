use super::{Vertex, Shader, Shadable, Drawable, Surface};

use std::collections::HashMap;
use std::os::raw::c_void;

use gl;
use gl::types::*;
use gl_raii::{GLVertexBuffer, GLVertex, BufferUsage,
              VertexAttribData, GLSLType, GLPrim,
              GLVertexArray, GLIndexBuffer, GLBuffer,
              GLProgram, GLShader, ShaderType, BufModder};

static mut ID_COUNTER: u64 = 0;

pub struct BufferData {
    id: u64,
    verts: GLVertexBuffer<Vertex>,
    vert_indices: GLIndexBuffer,
    verts_vao: GLVertexArray<Vertex>
}

impl BufferData {
    pub fn new() -> BufferData {
        let id = unsafe{ ID_COUNTER };
        unsafe{ ID_COUNTER += 1 };

        let verts = GLVertexBuffer::new(0, BufferUsage::Static);
        let vert_indices = GLIndexBuffer::new(0, BufferUsage::Static);
        let verts_vao = GLVertexArray::new(&verts, Some(&vert_indices));
        BufferData {
            id: id,
            verts: verts,
            vert_indices: vert_indices,
            verts_vao: verts_vao
        }
    }
}

unsafe impl GLVertex for Vertex {
    unsafe fn vertex_attrib_data() -> &'static [VertexAttribData] {
        const VAD: &'static [VertexAttribData] = &[
            // Relative point
            VertexAttribData {
                index: 0,
                glsl_type: GLSLType::Vec2(GLPrim::Float),
                offset: 0
            },

            // Absolute point
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

pub struct Facade {
    id_update_map: HashMap<u64, u64>,
    color_passthrough: GLProgram
}

impl Facade {
    pub fn new<F: Fn(&str) -> *const c_void>(load_with: F) -> Facade {
        use std::fs::File;
        use std::io::Read;

        gl::load_with(load_with);

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let mut colored_vertex_string = String::new();
        File::open("./src/shaders/colored_vertex.vert").unwrap().read_to_string(&mut colored_vertex_string).unwrap();
        let colored_vertex_vert = GLShader::new(ShaderType::Vertex, &colored_vertex_string).unwrap();

        let mut color_passthrough_string = String::new();
        File::open("./src/shaders/color_passthrough.frag").unwrap().read_to_string(&mut color_passthrough_string).unwrap();
        let color_passthrough_frag = GLShader::new(ShaderType::Fragment, &color_passthrough_string).unwrap();

        let color_passthrough = GLProgram::new(&colored_vertex_vert, &color_passthrough_frag).unwrap();

        Facade {
            id_update_map: HashMap::with_capacity(32),
            color_passthrough: color_passthrough
        }
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
        use std::ptr;

        use std::collections::hash_map::Entry;

        let buffers = drawable.buffer_data();
        // Whether or not to re-upload any data to the GPU buffers
        let update_buffers: bool;

        match self.facade.id_update_map.entry(buffers.id) {
            Entry::Occupied(mut entry) => {
                update_buffers = !drawable.num_updates() == *entry.get();
                entry.insert(drawable.num_updates());
            }
            Entry::Vacant(entry)   => {
                update_buffers = true;
                entry.insert(drawable.num_updates());
            }
        }

        let draw_len = drawable.shader_data().count();
        if update_buffers {
            buffers.verts.with(|vert_modder|
                buffers.vert_indices.with(|index_modder| {
                    let mut bud = BufferUpdateData {
                        vert_offset: 0,
                        vert_modder: vert_modder,
                        index_offset: 0,
                        offsetted_indices: Vec::new(),
                        index_modder: index_modder
                    };

                    bud.update_buffers(drawable);
                })
            );
        }

        self.facade.color_passthrough.with(|_| {
            buffers.verts_vao.with(|_| {unsafe{
                gl::DrawElements(gl::TRIANGLES, draw_len as GLsizei, gl::UNSIGNED_SHORT, ptr::null());
            }});
        });
    }
}

struct BufferUpdateData<'a> {
    vert_offset: usize,
    vert_modder: BufModder<'a, Vertex, GLVertexBuffer<Vertex>>,
    index_offset: usize,
    offsetted_indices: Vec<u16>,
    index_modder: BufModder<'a, u16, GLIndexBuffer>
}

impl<'a> BufferUpdateData<'a> {
    fn update_buffers<S: Shadable>(&mut self, shadable: &S) {
        match shadable.shader_data() {
            Shader::Verts {verts, indices} => {
                self.vert_modder.sub_data(self.vert_offset, verts);

                if self.index_offset > 0 {
                    // Clear the vector without de-allocating memory
                    unsafe{ self.offsetted_indices.set_len(0) };

                    // Because every vertex is being stored in one vertex buffer, we need to offset the indices so that
                    // they all get drawn properly
                    let vert_offset = self.vert_offset as u16;
                    self.offsetted_indices.extend(indices.iter().map(|i| *i + vert_offset));

                    self.index_modder.sub_data(self.index_offset, &self.offsetted_indices);
                } else {
                    self.index_modder.sub_data(self.index_offset, indices);
                }

                self.vert_offset += verts.len();
                self.index_offset += indices.len();
            }

            Shader::Composite{foreground, fill, backdrop, ..} => {
                // We order the vertices so that OpenGL draws them back-to-front
                self.update_buffers(&backdrop);
                self.update_buffers(&fill);
                self.update_buffers(&foreground);
            }

            Shader::None => ()
        }
    }
}
