//////////////////////////////////////////////////
// Using

use image;
use std::mem::size_of;

use crate::gl;
use crate::gl::types::*;
use crate::Gl;

//////////////////////////////////////////////////
// Resources

#[allow(drop_bounds)]
pub trait GlResource: Drop {
    fn release(&mut self);
}

#[derive(Debug, Default)]
pub struct GlVertexArrayObject {
    gl: Option<Gl>,
    id: GLuint,
    active_slots: [bool; 32],
}

#[derive(Debug, Default)]
pub struct GlVertexBuffer<T: Default> {
    gl: Option<Gl>,
    id: GLuint,
    count: usize,
    max_count: usize,
    phantom: std::marker::PhantomData<T>,
}

#[derive(Debug, Default)]
pub struct GlIndexBuffer {
    gl: Option<Gl>,
    id: GLuint,
    count: usize,
    max_count: usize,
}

#[derive(Debug, Default)]
pub struct GlUniformBuffer<T: Default> {
    gl: Option<Gl>,
    id: GLuint,
    active_slots: [bool; 32],
    phantom: std::marker::PhantomData<T>,
}

#[derive(Debug, Default)]
pub struct GlTexture {
    gl: Option<Gl>,
    id: GLuint,
    active_slots: [bool; 32],
}

#[derive(Debug, Default)]
pub struct GlShader {
    gl: Option<Gl>,
    vs: GLuint,
    fs: GLuint,
    program: GLuint,
}

//////////////////////////////////////////////////
// Vertex Array Object

impl GlVertexArrayObject {
    pub fn new(gl: &Gl) -> GlVertexArrayObject {
        let mut id: GLuint = 0;
        unsafe {
            gl.GenVertexArrays(1, &mut id as _);
            if !check_error(gl, "Failed to create vertex array object") {
                log::debug!("Created vertex array object {}", id);
            }
        }
        GlVertexArrayObject {
            gl: Some(gl.clone()),
            id,
            ..Default::default()
        }
    }

    pub fn bind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindVertexArray(self.id);
            check_error(gl, "Failed to bind vertex array");
        }
    }

    pub fn unbind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindVertexArray(0);
            check_error(gl, "Failed to unbind vertex array");
        }
    }

    pub fn bind_attrib<T: Default>(&mut self, vbo: &GlVertexBuffer<T>, slot: GLuint, count: GLint, type_: GLenum, normalized: GLboolean, offset: usize, stride: usize, divisor: GLuint) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBuffer(gl::ARRAY_BUFFER, vbo.id);
            check_error(gl, "Failed to bind vertex buffer");
            gl.VertexAttribPointer(slot, count, type_, normalized, stride as i32, offset as *const () as *const _);
            check_error(gl, "Failed to set vertex attrib");
            gl.VertexAttribDivisor(slot, divisor);
            check_error(gl, "Failed to set vertex divisor");
            gl.EnableVertexAttribArray(slot);
            check_error(gl, "Failed to enable vertex attrib");
            gl.BindBuffer(gl::ARRAY_BUFFER, 0);
        }
        self.active_slots[slot as usize] = true;
    }

    pub fn clear_attribs(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            self.active_slots.iter_mut().enumerate().for_each(|(slot, active)| {
                if *active {
                    gl.VertexAttribDivisor(slot as GLuint, 0);
                    gl.DisableVertexAttribArray(slot as GLuint);
                    check_error(gl, "Failed to unbind attrib");
                    *active = false;
                }
            });
        }
    }
}

//////////////////////////////////////////////////
// Vertex Buffer

impl<T: Default> GlVertexBuffer<T> {
    pub fn new(gl: &Gl, usage: GLenum, data: &[T]) -> GlVertexBuffer<T> {
        let mut id: GLuint = 0;
        unsafe {
            gl.GenBuffers(1, &mut id);
            gl.BindBuffer(gl::ARRAY_BUFFER, id);
            gl.BufferData(gl::ARRAY_BUFFER, (data.len() * size_of::<T>()) as GLsizeiptr, data.as_ptr() as *const _, usage);
            gl.BindBuffer(gl::ARRAY_BUFFER, 0);
            if !check_error(gl, "Failed to create vertex buffer") {
                log::debug!("Created vertex buffer {}", id)
            }
        }
        let count = data.len();
        let max_count = data.len();
        GlVertexBuffer {
            gl: Some(gl.clone()),
            id,
            phantom: std::marker::PhantomData,
            count,
            max_count,
        }
    }

