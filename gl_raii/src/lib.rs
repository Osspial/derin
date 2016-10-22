extern crate gl;

use gl::types::*;

use std::ptr;
use std::marker::PhantomData;
use std::cell::{Cell};

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
    Vec4(GLPrim),
    Mat2(GLPrim),
    Mat3(GLPrim),
    Mat4(GLPrim)
}

impl GLSLType {
    /// Gets the underlying GLSL primitive
    fn prim(self) -> GLPrim {
        use self::GLSLType::*;

        match self {
            Single(p) |
            Vec2(p)   |
            Vec3(p)   |
            Vec4(p)   |
            Mat2(p)   |
            Mat3(p)   |
            Mat4(p)  => p
        }
    }

    /// Gets the number of primitives stored in this type
    fn len(self) -> GLint {
        use self::GLSLType::*;

        match self {
            Single(_) => 1,
            Vec2(_)   => 2,
            Vec3(_)   => 3,
            Vec4(_)   => 4,
            Mat2(_)   => 4,
            Mat3(_)   => 9,
            Mat4(_)   => 16
        }
    }


    /// Gets the byte size of the type represented by this enum
    fn size(self) -> usize {
        use self::GLSLType::*;

        match self {
            Single(p) => p.size(),
            Vec2(p)   => p.size() * 2,
            Vec3(p)   => p.size() * 3,
            Vec4(p)   => p.size() * 4,
            Mat2(p)   => p.size() * 4,
            Mat3(p)   => p.size() * 9,
            Mat4(p)   => p.size() * 16
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

    fn size(self) -> usize {
        use self::GLPrim::*;

        match self {
            Byte     |
            NByte    |
            UByte    |
            NUByte  => 1,
            Short    |
            NShort   |
            UShort   |
            NUShort => 2,
            Int      |
            NInt     |
            UInt     |
            NUInt    |
            Float   => 4,
            Double  => 8
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
    fn modify(&mut self) -> BufModder<T>;

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

    fn modify(&mut self) -> BufModder<V> {
        self.0.modify()
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

    fn modify(&mut self) -> BufModder<GLushort> {
        self.0.modify()
    }

    #[inline]
    fn buffer_type() -> GLenum {
        gl::ELEMENT_ARRAY_BUFFER
    }
}

/// RAII wrapper around OpenGL buffers
struct Buffer<T: Copy, B: GLBuffer<T>> {
    handle: GLuint,
    data: Vec<T>,
    usage: BufferUsage,
    _marker: PhantomData<(T, B)>
}

impl<T: Copy, B: GLBuffer<T>> Buffer<T, B> {
    fn new(capacity: usize, usage: BufferUsage) -> Buffer<T, B> {
        unsafe {
            use std::mem::size_of;

            let mut handle = 0;
            gl::GenBuffers(1, &mut handle);

            reset_vao_bind();

            gl::BindBuffer(B::buffer_type(), handle);
            gl::BufferData(B::buffer_type(), (size_of::<T>() * capacity) as GLsizeiptr, ptr::null(), usage.to_gl_enum());
            gl::BindBuffer(B::buffer_type(), 0);

            Buffer {
                handle: handle,
                data: Vec::with_capacity(capacity),
                usage: usage,
                _marker: PhantomData
            }
        }
    }

    fn from_slice(slice: &[T], usage: BufferUsage) -> Buffer<T, B> {
        unsafe {
            let mut handle = 0;
            gl::GenBuffers(1, &mut handle);

            reset_vao_bind();

            gl::BindBuffer(B::buffer_type(), handle);
            gl::BufferData(B::buffer_type(), byte_slice_size(slice), slice.as_ptr() as *const _, usage.to_gl_enum());
            gl::BindBuffer(B::buffer_type(), 0);

            let data = slice.to_vec();

            Buffer {
                handle: handle,
                data: data,
                usage: usage,
                _marker: PhantomData
            }
        }
    }

    fn modify(&mut self) -> BufModder<T> {
        BufModder {
            buffer_type: B::buffer_type(),
            buffer_handle: self.handle,
            buffer_usage: self.usage,

            old_vec_len: self.data.len(),
            buffer_vec: &mut self.data
        }
    }
}

pub struct BufModder<'a, T> 
        where T: 'a + Copy {
    buffer_type: GLenum,
    buffer_handle: GLuint,
    buffer_usage: BufferUsage,

    buffer_vec: &'a mut Vec<T>,
    old_vec_len: usize
}

impl<'a, T> BufModder<'a, T>
        where T: 'a + Copy {
    pub fn buffer_vec(&mut self) -> &mut Vec<T> {
        &mut self.buffer_vec
    }
}

impl<'a, T> Drop for BufModder<'a, T>
        where T: 'a + Copy {
    fn drop(&mut self) {
        thread_local!(static BOUND_BUFFER: Cell<GLuint> = Cell::new(0));

        BOUND_BUFFER.with(|bb| unsafe {
            let last_bound = bb.get();
            bb.set(self.buffer_handle);

            reset_vao_bind();

            gl::BindBuffer(self.buffer_type, self.buffer_handle);
            if self.old_vec_len < self.buffer_vec.len() {
                gl::BufferData(
                    self.buffer_type,
                    byte_slice_size(&self.buffer_vec),
                    self.buffer_vec.as_ptr() as *const _,
                    self.buffer_usage.to_gl_enum()
                );
            } else {
                gl::BufferSubData(
                    self.buffer_type,
                    0,
                    byte_slice_size(&self.buffer_vec),
                    self.buffer_vec.as_ptr() as *const _
                )
            }
            gl::BindBuffer(self.buffer_type, last_bound);

            bb.set(last_bound);
        });
    }
}

pub unsafe trait GLUniformBlock: Clone {
    unsafe fn data_types() -> &'static [UniformBlockEntry];
}

pub struct UniformBlockEntry {
    pub entry_name: &'static str,
    pub entry_type: GLSLType,
    /// The element's byte offset into the struct. Please note that this is different from the offset
    /// into the uniform buffer.
    pub offset: usize
}

#[derive(Debug)]
struct UBEntryLocationInfo {
    entry_size: usize,
    struct_offset: usize,
    buffer_offset: GLsizeiptr
}

impl UBEntryLocationInfo {
    #[inline]
    fn new(entry_size: usize, struct_offset: usize, buffer_offset: GLsizeiptr) -> UBEntryLocationInfo {
        UBEntryLocationInfo {
            entry_size: entry_size,
            struct_offset: struct_offset,
            buffer_offset: buffer_offset
        }
    }
}

pub struct GLUniformBuffer<U: GLUniformBlock> {
    block: U,
    handle: GLuint,
    uniform_block_index: GLuint,
    /// The offsets into the uniform buffer for writing data to the buffer
    offsets: Vec<UBEntryLocationInfo>
}

impl<U: GLUniformBlock> GLUniformBuffer<U> {
    pub fn new(block: U, uniform_block_name: &str, program: &GLProgram) -> GLUniformBuffer<U> {
        unsafe {
            let mut offsets = Vec::with_capacity(U::data_types().len());

            let uniform_block_name = uniform_block_name.to_owned() + "\0";
            let uniform_block_index = gl::GetUniformBlockIndex(program.handle, uniform_block_name.as_ptr() as *const _);

            reset_vao_bind();

            let mut handle = 0;
            gl::GenBuffers(1, &mut handle);
            gl::BindBuffer(gl::UNIFORM_BUFFER, handle);

            for ube in U::data_types() {
                let entry_name = ube.entry_name.to_owned() + "\0";

                let mut uniform_index = 0;
                gl::GetUniformIndices(program.handle, 1, &(entry_name.as_ptr() as *const _), &mut uniform_index);
                assert!(gl::INVALID_INDEX != uniform_index);

                let mut uniform_offset = 0;
                gl::GetActiveUniformsiv(program.handle, 1, &uniform_index, gl::UNIFORM_OFFSET, &mut uniform_offset);
                assert!(-1 != uniform_offset);
                offsets.push(UBEntryLocationInfo::new(ube.entry_type.size(), ube.offset, uniform_offset as GLsizeiptr));
            }

            let mut buffer_size = 0;
            gl::GetActiveUniformBlockiv(program.handle, uniform_block_index, gl::UNIFORM_BLOCK_DATA_SIZE, &mut buffer_size);
            gl::BufferData(gl::UNIFORM_BUFFER, buffer_size as GLsizeiptr, ptr::null(), gl::STREAM_DRAW);

            let u_buffer = GLUniformBuffer {
                block: block,
                handle: handle,
                uniform_block_index: uniform_block_index,
                offsets: offsets
            };
            u_buffer.update_data();
            u_buffer.bind_to_program(program);
            gl::BindBuffer(gl::UNIFORM_BUFFER, 0);

            u_buffer
        }
    }

    /// Upload the cached data to the GPU. The caller of this function must ensure that the proper
    /// buffer is bound.
    unsafe fn update_data(&self) {
        let block_ptr = &self.block as *const U as *const GLvoid;
        
        for off in self.offsets.iter() {
            gl::BufferSubData(gl::UNIFORM_BUFFER, off.buffer_offset, off.entry_size as GLsizeiptr, block_ptr.offset(off.struct_offset as isize));
        }
    }

    pub fn sub_data(&mut self, block: &U) {
        unsafe {
            reset_vao_bind();

            gl::BindBuffer(gl::UNIFORM_BUFFER, self.handle);
            
            self.block = block.clone();
            self.update_data();

            gl::BindBuffer(gl::UNIFORM_BUFFER, 0);
        }
    }

    pub fn bind_to_program(&self, program: &GLProgram) {
        unsafe {
            let mut buffer_size = 0;
            gl::GetActiveUniformBlockiv(program.handle, self.uniform_block_index, gl::UNIFORM_BLOCK_DATA_SIZE, &mut buffer_size);

            reset_vao_bind();

            // This is re-using the uniform block index as the uniform binding index, mainly because I'm not
            // aware of any major consequences to doing so and it's easier.
            gl::UniformBlockBinding(program.handle, self.uniform_block_index, self.uniform_block_index);
            gl::BindBufferRange(
                gl::UNIFORM_BUFFER, 
                self.uniform_block_index, 
                self.handle, 
                0,
                buffer_size as GLsizeiptr
            );
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
        let vao: GLVertexArray<V>;
        
        unsafe{ 
            gl::GenVertexArrays(1, &mut handle);
            vao = GLVertexArray {
                handle: handle,
                _marker: PhantomData
            };

            vao.with(|_| {
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
                                glsl_type.len(),
                                gl::DOUBLE,
                                stride,
                                offset
                            );
                        } else {
                            gl::VertexAttribPointer(
                                attrib.index, 
                                glsl_type.len(), 
                                prim.to_gl_enum(), 
                                prim.is_normalized(),
                                stride,
                                offset
                            );
                        }
                    } else {
                        gl::VertexAttribIPointer(
                            attrib.index,
                            glsl_type.len(),
                            prim.to_gl_enum(),
                            stride,
                            offset
                        );
                    }
                }

                gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            });
        }

