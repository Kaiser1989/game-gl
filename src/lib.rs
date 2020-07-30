//////////////////////////////////////////////////
// Public API

pub mod prelude {
    pub use crate::{GameLoop, Runner, Gl, input::InputEvent};
    pub use crate::gl;
    pub use crate::gl::types::*;
}


//////////////////////////////////////////////////
// OpenGL binding

pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}


//////////////////////////////////////////////////
// Using

use std::ffi::CStr;
use std::time::Instant;
use std::rc::Rc;
use std::mem::size_of;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};

#[cfg(target_os = "android")]
use glutin::platform::android::AndroidEventLoop as EventLoop;
#[cfg(not(target_os = "android"))]
use glutin::event_loop::EventLoop;

use crate::gl::types::*;
use crate::input::{ InputEvent, CursorEvent, MouseEvent };


//////////////////////////////////////////////////
// Types

pub type Gl = Rc<gl::Gles2>; 


//////////////////////////////////////////////////
// Traits

pub trait Runner {

    fn init(&mut self);

    fn cleanup(&mut self);

    fn pause(&mut self);

    fn resume(&mut self);

    fn input(&mut self, input_events: &[InputEvent]);

    fn update(&mut self, elapsed_time: f32);

    fn render(&mut self, gl: &Gl);

    fn create_device(&mut self, gl: &Gl);

    fn destroy_device(&mut self, gl: &Gl);

    fn resize_device(&mut self, gl: &Gl, width: u32, height: u32);
}


//////////////////////////////////////////////////
// Game loop

pub struct GameLoop<T: Runner> {
    runner: T,
    device_ctx: Option<DeviceContext>,
}

impl<T: Runner> GameLoop<T> {

    pub fn new(runner: T) -> GameLoop<T> {
        Self { runner, device_ctx: None }
    }

    pub fn run(&mut self) {
        // call init callback
        self.runner.init();

        // start game time
        let mut time = Instant::now();

        // init input
        let mut input_events: Vec<InputEvent> = Vec::with_capacity(10);

        // init game loop
        let mut event_loop = EventLoop::new();

        // WINDOWS: create context
        #[cfg(not(target_os = "android"))]
        {
            self.device_ctx = Some(DeviceContext::new(&event_loop));
            if let Some(device_ctx) = self.device_ctx.as_mut() {
                // call create device callback
                self.runner.create_device(&device_ctx.gl);

                // call resize device callback
                let resolution = device_ctx.window_context.window().inner_size();
                self.runner.resize_device(&device_ctx.gl, resolution.width, resolution.height)
            }
        }

        // starting game loop
        let mut running = true;
        while running {

            // check glutin events
            event_loop.run_return(|event, _event_loop, control_flow| {
                *control_flow = ControlFlow::Exit;
        
                match event {
                    Event::Resumed => {

                        // ANDROID: only create if native window is available
                        #[cfg(target_os = "android")]
                        {
                            if self.device_ctx.is_none() && ndk_glue::native_window().is_some() {
                                self.device_ctx = Some(DeviceContext::new(_event_loop));
                                if let Some(device_ctx) = self.device_ctx.as_mut() {
                                    // call create device callback
                                    self.runner.create_device(&device_ctx.gl);
                                }
                            }
                        }

                        // call resume callback
                        self.runner.resume();

                    },
                    Event::Suspended => {

                        // call pause callback
                        self.runner.pause();

                        // ANDROID: only destroy if native window is available
                        #[cfg(target_os = "android")]
                        {
                            if self.device_ctx.is_some() && ndk_glue::native_window().is_some() {
                                if let Some(device_ctx) = self.device_ctx.as_mut() {
                                    // call destroy device callback
                                    self.runner.destroy_device(&device_ctx.gl);
                                }
                                self.device_ctx = None;
                            }
                        }

                    },
                    Event::RedrawRequested(_) => {

                        if let Some(device_ctx) = self.device_ctx.as_mut() {
                            // call render callback
                            self.runner.render(&device_ctx.gl);

                            device_ctx.window_context.swap_buffers().unwrap();
                        }
                    },
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(physical_size) => {
                            if let Some(device_ctx) = self.device_ctx.as_mut() {
                                device_ctx.window_context.resize(physical_size);

                                // call resize device callback
                                self.runner.resize_device(&device_ctx.gl, physical_size.width, physical_size.height);
                            }
                        },
                        WindowEvent::CloseRequested => {
                            running = false;
                        },
                        WindowEvent::CursorMoved { position, ..} => {
                            input_events.push(
                                InputEvent::Cursor(
                                    CursorEvent { location: position.into() }
                                )
                            );
                        },
                        WindowEvent::MouseInput { state, button, ..} => {
                            input_events.push(
                                InputEvent::Mouse(
                                    MouseEvent { state: state.into(), button: button.into() }
                                )
                            );
                        },
                        WindowEvent::Touch(touch) => {
                            input_events.push(
                                InputEvent::Touch(
                                    touch.into()
                                )
                            );
                        },
                        WindowEvent::KeyboardInput { input, ..} => {
                            input_events.push(
                                InputEvent::Keyboard(
                                    input.into()
                                )
                            );
                        }
                        _ => (),
                    },
                    Event::MainEventsCleared => {
                        // update time
                        let new_time = Instant::now();
                        let elapsed_time = new_time.duration_since(time).as_millis() as f32 / 1000.0;
                        time = new_time;

                        // process input
                        self.runner.input(&input_events);
                        input_events.clear();

                        // call update callback
                        self.runner.update(elapsed_time);

                        // render call
                        if let Some(device_ctx) = self.device_ctx.as_mut() {

                            // call render callback
                            self.runner.render(&device_ctx.gl);

                            device_ctx.window_context.swap_buffers().unwrap();
                        }
                    },
                    _ => (),
                }
            });
        }