    pub fn update(&mut self, data: &[T]) {
        assert!(data.len() <= self.max_count, "Update data must fit into buffer");
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBuffer(gl::ARRAY_BUFFER, self.id);
            gl.BufferSubData(gl::ARRAY_BUFFER, 0, (data.len() * size_of::<T>()) as GLsizeiptr, data.as_ptr() as *const _);
            gl.BindBuffer(gl::ARRAY_BUFFER, 0);
            if !check_error(gl, "Failed to update vertex buffer") {
                log::debug!("Updated vertex buffer {}", self.id)
            }
        }
        self.count = data.len();
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn max_count(&self) -> usize {
        self.max_count
    }
}

//////////////////////////////////////////////////
// Index Buffer

impl GlIndexBuffer {
    pub fn new(gl: &Gl, usage: GLenum, indices: &[u32]) -> GlIndexBuffer {
        let mut id: GLuint = 0;
        unsafe {
            gl.GenBuffers(1, &mut id);
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, id);
            gl.BufferData(gl::ELEMENT_ARRAY_BUFFER, (indices.len() * size_of::<u32>()) as GLsizeiptr, indices.as_ptr() as *const _, usage);
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            if !check_error(gl, "Failed to create index buffer") {
                log::debug!("Created index buffer {}", id)
            }
        }
        let count = indices.len();
        let max_count = indices.len();
        GlIndexBuffer {
            gl: Some(gl.clone()),
            id,
            count,
            max_count,
        }
    }

    pub fn bind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
            check_error(gl, "Failed to bind index buffer");
        }
    }

    pub fn unbind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            check_error(gl, "Failed to unbind index buffer");
        }
    }

    pub fn update(&mut self, indices: &[u32]) {
        assert!(indices.len() <= self.max_count, "Update data must fit into buffer");
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.id);
            gl.BufferSubData(gl::ELEMENT_ARRAY_BUFFER, 0, (indices.len() * size_of::<u32>()) as GLsizeiptr, indices.as_ptr() as *const _);
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            if !check_error(gl, "Failed to update index buffer") {
                log::debug!("Updated index buffer {}", self.id)
            }
        }
        self.count = indices.len();
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn max_count(&self) -> usize {
        self.max_count
    }
}

//////////////////////////////////////////////////
// Uniform Buffer

impl<T: Default> GlUniformBuffer<T> {
    pub fn new(gl: &Gl, usage: GLenum, data: &T) -> GlUniformBuffer<T> {
        let mut id: GLuint = 0;
        unsafe {
            gl.GenBuffers(1, &mut id);
            gl.BindBuffer(gl::UNIFORM_BUFFER, id);
            gl.BufferData(gl::UNIFORM_BUFFER, size_of::<T>() as GLsizeiptr, data as *const T as *const _, usage);
            gl.BindBuffer(gl::UNIFORM_BUFFER, 0);
            if !check_error(gl, "Failed to create index buffer") {
                log::debug!("Created uniform buffer {}", id)
            }
        }
        GlUniformBuffer {
            gl: Some(gl.clone()),
            id,
            phantom: std::marker::PhantomData,
            ..Default::default()
        }
    }

    pub fn bind(&mut self, unit: GLuint) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBufferBase(gl::UNIFORM_BUFFER, unit, self.id);
            check_error(gl, "Failed to bind uniform buffer");
        }
        self.active_slots[unit as usize] = true;
    }

    pub fn unbind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            self.active_slots.iter_mut().enumerate().for_each(|(slot, active)| {
                if *active {
                    gl.BindBufferBase(gl::UNIFORM_BUFFER, slot as GLuint, 0);
                    check_error(gl, "Failed to unbind uniform buffer");
                    *active = false;
                }
            });
        }
    }

    pub fn update(&mut self, data: &T) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBuffer(gl::UNIFORM_BUFFER, self.id);
            gl.BufferSubData(gl::UNIFORM_BUFFER, 0, size_of::<T>() as GLsizeiptr, data as *const T as *const _);
            gl.BindBuffer(gl::UNIFORM_BUFFER, 0);
            if !check_error(gl, "Failed to update uniform buffer") {
                log::debug!("Updated uniform buffer {}", self.id)
            }
        }
    }
}

