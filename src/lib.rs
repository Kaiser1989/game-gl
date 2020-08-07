//////////////////////////////////////////////////
// Module

pub mod input;
pub mod opengl;
pub mod file;
mod config;


//////////////////////////////////////////////////
// Prelude

pub mod prelude {
    pub use crate::{GameLoop, Runner, Gl, input::InputEvent};
    pub use crate::gl;
    pub use crate::gl::types::*;
    pub use image;
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

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};

#[cfg(target_os = "android")]
use glutin::platform::android::AndroidEventLoop as EventLoop;
#[cfg(not(target_os = "android"))]
use glutin::event_loop::EventLoop;

use crate::gl::types::*;
use crate::config::Config;
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
                            // enable immersive mode
                            enable_immersive();

                            // create graphics context
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
                        if let (true, Some(device_ctx)) = (self.has_render_context(), self.device_ctx.as_mut()) {

                            // call render callback
                            self.runner.render(&device_ctx.gl);

                            // swap buffers
                            match device_ctx.window_context.swap_buffers() {
                                Err(_) => {
                                    log::warn!("Corrupted render context, try recovering ...");
                                    self.runner.destroy_device(&device_ctx.gl);
                                    *device_ctx = DeviceContext::new(_event_loop);
                                    self.runner.create_device(&device_ctx.gl);
                                    log::warn!("... recovering successful!");
                                },
                                Ok(_) => {}
                            }
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

    // ANDROID: check if we have render context
    #[cfg(target_os = "android")]
    fn has_render_context(&self) -> bool {
        self.device_ctx.is_some() && ndk_glue::native_window().is_some()
    }

    // WINDOWS: check if we have render context
    #[cfg(not(target_os = "android"))]
    fn has_render_context(&self) -> bool {
        self.device_ctx.is_some()
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
            .with_gl(GlRequest::Specific(Config::opengl_api_support(), Config::opengl_version_support()))
            //.with_gl_debug_flag(true)
            .with_srgb(Config::srgb_support())
            .with_vsync(Config::vsync_support())
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
// Traits

impl std::fmt::Debug for gl::Gles2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gles2").finish()
    }
}


//////////////////////////////////////////////////
// Enable Immersive mode

#[cfg(target_os = "android")]
fn enable_immersive() {
    let vm_ptr = ndk_glue::native_activity().vm();
    let vm = unsafe { jni::JavaVM::from_raw(vm_ptr) }.unwrap();
    let env = vm.attach_current_thread_permanently().unwrap();
    let activity = ndk_glue::native_activity().activity();
    let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[]).unwrap().l().unwrap();
    let view = env.call_method(window, "getDecorView", "()Landroid/view/View;", &[]).unwrap().l().unwrap();
    let view_class = env.find_class("android/view/View").unwrap();
    let flag_fullscreen = env.get_static_field(view_class, "SYSTEM_UI_FLAG_FULLSCREEN", "I").unwrap().i().unwrap();
    let flag_hide_navigation = env.get_static_field(view_class, "SYSTEM_UI_FLAG_HIDE_NAVIGATION", "I").unwrap().i().unwrap();
    let flag_immersive_sticky = env.get_static_field(view_class, "SYSTEM_UI_FLAG_IMMERSIVE_STICKY", "I").unwrap().i().unwrap();
    let flag = flag_fullscreen | flag_hide_navigation | flag_immersive_sticky;
    match env.call_method(view, "setSystemUiVisibility", "(I)V", &[jni::objects::JValue::Int(flag)]) {
        Err(_) => log::warn!("Failed to enable immersive mode"),
        Ok(_) => {}
    }
    env.exception_clear().unwrap();
}