        // WINDOWS: destroy context
        #[cfg(not(target_os = "android"))]
        {
            if let Some(device_ctx) = self.device_ctx.as_mut() {
                // call destroy device callback
                self.runner.destroy_device(&device_ctx.gl);
            }
            self.device_ctx = None;
        }

        // call cleanup callback
        self.runner.cleanup();
    }
}


//////////////////////////////////////////////////
// Internal Device

struct DeviceContext {
    gl: Gl,
    window_context: WindowedContext<PossiblyCurrent>,
}

impl DeviceContext {

    fn new(el: &EventLoopWindowTarget<()>) -> DeviceContext {
        let wb = WindowBuilder::new().with_title("A fantastic window!");
        let window_context = ContextBuilder::new()
            .with_gl(GlRequest::Specific(Platform::opengl_api_support(), Platform::opengl_version_support()))
            //.with_gl_debug_flag(true)
            .with_srgb(Platform::srgb_support())
            .with_vsync(Platform::vsync_support())
            .build_windowed(wb, &el)
            .unwrap();
        let window_context = unsafe { window_context.make_current().unwrap() };
        let gl = Gl::new(gl::Gles2::load_with(|ptr| window_context.get_proc_address(ptr) as *const _));
        let device_ctx = DeviceContext{ gl, window_context };
        device_ctx.print_context();
        device_ctx
    }

    fn print_context(&self) {
        log::info!("Created OpenGL context:");
        log::info!("- Version: {:?}", self.get_string(gl::VERSION));
        log::info!("- {:?}", self.window_context.get_pixel_format());
    }

    // +++ Starting OpenGL functions

    fn get_string(&self, gl_enum: GLenum) -> String {
        unsafe {
            let data = CStr::from_ptr(self.gl.GetString(gl_enum) as *const _).to_bytes().to_vec();
            String::from_utf8(data).unwrap()
        }
    }
}

impl Drop for DeviceContext {
    fn drop(&mut self) {
        // only this reference is allowed
        assert!(Gl::strong_count(&self.gl) == 1, "Error! Unreleased OpenGL resources");
    }
}


//////////////////////////////////////////////////
// Support

struct Platform {}

#[cfg(target_os = "android")]
impl Platform {
    fn srgb_support() -> bool { false }
    fn vsync_support() -> bool { true }
    fn opengl_api_support() -> Api { Api::OpenGlEs }
    fn opengl_version_support() -> (u8, u8) { (2, 0) }
}