//////////////////////////////////////////////////
// Texture

impl GlTexture {
    pub fn new<P, Container>(gl: &Gl, images: &[image::ImageBuffer<P, Container>]) -> GlTexture
    where
        P: image::PixelWithColorType + 'static,
        P::Subpixel: 'static,
        Container: std::ops::Deref<Target = [P::Subpixel]>,
    {
        // all textures need same size
        assert!(!images.is_empty());
        assert!(images.windows(2).all(|w| w[0].dimensions() == w[1].dimensions()));
        // get specs from first image
        let img = images.first().unwrap();
        let pixel_type = if size_of::<P::Subpixel>() == 1 { gl::UNSIGNED_BYTE } else { gl::UNSIGNED_SHORT };
        let (format, internal_format) = match <P as image::PixelWithColorType>::COLOR_TYPE {
            image::ColorType::L8 => (gl::RED, gl::R8),
            image::ColorType::Rgb8 => (gl::RGB, gl::RGB8),
            image::ColorType::Rgb16 => (gl::RGB, gl::RGBA16F),
            image::ColorType::Rgba8 => (gl::RGBA, gl::RGBA8),
            image::ColorType::Rgba16 => (gl::RGBA, gl::RGBA16F),
            _ => unimplemented!(),
        };
        let num_mip_map = 1 + (img.width().min(img.height()) as f32).log2().floor() as i32;

        let mut id: GLuint = 0;
        unsafe {
            gl.GenTextures(1, &mut id);
            gl.BindTexture(gl::TEXTURE_2D_ARRAY, id);
            gl.TexStorage3D(
                gl::TEXTURE_2D_ARRAY,
                num_mip_map,
                internal_format,
                img.width() as GLsizei,
                img.height() as GLsizei,
                images.len() as GLsizei,
            );
            images.iter().enumerate().for_each(|(i, img)| {
                gl.TexSubImage3D(
                    gl::TEXTURE_2D_ARRAY,
                    0,
                    0,
                    0,
                    i as GLint,
                    img.width() as GLsizei,
                    img.height() as GLsizei,
                    1,
                    format,
                    pixel_type,
                    img.as_ptr() as *const _,
                );
            });
            gl.TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_MAG_FILTER, gl::LINEAR as GLint);
            gl.TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_MIN_FILTER, gl::LINEAR as GLint);
            gl.TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
            gl.TexParameteri(gl::TEXTURE_2D_ARRAY, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);
            if !check_error(gl, "Failed to create texture array") {
                log::debug!("Created texture array {}", id)
            }

            gl.GenerateMipmap(gl::TEXTURE_2D_ARRAY);
            if !check_error(gl, "Failed to create texture mipmapping") {
                log::debug!("Created mipmapping for texture {}", id)
            }

            gl.BindTexture(gl::TEXTURE_2D_ARRAY, 0);
        }
        GlTexture {
            gl: Some(gl.clone()),
            id,
            ..Default::default()
        }
    }

    pub fn bind(&mut self, unit: GLuint) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.ActiveTexture(gl::TEXTURE0 + unit as GLuint);
            gl.BindTexture(gl::TEXTURE_2D_ARRAY, self.id);
            check_error(gl, "Failed to bind texture");
        }
        self.active_slots[unit as usize] = true;
    }

    pub fn unbind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            self.active_slots.iter_mut().enumerate().for_each(|(slot, active)| {
                if *active {
                    gl.ActiveTexture(gl::TEXTURE0 + slot as GLuint);
                    gl.BindTexture(gl::TEXTURE_2D_ARRAY, 0);
                    check_error(gl, "Failed to unbind texture");
                    *active = false;
                }
            });
        }
    }
}

//////////////////////////////////////////////////
// Shader