        assert!(handle != 0);

        vao
    }

    pub fn with<F: FnOnce(GLuint)>(&self, func: F) {
        BOUND_VAO.with(|bv| unsafe{
            let last_bound = bv.0.get();
            let is_nested_bind = bv.1.get();

            // The logic in here is effectively the same as the logic in GLProgram::with(), so look at that
            // for documentation on what this if/else block is for.
            if last_bound == self.handle && !is_nested_bind {
                bv.1.set(true);
                func(self.handle);
                bv.1.set(false);

            } else if is_nested_bind {
                bv.0.set(self.handle);

                gl::BindVertexArray(self.handle);
                func(self.handle);
                gl::BindVertexArray(last_bound);

                bv.0.set(last_bound);

            } else {
                bv.0.set(self.handle);
                bv.1.set(true);

                gl::BindVertexArray(self.handle);
                func(self.handle);

                bv.1.set(false);
            }
        })
    }
}

/// Whenever we bind a buffer, we change VAO state. This function must be called before any
/// non-vao-changing buffer binds so that it doesn't cause adverse effects on a VAO.
fn reset_vao_bind() {
    BOUND_VAO.with(|bv| {
        if 0 != bv.0.get() {
            bv.0.set(0);
            unsafe{ gl::BindVertexArray(0) };
        }
    })
}

