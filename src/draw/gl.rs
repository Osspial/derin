use super::{Vertex, Shader, Drawable, Surface};

use std::collections::HashMap;

use gl::types::*;
use gl_raii::{GLVertexBuffer, GLVertex, BufferUsage,
              VertexAttribData, GLSLType, GLPrim,
              GLVertexArray, GLIndexBuffer, GLBuffer};

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
    id_update_map: HashMap<u64, u64>
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

        buffers.verts_vao.with(|_| {unsafe{
            gl::DrawElements(gl::TRIANGLES, draw_len as GLsizei, gl::UNSIGNED_SHORT, ptr::null());
        }});
    }
}
