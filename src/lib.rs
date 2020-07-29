//////////////////////////////////////////////////
// Modules

//pub(crate) mod gl {
pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

pub mod utils;


//////////////////////////////////////////////////
// Using

use std::ffi::CStr;
use std::time::Instant;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};

#[cfg(target_os = "android")]
use glutin::platform::android::AndroidEventLoop as EventLoop;
#[cfg(not(target_os = "android"))]
use glutin::event_loop::EventLoop;

use gl::types::*;

use crate::input::{ InputEvent, CursorEvent, MouseEvent };

pub use utils::resources::Gl;


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
        println!("OpenGL device:\n-> Version: {:?}\n-> {:?}", self.get_string(gl::VERSION), self.window_context.get_pixel_format());
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

pub struct Platform {}

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
// Testing

use std::mem::size_of;
use utils::resources::*;

#[derive(Debug, Default)]
pub struct ExampleRunner {
    vao: utils::resources::GlVertexArrayObject,
    vbo: utils::resources::GlVertexBuffer<[f32; 4]>,
    ibo: utils::resources::GlIndexBuffer,
    ubo: utils::resources::GlUniformBuffer<(f32, f32, f32, f32)>,
    texture: utils::resources::GlTexture,
    shader: utils::resources::GlShader,
    resolution: (GLsizei, GLsizei),
}

impl Runner for ExampleRunner {

    fn init(&mut self) { }
    fn cleanup(&mut self) { }
    fn pause(&mut self) { }
    fn resume(&mut self) { }
    fn update(&mut self, _elapsed_time: f32) { }
    fn input(&mut self, _input_events: &[InputEvent]) { }

    fn render(&mut self, gl: &Gl) {
        unsafe { 
            gl.ClearColor(1.0, 0.0, 0.0, 1.0); 
            gl.ClearDepthf(1.0);
            gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            self.vao.bind();
            self.ibo.bind();

            self.texture.bind(1);
            self.ubo.bind(1);

            self.shader.bind();
            self.shader.link_texture(1, "t_Sampler");
            self.shader.link_uniform(1, "Settings");

            gl.Viewport(0, 0, self.resolution.0, self.resolution.1);
            // gl.Disable(gl::CULL_FACE);
            // gl.Disable(gl::DEPTH_TEST);
            // gl.Enable(gl::DEPTH_TEST);
            // gl.DepthMask(gl::TRUE);
            // gl.DepthFunc(gl::LESS);

            self.shader.draw_elements(gl::TRIANGLE_STRIP, 4);

            self.shader.unbind();

            self.ubo.unbind();
            self.texture.unbind();

            self.ibo.unbind();
            self.vao.unbind();
        }
    }

    fn create_device(&mut self, gl: &Gl) {
        println!("create_device");

        // create resources
        self.vao = GlVertexArrayObject::new(gl);

        self.vbo = GlVertexBuffer::new(gl, gl::STATIC_DRAW, &[
            [-0.5, -0.5, 0.0, 0.0], 
            [-0.5,  0.5, 0.0, 1.0], 
            [ 0.5, -0.5, 1.0, 0.0],
            [ 0.5,  0.5, 1.0, 1.0],
        ]);

        self.ibo = GlIndexBuffer::new(gl, gl::STATIC_DRAW, &[
            0, 1, 2, 3
        ]);

        self.ubo = GlUniformBuffer::new(gl, gl::DYNAMIC_DRAW, &(0.5, 0.5, 0.5, 1.0));

        let image: image::RgbaImage = image::RgbaImage::from_vec(1, 1, vec![0, 255, 0, 255]).unwrap();
        self.texture = GlTexture::new(gl, &[image]);

        self.shader = GlShader::new(gl, VS, FS);

        // bind buffers to vao
        self.vao.bind();
        self.vao.bind_attrib(&self.vbo, 0, 2, gl::FLOAT, gl::FALSE, 0 * size_of::<f32>(), 4 * size_of::<f32>(), 0);
        self.vao.bind_attrib(&self.vbo, 1, 2, gl::FLOAT, gl::FALSE, 2 * size_of::<f32>(), 4 * size_of::<f32>(), 0);
        self.vao.unbind();
    }

    fn destroy_device(&mut self, _gl: &Gl) {
        println!("destroy_device");

        self.vao.release();
        self.vbo.release();
        self.ibo.release();
        self.ubo.release();
        self.texture.release();
        self.shader.release();
    }

    fn resize_device(&mut self, _gl: &Gl, width: u32, height: u32) {
        println!("resize_device ({} x {})", width, height);
        self.resolution = (width as GLsizei, height as GLsizei);
    }
}

const VS: &'static [u8] = b"
#version 300 es
layout(location = 0) in vec2 a_Pos;
layout(location = 1) in vec2 a_TexCoord;

//in float a_TexSlot;

out vec3 v_TexCoord;

void main() {
    v_TexCoord = vec3(a_TexCoord, 0.0);
    gl_Position = vec4(a_Pos, 0.0, 1.0);
}
\0";

const FS: &'static [u8] = b"
#version 300 es
precision mediump float;

in vec3 v_TexCoord;

uniform sampler2DArray t_Sampler;

layout(std140) uniform Settings {
    vec4 u_Color;
};

layout(location = 0) out vec4 target0;

void main() {
    target0 = texture(t_Sampler, v_TexCoord) * u_Color;
}
\0";

pub fn main () {
    let mut game_loop = GameLoop::new(ExampleRunner{ .. Default::default() });
    game_loop.run();
}