#[cfg(not(target_os = "android"))]
impl Platform {
    fn srgb_support() -> bool { true }
    fn vsync_support() -> bool { true }
    fn opengl_api_support() -> Api { Api::OpenGl }
    fn opengl_version_support() -> (u8, u8) { (4, 5) }
}


//////////////////////////////////////////////////
// Input

pub mod input {

#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    Cursor(CursorEvent),
    Mouse(MouseEvent),
    Touch(TouchEvent),
    Keyboard(KeyboardEvent)
}

#[derive(Debug, Copy, Clone)]
pub struct CursorEvent {
    pub location: Location,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseEvent {
    pub state: MouseState,
    pub button: MouseButton,
}

#[derive(Debug, Copy, Clone)]
pub enum MouseState {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other(u8),
}

#[derive(Debug, Copy, Clone)]
pub struct TouchEvent {
    pub state: TouchState,
    pub location: Location,
    pub id: u64,
}

#[derive(Debug, Copy, Clone)]
pub enum TouchState {
    Down,
    Up,
    Move,
    Cancelled,
}

#[derive(Debug, Copy, Clone)]
pub struct KeyboardEvent {
    pub state: KeyState,
    pub key: u32,
}

#[derive(Debug, Copy, Clone)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

impl From<glutin::dpi::PhysicalPosition<f64>> for Location {
    fn from(e: glutin::dpi::PhysicalPosition<f64>) -> Location {
        Location { x: e.x as f32, y: e.y as f32 }
    }
}

impl From<glutin::event::ElementState> for MouseState {
    fn from(e: glutin::event::ElementState) -> MouseState {
        match e { 
            glutin::event::ElementState::Pressed => MouseState::Pressed,
            glutin::event::ElementState::Released => MouseState::Released,
        }
    }
}

impl From<glutin::event::MouseButton> for MouseButton {
    fn from(e: glutin::event::MouseButton) -> MouseButton {
        match e {
            glutin::event::MouseButton::Left => MouseButton::Left,
            glutin::event::MouseButton::Middle => MouseButton::Middle,
            glutin::event::MouseButton::Right => MouseButton::Right,
            glutin::event::MouseButton::Other(x) => MouseButton::Other(x),
        }
    }
}

impl From<glutin::event::Touch> for TouchEvent {
    fn from(e: glutin::event::Touch) -> TouchEvent {
        let glutin::event::Touch { phase, location, id, .. } = e;
        TouchEvent { state: phase.into(), location: location.into(), id }
    }
}

impl From<glutin::event::TouchPhase> for TouchState {
    fn from(e: glutin::event::TouchPhase) -> TouchState {
        match e {
            glutin::event::TouchPhase::Started => TouchState::Down,
            glutin::event::TouchPhase::Ended => TouchState::Up,
            glutin::event::TouchPhase::Moved => TouchState::Move,
            glutin::event::TouchPhase::Cancelled => TouchState::Cancelled,
        }
    }
}

impl From<glutin::event::ElementState> for KeyState {
    fn from(e: glutin::event::ElementState) -> KeyState {
        match e { 
            glutin::event::ElementState::Pressed => KeyState::Pressed,
            glutin::event::ElementState::Released => KeyState::Released,
        }
    }
}

impl From<glutin::event::KeyboardInput> for KeyboardEvent {
    fn from(e: glutin::event::KeyboardInput) -> KeyboardEvent {
        let glutin::event::KeyboardInput { scancode, state, ..} = e;
        KeyboardEvent { state: state.into(), key: scancode }
    }
}

}

pub mod resource {

use super::*;
pub use image;

//////////////////////////////////////////////////
// Resources

pub trait GlResource : Drop {
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
    phantom: std::marker::PhantomData<T>,
}

#[derive(Debug, Default)]
pub struct GlIndexBuffer {
    gl: Option<Gl>,
    id: GLuint,
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
        GlVertexArrayObject { gl: Some(gl.clone()), id, .. Default::default() }
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