thread_local!(static BOUND_VAO: (Cell<GLuint>, Cell<bool>) = (Cell::new(0), Cell::new(false)));

impl<V: GLVertex> Drop for GLVertexArray<V> {
    fn drop(&mut self) {
        unsafe{ gl::DeleteVertexArrays(1, &self.handle) };
    }
}

pub enum TextureFormat {
    R8,
    RGB8
}

pub struct GLTexture {
    handle: GLuint
}

impl GLTexture {
    pub fn new(width: u32, height: u32, pixels: &[u8], format: TextureFormat) -> GLTexture {
        let tex = GLTexture::empty();
        tex.swap_data(width, height, pixels, format);
        tex
    }

    pub fn empty() -> GLTexture {
        unsafe {
            debug_assert_eq!(gl::TEXTURE0 as i32, {
                let mut active_texture = 0;
                gl::GetIntegerv(gl::ACTIVE_TEXTURE, &mut active_texture);
                active_texture
            });

            debug_assert_eq!(0, {
                let mut texture_binding_2d = 0;
                gl::GetIntegerv(gl::TEXTURE_BINDING_2D, &mut texture_binding_2d);
                texture_binding_2d
            });

            let mut handle = 0;
            gl::GenTextures(1, &mut handle);

            let tex = GLTexture {
                handle: handle
            };

            tex
        }
    }