impl GlShader {
    pub fn new(gl: &Gl, vert: &[u8], frag: &[u8]) -> GlShader {
        // ensure null termination
        let vert_string = std::ffi::CString::new(vert).expect("Failed to convert to C-String");
        let frag_string = std::ffi::CString::new(frag).expect("Failed to convert to C-String");
        let vert = vert_string.as_bytes_with_nul();
        let frag = frag_string.as_bytes_with_nul();

        unsafe {
            let vs = gl.CreateShader(gl::VERTEX_SHADER);
            if !check_error(gl, "Failed to create shaders") {
                log::debug!("Created vertex shader {}", vs);
            }
            let fs = gl.CreateShader(gl::FRAGMENT_SHADER);
            if !check_error(gl, "Failed to create shaders") {
                log::debug!("Created fragment shader {}", fs);
            }

            gl.ShaderSource(vs, 1, [vert.as_ptr() as *const _].as_ptr(), std::ptr::null());
            gl.CompileShader(vs);
            let mut status = 0;
            gl.GetShaderiv(vs, gl::COMPILE_STATUS, &mut status);
            if status == 0 {
                log::error!("Failed to compile vertex shader");
                print_shader_log(gl, vs);
            } else {
                log::debug!("Compiled vertex shader {}", vs);
            }

            gl.ShaderSource(fs, 1, [frag.as_ptr() as *const _].as_ptr(), std::ptr::null());
            gl.CompileShader(fs);
            let mut status = 0;
            gl.GetShaderiv(fs, gl::COMPILE_STATUS, &mut status);
            if status == 0 {
                log::error!("Failed to compile fragment shader");
                print_shader_log(gl, fs);
            } else {
                log::debug!("Compiled fragment shader {}", fs);
            }

            let program = gl.CreateProgram();
            if !check_error(gl, "Failed to create shader program") {
                log::debug!("Created shader program {}", program);
            }

            gl.AttachShader(program, vs);
            if !check_error(gl, "Failed to attach vertex shader") {
                log::debug!("Attached vertex shader {} to program {}", vs, program);
            }

            gl.AttachShader(program, fs);
            if !check_error(gl, "Failed to attach fragment shader") {
                log::debug!("Attached fragment shader {} to program {}", fs, program);
            }

            gl.LinkProgram(program);
            //print_program_info(gl, program);
            if !check_error(gl, "Failed to link program") {
                log::debug!("Linked program {}", program);
            }

            GlShader {
                gl: Some(gl.clone()),
                vs,
                fs,
                program,
            }
        }
    }

    pub fn bind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.UseProgram(self.program);
            check_error(gl, "Failed to bind program");
        }
    }

    pub fn unbind(&mut self) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.UseProgram(0);
            check_error(gl, "Failed to unbind program");
        }
    }

    pub fn link_uniform(&mut self, unit: GLuint, location: &str) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            let c_location = std::ffi::CString::new(location).unwrap();
            let loc = gl.GetUniformBlockIndex(self.program, c_location.as_ptr());
            gl.UniformBlockBinding(self.program, loc, unit);
            check_error(gl, "Failed to bind uniform");
        }
    }

    pub fn link_texture(&mut self, unit: GLint, location: &str) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            let c_location = std::ffi::CString::new(location).unwrap();
            let loc = gl.GetUniformLocation(self.program, c_location.as_ptr());
            gl.Uniform1i(loc, unit);
            check_error(gl, "Failed to bind texture");
        }
    }

    pub fn draw_arrays(&mut self, mode: GLenum, vertex_count: usize) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.DrawArrays(mode, 0, vertex_count as GLsizei);
            check_error(gl, "Failed to draw");
        }
    }
    pub fn draw_elements(&mut self, mode: GLenum, index_count: usize) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.DrawElements(mode, index_count as GLsizei, gl::UNSIGNED_INT, std::ptr::null::<()>() as *const _);
            check_error(gl, "Failed to draw");
        }
    }

    pub fn draw_elements_instanced(&mut self, mode: GLenum, index_count: usize, instance_count: usize) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.DrawElementsInstanced(mode, index_count as GLsizei, gl::UNSIGNED_INT, std::ptr::null::<()>() as *const _, instance_count as GLsizei);
            check_error(gl, "Failed to draw");
        }
    }
}

//////////////////////////////////////////////////
// Trait Impl GlResource

impl GlResource for GlVertexArrayObject {
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.DeleteVertexArrays(1, [self.id].as_ptr() as *const _);
                if !check_error(gl, "Failed to release vertex array object") {
                    log::debug!("Deleted vertex array object {}", self.id)
                }
            }
        }
        self.gl = None;
    }
}
impl Drop for GlVertexArrayObject {
    fn drop(&mut self) {
        self.release()
    }
}

