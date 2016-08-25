extern crate gl;

use gl::types::*;

use std::ptr;
use std::marker::PhantomData;
use std::cell::RefCell;

pub type GLError = String;

fn byte_slice_size<T>(slice: &[T]) -> GLsizeiptr {
    use std::mem::size_of;
    (size_of::<T>() * slice.len()) as GLsizeiptr
}

pub unsafe trait GLVertex: Sized + Copy {
    unsafe fn vertex_attrib_data() -> &'static [VertexAttribData];
}

pub struct VertexAttribData {
    pub index: GLuint,
    pub glsl_type: GLSLType,
    pub offset: GLsizei
}

#[derive(Debug, Clone, Copy)]
pub enum GLSLType {
    Single(GLPrim),
    Vec2(GLPrim),
    Vec3(GLPrim),
    Vec4(GLPrim)
}

impl GLSLType {
    /// Gets the underlying GLSL primitive
    fn prim(self) -> GLPrim {
        use self::GLSLType::*;

        match self {
            Single(p) |
            Vec2(p)   |
            Vec3(p)   |
            Vec4(p)  => p
        }
    }

    /// Gets the number of primitives stored in this type
    fn size(self) -> GLint {
        use self::GLSLType::*;

        match self {
            Single(_) => 1,
            Vec2(_)   => 2,
            Vec3(_)   => 3,
            Vec4(_)   => 4
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GLPrim {
    // Standard integer types
    Byte,
    UByte,
    Short,
    UShort,
    Int,
    UInt,

    // Normalized integer types. Converted to floats.
    NByte,
    NUByte,
    NShort,
    NUShort,
    NInt,
    NUInt,

    // Floating-point types
    Float,
    Double
}

impl GLPrim {
    /// Gets whether or not this value is converted into a GLSL floating point value
    fn is_glsl_float(self) -> bool {
        use self::GLPrim::*;

        match self {
            Byte   |
            UByte  |
            Short  |
            UShort |
            Int    |
            UInt  => false,
            _     => true
        }
    }

    fn is_normalized(self) -> GLboolean {
        use self::GLPrim::*;

        match self {
            NByte   |
            NUByte  |
            NShort  |
            NUShort |
            NInt    |
            NUInt  => gl::TRUE,
            _      => gl::FALSE
        }
    }

    fn to_gl_enum(self) -> GLenum {
        use self::GLPrim::*;

        match self {
            Byte     |
            NByte   => gl::BYTE,
            UByte    |
            NUByte  => gl::UNSIGNED_BYTE,
            Short    |
            NShort  => gl::SHORT,
            UShort   |
            NUShort => gl::UNSIGNED_SHORT,
            Int      |
            NInt    => gl::INT,
            UInt     |
            NUInt   => gl::UNSIGNED_INT,
            Float   => gl::FLOAT,
            Double  => gl::DOUBLE
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BufferUsage {
    Static,
    Dynamic,
}

impl BufferUsage {
    fn to_gl_enum(self) -> GLenum {
        use self::BufferUsage::*;

        match self {
            Static => gl::STATIC_DRAW,
            Dynamic => gl::DYNAMIC_DRAW
        }
    }
}

pub trait GLBuffer<T: Copy> 
        where Self: Sized {
    fn new(capacity: usize, usage: BufferUsage) -> Self;
    fn from_slice(slice: &[T], usage: BufferUsage) -> Self;
    fn with<'a, F: FnOnce(BufModder<'a, T, Self>)>(&'a self, func: F)
            where T: 'a;

    fn buffer_type() -> GLenum;
}

pub struct GLVertexBuffer<V: GLVertex> (Buffer<V, GLVertexBuffer<V>>);

impl<V: GLVertex> GLBuffer<V> for GLVertexBuffer<V> {
    fn new(capacity: usize, usage: BufferUsage) -> GLVertexBuffer<V> {
        GLVertexBuffer( Buffer::new(capacity, usage) )
    }

    fn from_slice(slice: &[V], usage: BufferUsage) -> GLVertexBuffer<V> {
        GLVertexBuffer( Buffer::from_slice(slice, usage) )
    }

    fn with<'a, F: FnOnce(BufModder<'a, V, GLVertexBuffer<V>>)>(&'a self, func: F) {
        self.0.with(func)
    }

    #[inline]
    fn buffer_type() -> GLenum {
        gl::ARRAY_BUFFER
    }
}

pub struct GLIndexBuffer (Buffer<GLushort, GLIndexBuffer>);

impl GLBuffer<GLushort> for GLIndexBuffer {
    fn new(capacity: usize, usage: BufferUsage) -> GLIndexBuffer {
        GLIndexBuffer( Buffer::new(capacity, usage) )
    }

    fn from_slice(slice: &[GLushort], usage: BufferUsage) -> GLIndexBuffer {
        GLIndexBuffer( Buffer::from_slice(slice, usage) )
    }

    fn with<'a, F: FnOnce(BufModder<'a, GLushort, GLIndexBuffer>)>(&'a self, func: F) {
        self.0.with(func)
    }

    #[inline]
    fn buffer_type() -> GLenum {
        gl::ELEMENT_ARRAY_BUFFER
    }
}

/// RAII wrapper around OpenGL buffers
struct Buffer<T: Copy, B: GLBuffer<T>> {
    handle: GLuint,
    data: RefCell<Vec<T>>,
    usage: BufferUsage,
    _marker: PhantomData<(T, B)>
}

impl<T: Copy, B: GLBuffer<T>> Buffer<T, B> {
    fn new(capacity: usize, usage: BufferUsage) -> Buffer<T, B> {
        unsafe {
            use std::mem::size_of;

            let mut handle = 0;
            gl::GenBuffers(1, &mut handle);

            gl::BindBuffer(B::buffer_type(), handle);
            gl::BufferData(B::buffer_type(), (size_of::<T>() * capacity) as GLsizeiptr, ptr::null(), usage.to_gl_enum());
            gl::BindBuffer(B::buffer_type(), 0);

            Buffer {
                handle: handle,
                data: RefCell::new(Vec::with_capacity(capacity)),
                usage: usage,
                _marker: PhantomData
            }
        }
    }

    fn from_slice(slice: &[T], usage: BufferUsage) -> Buffer<T, B> {
        unsafe {
            let mut handle = 0;
            gl::GenBuffers(1, &mut handle);

            gl::BindBuffer(B::buffer_type(), handle);
            gl::BufferData(B::buffer_type(), byte_slice_size(slice), slice.as_ptr() as *const _, usage.to_gl_enum());
            gl::BindBuffer(B::buffer_type(), 0);

            let data = slice.to_vec();

            Buffer {
                handle: handle,
                data: RefCell::new(data),
                usage: usage,
                _marker: PhantomData
            }
        }
    }

    fn with<'a, F: FnOnce(BufModder<'a, T, B>)>(&'a self, func: F) {
        unsafe {
            gl::BindBuffer(B::buffer_type(), self.handle);
            func(BufModder(self));
            gl::BindBuffer(B::buffer_type(), 0);
        }
    }
}

pub struct BufModder<'a, T: 'a + Copy, B: 'a + GLBuffer<T>>(&'a Buffer<T, B>);

impl<'a, T: 'a + Copy, B: 'a + GLBuffer<T>> BufModder<'a, T, B> {
    pub fn sub_data(&self, offset: usize, data: &[T]) 
            where T: std::fmt::Debug {
        use std::mem;
        unsafe {
            let mut cached_data = self.0.data.borrow_mut();

            if cached_data.capacity() < data.len() + offset {
                cached_data.truncate(offset);
                cached_data.extend_from_slice(data);
                
                gl::BufferData(
                    B::buffer_type(),
                    byte_slice_size(&cached_data),
                    cached_data.as_ptr() as *const _,
                    self.0.usage.to_gl_enum()
                );
            } else {
                cached_data[offset..data.len() + offset].copy_from_slice(data); 
                gl::BufferSubData(
                    B::buffer_type(),
                    (mem::size_of::<T>() * offset) as GLintptr,
                    byte_slice_size(data),
                    data.as_ptr() as *const _
                )
            }
        }
    }
}


/// RAII wrapper aound Vertex Arrays
pub struct GLVertexArray<V: GLVertex> {
    pub handle: GLuint,
    _marker: PhantomData<V>
}

impl<V: GLVertex> GLVertexArray<V> {
    pub fn new(vertex_buffer: &GLVertexBuffer<V>, index_buffer: Option<&GLIndexBuffer>) -> GLVertexArray<V> {
        use std::mem::size_of;

        let mut handle = 0;
        
        unsafe{ 
            gl::GenVertexArrays(1, &mut handle);
            gl::BindVertexArray(handle);

            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer.0.handle);

            if let Some(ib) = index_buffer {
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ib.0.handle);
            }

            let stride = size_of::<V>() as GLsizei;
            for attrib in V::vertex_attrib_data() {
                let glsl_type = attrib.glsl_type;
                let prim = attrib.glsl_type.prim();
                let offset = attrib.offset as *const GLvoid;

                gl::EnableVertexAttribArray(attrib.index);

                if prim.is_glsl_float() {
                    if prim == GLPrim::Double {
                        gl::VertexAttribLPointer(
                            attrib.index,
                            glsl_type.size(),
                            gl::DOUBLE,
                            stride,
                            offset
                        );
                    } else {
                        gl::VertexAttribPointer(
                            attrib.index, 
                            glsl_type.size(), 
                            prim.to_gl_enum(), 
                            prim.is_normalized(),
                            stride,
                            offset
                        );
                    }
                } else {
                    gl::VertexAttribIPointer(
                        attrib.index,
                        glsl_type.size(),
                        prim.to_gl_enum(),
                        stride,
                        offset
                    );
                }
            }

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }

        assert!(handle != 0);

        GLVertexArray {
            handle: handle,
            _marker: PhantomData
        }
    }