    pub fn swap_data(&self, width: u32, height: u32, pixels: &[u8], format: TextureFormat) {
        let (internal_format, format) = match format {
            TextureFormat::R8 => (gl::R8 as i32, gl::RED),
            TextureFormat::RGB8 => (gl::RGB8 as i32, gl::RGB)
        };

        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.handle);
            gl::TexImage2D(gl::TEXTURE_2D, 0, internal_format, width as i32, height as i32,
                0, format, gl::UNSIGNED_BYTE, pixels.as_ptr() as *const GLvoid);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }
}

impl Drop for GLTexture {
    fn drop(&mut self) {
        unsafe{ gl::DeleteTextures(1, &self.handle) };
    }
}

pub struct GLSampler {
    handle: GLuint
}

impl GLSampler {
    pub fn new() -> GLSampler {
        unsafe {
            let mut handle = 0;
            gl::GenSamplers(1, &mut handle);
            gl::SamplerParameteri(handle, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::SamplerParameteri(handle, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::SamplerParameteri(handle, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::SamplerParameteri(handle, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            GLSampler {
                handle: handle
            }
        }
    }

    pub fn with_texture(&self, tex_unit: GLuint, texture: &GLTexture) {
        assert!(tex_unit != 0);
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + tex_unit);
            gl::BindTexture(gl::TEXTURE_2D, texture.handle);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindSampler(tex_unit, self.handle);
        }
    }
}

impl Drop for GLSampler {
    fn drop(&mut self) {
        unsafe{ gl::DeleteSamplers(1, &self.handle) };
    }
}

impl Default for GLSampler {
    fn default() -> Self {
        Self::new()
    }
}


#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Vertex,
    Geometry,
    Fragment
}

impl ShaderType {
    pub fn to_gl_enum(self) -> GLenum {
        match self {
            ShaderType::Vertex => gl::VERTEX_SHADER,
            ShaderType::Geometry => gl::GEOMETRY_SHADER,
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

    pub fn new_geometry(vertex_shader: &GLShader, geometry_shader: &GLShader, fragment_shader: &GLShader) -> Result<GLProgram, GLError> {
        use std::iter;

        GLProgram::from_program_iter(
            iter::once(vertex_shader).chain(
            iter::once(geometry_shader).chain(
            iter::once(fragment_shader))))
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
        thread_local!(static BOUND_PROGRAM: (Cell<GLuint>, Cell<bool>) = (Cell::new(0), Cell::new(false)));

        BOUND_PROGRAM.with(|bp| unsafe {
            let last_bound = bp.0.get();
            let is_nested_bind = bp.1.get();
            
            // This logic is set up so that we can minimize the number of program binds we have to do, as
            // binding stuff in OpenGL is fairly expensive. The first case is for when `self` is the bound
            // program, in which case we just need to run the given function. The second case is for when
            // a `with` function is being called *inside* of another `with` function, in which case we must
            // bind the current program, run `func`, and then re-bind the original program. This is the most
            // expensive case. The third case is for when `self` isn't bound and we aren't inside another
            // `with` function, in which case we can just bind `self` and leave it bound.
            if last_bound == self.handle && !is_nested_bind {
                bp.1.set(true);
                func(self.handle);
                bp.1.set(false);

            } else if is_nested_bind {
                bp.0.set(self.handle);

                gl::UseProgram(self.handle);
                func(self.handle);
                gl::UseProgram(last_bound);

                bp.0.set(last_bound);

            } else {
                bp.0.set(self.handle);
                bp.1.set(true);

                gl::UseProgram(self.handle);
                func(self.handle);

                bp.1.set(false);
            }
        })
    }
}

impl Drop for GLProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.handle) };
    }
}