impl<T: Default> GlResource for GlVertexBuffer<T> {
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.DeleteBuffers(1, &self.id);
                if !check_error(gl, "Failed to release vertex buffer") {
                    log::debug!("Deleted vertex buffer {}", self.id)
                }
            }
        }
        self.gl = None;
    }
}
impl<T: Default> Drop for GlVertexBuffer<T> {
    fn drop(&mut self) {
        self.release()
    }
}

impl GlResource for GlIndexBuffer {
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.DeleteBuffers(1, &self.id);
                if !check_error(gl, "Failed to release index buffer") {
                    log::debug!("Deleted index buffer {}", self.id);
                }
            }
        }
        self.gl = None;
    }
}
impl Drop for GlIndexBuffer {
    fn drop(&mut self) {
        self.release()
    }
}

impl<T: Default> GlResource for GlUniformBuffer<T> {
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.DeleteBuffers(1, &self.id);
                if !check_error(gl, "Failed to release uniform buffer") {
                    log::debug!("Deleted uniform buffer {}", self.id)
                }
            }
        }
        self.gl = None;
    }
}
impl<T: Default> Drop for GlUniformBuffer<T> {
    fn drop(&mut self) {
        self.release()
    }
}

impl GlResource for GlTexture {
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.DeleteTextures(1, &self.id);
                if !check_error(gl, "Failed to release texture") {
                    log::debug!("Deleted texture {}", self.id);
                }
            }
        }
        self.gl = None;
    }
}
impl Drop for GlTexture {
    fn drop(&mut self) {
        self.release()
    }
}

impl GlResource for GlShader {
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe {
                gl.DetachShader(self.program, self.vs);
                if !check_error(gl, "Failed to destroy shaders") {
                    log::debug!("Detached vertex shader {} from program {}", self.vs, self.program);
                }
                gl.DetachShader(self.program, self.fs);
                if !check_error(gl, "Failed to destroy shaders") {
                    log::debug!("Detached fragment shader {} from program {}", self.fs, self.program);
                }
                gl.DeleteShader(self.vs);
                if !check_error(gl, "Failed to destroy shaders") {
                    log::debug!("Deleted vertex shader {}", self.vs);
                }
                gl.DeleteShader(self.fs);
                if !check_error(gl, "Failed to destroy shaders") {
                    log::debug!("Deleted fragment shader {}", self.fs);
                }
                gl.DeleteProgram(self.program);
                if !check_error(gl, "Failed to destroy shaders") {
                    log::debug!("Deleted program {}", self.program);
                }
            }
        }
        self.gl = None;
    }
}
impl Drop for GlShader {
    fn drop(&mut self) {
        self.release()
    }
}

//////////////////////////////////////////////////
// Check error call

#[inline]
#[cfg(debug_assertions)]
pub unsafe fn check_error(gl: &Gl, description: &str) -> bool {
    let mut err = gl.GetError();
    let mut has_error = false;
    while err != gl::NO_ERROR {
        log::error!("{}. ErrorCode {}", description, err);
        err = gl.GetError();
        has_error = true;
    }
    has_error
}

#[inline]
#[cfg(not(debug_assertions))]
pub unsafe fn check_error(_gl: &Gl, _description: &str) -> bool {
    false
}

pub unsafe fn print_shader_log(gl: &Gl, shader: GLuint) {
    let mut buffer = vec![0u8; 2048];
    let mut length = 0;
    gl.GetShaderInfoLog(shader, (buffer.len() * size_of::<u8>()) as GLsizei, &mut length, buffer.as_mut_ptr() as *mut _);
    log::debug!("{}", &String::from_utf8_lossy(&buffer[..length as usize]));
}

pub unsafe fn print_program_info(gl: &Gl, program: GLuint) {
    let mut buffer = vec![0u8; 2048];
    let mut length = 0;
    gl.GetProgramInfoLog(program, (buffer.len() * size_of::<u8>()) as GLsizei, &mut length, buffer.as_mut_ptr() as *mut _);
    log::debug!("{}", &String::from_utf8_lossy(&buffer[..length as usize]));
}