    pub fn bind_attrib<T: Default>(
        &mut self, 
        vbo: &GlVertexBuffer<T>,
        slot: GLuint,
        count: GLint,
        type_: GLenum,
        normalized: GLboolean,
        offset: usize,
        stride: usize,
        divisor: GLuint,
    ) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.BindBuffer(gl::ARRAY_BUFFER, vbo.id);
            check_error(gl, "Failed to bind vertex buffer");
            gl.VertexAttribPointer(slot, count, type_, normalized, stride as i32, offset as * const() as * const _);
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
            gl.BufferData(gl::ARRAY_BUFFER, (data.len() * size_of::<T>()) as GLsizeiptr, data.as_ptr() as * const _, usage);
            gl.BindBuffer(gl::ARRAY_BUFFER, 0);
            if !check_error(gl, "Failed to create vertex buffer") {
                log::debug!("Created vertex buffer {}", id)
            }
        }
        GlVertexBuffer { gl: Some(gl.clone()), id, phantom: std::marker::PhantomData }
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
            gl.BufferData(gl::ELEMENT_ARRAY_BUFFER, (indices.len() * size_of::<u32>()) as GLsizeiptr, indices.as_ptr() as * const _, usage);
            gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            if !check_error(gl, "Failed to create index buffer") {
                log::debug!("Created index buffer {}", id)
            }
        }
        GlIndexBuffer { gl: Some(gl.clone()), id }
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
}

//////////////////////////////////////////////////
// Uniform Buffer

impl<T: Default> GlUniformBuffer<T> {

    pub fn new(gl: &Gl, usage: GLenum, data: &T) -> GlUniformBuffer<T>{
        let mut id: GLuint = 0;
        unsafe {
            gl.GenBuffers(1, &mut id);
            gl.BindBuffer(gl::UNIFORM_BUFFER, id);
            gl.BufferData(gl::UNIFORM_BUFFER, size_of::<T>() as GLsizeiptr, data as *const T as * const _, usage);
            gl.BindBuffer(gl::UNIFORM_BUFFER, 0);
            if !check_error(gl, "Failed to create index buffer") {
                log::debug!("Created uniform buffer {}", id)
            }
        }
        GlUniformBuffer { gl: Some(gl.clone()), id, phantom: std::marker::PhantomData, .. Default::default() }
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
}


//////////////////////////////////////////////////
// Texture

impl GlTexture {