    pub fn with<F: FnOnce(GLuint)>(&self, func: F) {
        unsafe {        
            gl::BindVertexArray(self.handle);
            func(self.handle);
            gl::BindVertexArray(0);
        }
    }
}

impl<V: GLVertex> Drop for GLVertexArray<V> {
    fn drop(&mut self) {
        unsafe{ gl::DeleteVertexArrays(1, &self.handle) };
    }
}


#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Vertex,
    Fragment
}

impl ShaderType {
    pub fn to_gl_enum(self) -> GLenum {
        match self {
            ShaderType::Vertex => gl::VERTEX_SHADER,
            ShaderType::Fragment => gl::FRAGMENT_SHADER
        }
    }
}

/// RAII wrapper around OpenGL shader
pub struct GLShader {
    pub handle: GLuint,
    pub shader_type: ShaderType
}

impl GLShader {
    pub fn new(shader_type: ShaderType, shader_source: &str) -> Result<GLShader, GLError> {
        unsafe {
            let handle = gl::CreateShader(shader_type.to_gl_enum());

            // Load the shader source into GL, giving it the string pointer and string length, and then compile it.
            gl::ShaderSource(handle, 1, &(shader_source.as_ptr() as *const GLchar), &(shader_source.len() as GLint));
            gl::CompileShader(handle);

            // Check for compile errors and return appropriate value
            let mut status = 0;
            gl::GetShaderiv(handle, gl::COMPILE_STATUS, &mut status);
            if status == gl::FALSE as GLint {
                let mut info_log_length = 0;
                gl::GetShaderiv(handle, gl::INFO_LOG_LENGTH, &mut info_log_length);

                // Create a buffer for GL's error log
                let mut info_log: Vec<u8> = vec![0; info_log_length as usize];
                gl::GetShaderInfoLog(handle, info_log_length, ptr::null_mut(), info_log.as_mut_ptr() as *mut GLchar);

                // Clean up the shader so that it doesn't leak
                gl::DeleteShader(handle);

                // Turn the raw error buffer into a String
                let string_info_log = String::from_utf8_unchecked(info_log);
                Err(string_info_log)
            } else {
                Ok(GLShader{
                    handle: handle,
                    shader_type: shader_type
                })
            }
        }
    }
}

