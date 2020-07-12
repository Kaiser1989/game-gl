//////////////////////////////////////////////////
// Modules

pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}


//////////////////////////////////////////////////
// Using

use std::ffi::CStr;
use std::time::Instant;

use glutin::event::{Event, WindowEvent, ElementState, MouseButton, Touch};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};

#[cfg(target_os = "android")]
use glutin::platform::android::AndroidEventLoop as EventLoop;
#[cfg(not(target_os = "android"))]
use glutin::event_loop::EventLoop;

use gl::types::GLenum;


//////////////////////////////////////////////////
// Traits

pub trait Runner {

    fn init(&mut self);

    fn cleanup(&mut self);

    fn pause(&mut self);

    fn resume(&mut self);

    fn input(&mut self);

    fn update(&mut self, elapsed_time: f32);

    fn render(&mut self, device_ctx: &mut DeviceContext);

    fn create_device(&mut self, device_ctx: &mut DeviceContext);

    fn resize_device(&mut self, device_ctx: &mut DeviceContext);

    fn destroy_device(&mut self, device_ctx: &mut DeviceContext);
}


//////////////////////////////////////////////////
// Definition

pub struct GameLoop<T: Runner> {
    runner: T,
    device_ctx: Option<DeviceContext>,
}

pub struct DeviceContext {
    gl: gl::Gles2,
    window_context: WindowedContext<PossiblyCurrent>,
}


//////////////////////////////////////////////////
// Implementation

impl<T: Runner> GameLoop<T> {

    pub fn new(runner: T) -> GameLoop<T> {
        Self { runner, device_ctx: None }
    }

    pub fn run(&mut self) {
        // call init callback
        self.runner.init();

        // start game time
        let mut time = Instant::now();

        // starting game loop
        let mut event_loop = EventLoop::new();
        let mut running = true;
        while running {

            // check glutin events
            event_loop.run_return(|event, event_loop, control_flow| {
                *control_flow = ControlFlow::Exit;
        
                match event {
                    Event::Resumed => {
                        // only create if native window is available
                        if self.device_ctx.is_none() && ndk_glue::native_window().is_some() {
                            self.device_ctx = Some(DeviceContext::new(event_loop));
                            if let Some(device_ctx) = self.device_ctx.as_mut() {
                                // call create device callback
                                self.runner.create_device(device_ctx);
                            }
                        }
                    },
                    Event::Suspended => {
                        // only destroy if native window is available
                        if self.device_ctx.is_some() && ndk_glue::native_window().is_some() {
                            if let Some(device_ctx) = self.device_ctx.as_mut() {
                                // call destroy device callback
                                self.runner.destroy_device(device_ctx);
                            }
                            self.device_ctx = None;
                        }
                    },
                    Event::RedrawRequested(_) => {
                        if let Some(device_ctx) = self.device_ctx.as_mut() {
                            device_ctx.clear_color(1.0, 0.0, 0.0, 1.0);
                            device_ctx.clear_depth(1.0);

                            // call render callback
                            self.runner.render(device_ctx);

                            device_ctx.window_context.swap_buffers().unwrap();
                        }
                    },
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(physical_size) => {
                            if let Some(device_ctx) = self.device_ctx.as_mut() {
                                device_ctx.window_context.resize(physical_size);

                                // call resize device callback
                                self.runner.resize_device(device_ctx);
                            }
                        },
                        WindowEvent::CloseRequested => {
                            running = false;
                        },
                        WindowEvent::MouseInput { state, button, ..} => {
                            match state {
                                ElementState::Pressed => { print!("pressed "); } 
                                ElementState::Released => { print!("released "); } 
                            }
                            match button {
                                MouseButton::Left => { println!("left"); }
                                MouseButton::Right => { println!("right"); }
                                _ => {}
                            }
                        },
                        WindowEvent::Touch(touch) => {
                            println!("{:?}", touch);
                        },
                        _ => (),
                    },
                    Event::MainEventsCleared => {
                        // update time
                        let new_time = Instant::now();
                        let elapsed_time = new_time.duration_since(time).as_millis() as f32 / 1000.0;
                        time = new_time;

                        // call update callback
                        self.runner.update(elapsed_time);
                    },
                    _ => (),
                }
            });
        }

        // call cleanup callback
        self.runner.cleanup();
    }
}

impl DeviceContext {

    fn new(el: &EventLoopWindowTarget<()>) -> DeviceContext {
        let wb = WindowBuilder::new().with_title("A fantastic window!");
        let window_context = ContextBuilder::new()
            .with_gl(GlRequest::Specific(Api::OpenGlEs, (2, 0)))
            .with_gl_debug_flag(false)
            .with_srgb(false)
            .with_vsync(true)
            .build_windowed(wb, &el)
            .unwrap();
        let window_context = unsafe { window_context.make_current().unwrap() };
        let gl = gl::Gles2::load_with(|ptr| window_context.get_proc_address(ptr) as *const _);
        let device_ctx = DeviceContext{ gl, window_context };
        device_ctx.print_context();
        device_ctx
    }

    fn print_context(&self) {
        println!("OpenGL device:\n-> {:?}\n-> {:?}", self.window_context.get_pixel_format(), self.get_string(gl::VERSION));
    }

    // +++ Starting OpenGL functions

    pub fn clear_color(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        unsafe { 
            self.gl.ClearColor(red, green, blue, alpha); 
            self.gl.Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    pub fn clear_depth(&self, depth: f32) {
        unsafe {
            self.gl.ClearDepthf(depth);
            self.gl.Clear(gl::DEPTH_BUFFER_BIT);
        }
    }

    pub fn get_string(&self, gl_enum: GLenum) -> String {
        unsafe {
            let data = CStr::from_ptr(self.gl.GetString(gl_enum) as *const _).to_bytes().to_vec();
            String::from_utf8(data).unwrap()
        }
    }
}