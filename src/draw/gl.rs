use super::{Vertex, Shader, Drawable, Surface};

use std::collections::HashMap;

use gl::types::*;
use gl_raii::{GLVertexBuffer, GLVertex, BufferUsage,
              VertexAttribData, GLSLType, GLPrim,
              GLVertexArray, GLIndexBuffer, GLBuffer,
              GLProgram, GLShader, ShaderType};

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
                glsl_type: GLSLType::Vec4(GLPrim::NByte),
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
    pub fn new() -> Facade {
        use std::fs::File;
        use std::io::Read;

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
}

pub struct GLSurface<'a> {
    facade: &'a mut Facade
}

impl<'a> Surface for GLSurface<'a> {
    fn draw<D: Drawable>(&mut self, drawable: &D) {
        use gl;
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

        let mut draw_len = 0;
        if update_buffers {
            match drawable.shader_data() {
                Shader::Verts {verts, indices} => {
                    buffers.verts.with(|modder| {
                        modder.upload_data(verts);
                    });

                    buffers.vert_indices.with(|modder| {
                        modder.upload_data(indices);
                    });

                    draw_len = indices.len();
                }

                Shader::None => (),
                _ => unimplemented!()
            }
        }

        self.facade.color_passthrough.with(|_| {
            buffers.verts_vao.with(|_| {unsafe{
                gl::DrawElements(gl::TRIANGLES, draw_len as GLsizei, gl::UNSIGNED_SHORT, ptr::null());
            }});
        });
    }
}