impl Drop for GLShader {
    fn drop(&mut self) {
        unsafe { gl::DeleteShader(self.handle) };
    }
}

/// RAII wrapper around OpenGL program
pub struct GLProgram {
    pub handle: GLuint
}

impl GLProgram {
    pub fn new(vertex_shader: &GLShader, fragment_shader: &GLShader) -> Result<GLProgram, GLError> {
        use std::iter;

        GLProgram::from_program_iter(iter::once(vertex_shader).chain(iter::once(fragment_shader)))
    }

    pub fn from_program_iter<'a, I>(iter: I) -> Result<GLProgram, GLError> 
            where I: Iterator<Item = &'a GLShader> + Clone {
        unsafe {
            // Clone the shader iterator so that we can have an iterator to detach the shaders from the program
            let cleanup_iter = iter.clone();
            let program = gl::CreateProgram();

            // Attach each shader to the program
            for shader in iter {
                gl::AttachShader(program, shader.handle);
            }

            gl::LinkProgram(program);

            let mut is_linked = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut is_linked);
            if is_linked == gl::FALSE as GLint {
                let mut info_log_length = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut info_log_length);

                // Create a buffer for GL's error log, accounting for the null pointer
                let mut info_log: Vec<u8> = vec![0; info_log_length as usize];
                gl::GetProgramInfoLog(program, info_log_length, ptr::null_mut(), info_log.as_mut_ptr() as *mut GLchar);

                // Clean up the program so that it doesn't leak. We don't need to detach the shaders from the
                // program in this case, as there is no more program for the shaders to be attached to.
                gl::DeleteProgram(program);

                // Turn the raw error buffer into a String
                let string_info_log = String::from_utf8_unchecked(info_log);
                Err(string_info_log)
            } else {
                // Detach the shaders from the program
                for shader in cleanup_iter {
                    gl::DetachShader(program, shader.handle);
                }

                Ok(GLProgram{handle: program})
            }
        }
    }

    pub fn with<F: FnOnce(GLuint)>(&self, func: F) {
        unsafe {
            gl::UseProgram(self.handle);
            func(self.handle);
            gl::UseProgram(0);
        }
    }
}

impl Drop for GLProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.handle) };
    }
}