    pub fn new<P, Container>(gl: &Gl, images: &[image::ImageBuffer<P, Container>]) -> GlTexture 
        where
        P: image::Pixel + 'static,
        P::Subpixel: 'static,
        Container: std::ops::Deref<Target = [P::Subpixel]>
    {
        // all textures need same size
        assert!(images.len() > 0);
        assert!(images.windows(2).all(|w| w[0].dimensions() == w[1].dimensions()));
        // get specs from first image
        let img = images.first().unwrap();
        let pixel_type = if size_of::<P::Subpixel>() == 1 { gl::UNSIGNED_BYTE } else { gl::UNSIGNED_SHORT };
        let (format, internal_format) = match <P as image::Pixel>::COLOR_TYPE {
            image::ColorType::Rgb8 => (gl::RGB, gl::RGB8),
            image::ColorType::Rgb16 => (gl::RGB, gl::RGBA16F),
            image::ColorType::Rgba8 => (gl::RGBA, gl::RGBA8),
            image::ColorType::Rgba16 => (gl::RGBA, gl::RGBA16F),
            _ => unimplemented!()
        };
        let num_mip_map = 1 + (img.width().min(img.height()) as f32).log2().floor() as i32;

        let mut id: GLuint = 0;
        unsafe {
            gl.GenTextures(1, &mut id);
            gl.BindTexture(gl::TEXTURE_2D_ARRAY, id);
            gl.TexStorage3D(gl::TEXTURE_2D_ARRAY, num_mip_map, internal_format, img.width() as GLsizei, img.height() as GLsizei, images.len()  as GLsizei);
            images.iter().enumerate().for_each(|(i, img)| {
                gl.TexSubImage3D(gl::TEXTURE_2D_ARRAY, 0, 0, 0, i as GLint, img.width() as GLsizei, img.height() as GLsizei, 1, format, pixel_type, img.as_ptr() as * const _);
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
        GlTexture { gl: Some(gl.clone()), id, .. Default::default() }
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
        unsafe {
            let vs = gl.CreateShader(gl::VERTEX_SHADER);
            if !check_error(gl, "Failed to create shaders") {
                log::debug!("Created vertex shader {}", vs);
            }
            let fs = gl.CreateShader(gl::FRAGMENT_SHADER);
            if !check_error(gl, "Failed to create shaders") {
                log::debug!("Created fragment shader {}", fs);
            }

            gl.ShaderSource(vs, 1, [vert.as_ptr() as * const _].as_ptr(), std::ptr::null());
            gl.CompileShader(vs);
            let mut status = 0;
            gl.GetShaderiv(vs, gl::COMPILE_STATUS, &mut status);
            if status == 0 {
                log::error!("Failed to compile vertex shader");
                print_shader_log(gl, vs);
            } else {
                log::debug!("Compiled vertex shader {}", vs);
            }

            gl.ShaderSource(fs, 1, [frag.as_ptr() as * const _].as_ptr(), std::ptr::null());
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

            GlShader { gl: Some(gl.clone()), vs, fs, program }
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
            let loc = gl.GetUniformBlockIndex(self.program, std::ffi::CString::new(location).unwrap().as_ptr());
            gl.UniformBlockBinding(self.program, loc, unit);
            check_error(gl, "Failed to bind uniform");
        }
    }

    pub fn link_texture(&mut self, unit: GLint, location: &str) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            let loc = gl.GetUniformLocation(self.program, std::ffi::CString::new(location).unwrap().as_ptr());
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
            gl.DrawElements(mode, index_count as GLsizei, gl::UNSIGNED_INT, 0 as * const() as * const _);
            check_error(gl, "Failed to draw");
        }
    }

    pub fn draw_elements_instanced(&mut self, mode: GLenum, index_count: usize, instance_count: usize) {
        let gl = self.gl.as_ref().expect("Missing OpenGL Context!");
        unsafe {
            gl.DrawElementsInstanced(mode, index_count as GLsizei, gl::UNSIGNED_INT, 0 as * const() as * const _, instance_count as GLsizei);
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
                gl.DeleteVertexArrays(1, [self.id].as_ptr() as * const _); 
                if !check_error(gl, "Failed to release vertex array object") {
                    log::debug!("Deleted vertex array object {}", self.id)
                }
            }
            self.gl = None;
        }
    } 
}
impl Drop for GlVertexArrayObject { fn drop(&mut self) { self.release() } }

impl<T: Default> GlResource for GlVertexBuffer<T> { 
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe { 
                gl.DeleteBuffers(1, &self.id); 
                if !check_error(gl, "Failed to release vertex buffer") {
                    log::debug!("Deleted vertex buffer {}", self.id)
                }
            }
            self.gl = None;
        }
    }
}
impl<T: Default> Drop for GlVertexBuffer<T> { fn drop(&mut self) { self.release() } }

impl GlResource for GlIndexBuffer { 
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe { 
                gl.DeleteBuffers(1, &self.id);
                if !check_error(gl, "Failed to release index buffer") {
                    log::debug!("Deleted index buffer {}", self.id);
                }
            }
            self.gl = None;
        }
    }
}
impl Drop for GlIndexBuffer { fn drop(&mut self) { self.release() } }

impl<T: Default> GlResource for GlUniformBuffer<T> { 
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe { 
                gl.DeleteBuffers(1, &self.id); 
                if !check_error(gl, "Failed to release uniform buffer") {
                    log::debug!("Deleted uniform buffer {}", self.id)
                }
            }
            self.gl = None;
        }
    }
}
impl<T: Default> Drop for GlUniformBuffer<T> { fn drop(&mut self) { self.release() } }

impl GlResource for GlTexture {
    fn release(&mut self) {
        if let Some(gl) = self.gl.as_ref() {
            unsafe { 
                gl.DeleteTextures(1, &self.id);
                if !check_error(gl, "Failed to release texture") {
                    log::debug!("Deleted texture {}", self.id);
                }
            }
            self.gl = None;
        }
    }
}
impl Drop for GlTexture { fn drop(&mut self) { self.release() } }

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
            self.gl = None;
        }
    }
}
impl Drop for GlShader { fn drop(&mut self) { self.release() } }


//////////////////////////////////////////////////
// Check error call

#[inline]
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


//////////////////////////////////////////////////
// Traits

impl std::fmt::Debug for gl::Gles2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gles2").finish()
    }
}

}