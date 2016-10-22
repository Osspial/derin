use draw::{Complex, Color, ColorVert};
use draw::font::{CharVert};

use cgmath::{Matrix3, Vector2, Vector3};

use gl;
use gl::types::*;
use gl_raii::*;

pub struct BufferData {
    pub verts: GLVertexBuffer<ColorVertGpu>,
    pub vert_indices: GLIndexBuffer,
    pub verts_vao: GLVertexArray<ColorVertGpu>,
    pub chars: GLVertexBuffer<CharVertDepth>,
    pub chars_vao: GLVertexArray<CharVertDepth>
}

impl Default for  BufferData {
    fn default() -> BufferData {
        let verts = GLVertexBuffer::new(0, BufferUsage::Static);
        let vert_indices = GLIndexBuffer::new(0, BufferUsage::Static);
        let verts_vao = GLVertexArray::new(&verts, Some(&vert_indices));

        let chars = GLVertexBuffer::new(0, BufferUsage::Dynamic);
        let chars_vao = GLVertexArray::new(&chars, None);

        BufferData {
            verts: verts,
            vert_indices: vert_indices,
            verts_vao: verts_vao,
            chars: chars,
            chars_vao: chars_vao
        }
    }
}

impl Complex {
    pub fn mul_matrix(self, pts_rat_scale: Vector2<f32>, matrix: Matrix3<f32>) -> Vector3<f32> {
        let pure_rat = Vector2::new(self.rat.x, self.rat.y) +
                       Vector2::new(pts_rat_scale.x * self.pts.x, pts_rat_scale.y * self.pts.y);

        matrix * pure_rat.extend(1.0)
    }
}

/// The GPU representation of a `ColorVert`. The reason this exists is that it has the matrix
/// already performed, allowing us to batch the draw calls of multiple objects.
#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub struct ColorVertGpu {
    pub pos: Vector3<f32>,
    pub color: Color
}

impl ColorVert {
    pub fn to_gpu(self, depth: i16, pts_rat_scale: Vector2<f32>, matrix: Matrix3<f32>) -> ColorVertGpu {
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
pub struct CharVertDepth {
    pub char_vert: CharVert,
    pub depth: f32
}

impl CharVert {
    pub fn with_depth(self, depth: i16) -> CharVertDepth {
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

pub struct ColorVertexProgram {
    pub program: GLProgram
}

impl ColorVertexProgram {
    pub fn new() -> ColorVertexProgram {
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

impl Default for ColorVertexProgram {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CharVertexProgram {
    pub program: GLProgram,
    pub base_location_uniform: GLint,
    pub viewport_size_px_uniform: GLint,
    pub color_uniform: GLint,

    // font_image_uniform: GLint,
    pub font_image_tex_unit: GLint
}

impl CharVertexProgram {
    pub fn new() -> CharVertexProgram {
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

impl Default for CharVertexProgram {
    fn default() -> Self {
        Self::new()
    }
}
