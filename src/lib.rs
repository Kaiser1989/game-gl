//////////////////////////////////////////////////
// Module

mod config;
pub mod file;
pub mod input;
pub mod opengl;

//////////////////////////////////////////////////
// Prelude

pub mod prelude {
    pub use crate::gl;
    pub use crate::gl::types::*;
    pub use crate::{input::InputEvent, GameLoop, Gl, Runner};
    pub use image;
}

//////////////////////////////////////////////////
// OpenGL binding

pub mod gl {
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

//////////////////////////////////////////////////
// Using

use std::ffi::{CStr, CString};
use std::num::NonZeroU32;
use std::ops::Deref;
use std::rc::Rc;
use std::time::Instant;

use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use raw_window_handle::HasRawWindowHandle;

use glutin::config::{ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::*;
use glutin::surface::{Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};

use crate::config::Config;
use crate::gl::types::*;
use crate::input::{CursorEvent, InputEvent, MouseEvent};

//////////////////////////////////////////////////
// Types

pub type Gl = Rc<gl::Gles2>;

//////////////////////////////////////////////////
// Traits

pub trait Runner {
    fn init(&mut self);

    fn cleanup(&mut self);

    fn input(&mut self, input_events: &[InputEvent]);

    fn update(&mut self, elapsed_time: f32);

    fn render(&mut self, gl: &Gl);

    fn create_device(&mut self, gl: &Gl);

    fn destroy_device(&mut self, gl: &Gl);

    fn resize_device(&mut self, gl: &Gl, width: u32, height: u32);
}

//////////////////////////////////////////////////
// Game loop

pub struct GameLoop {}

impl GameLoop {
    pub fn run<T: Runner + 'static>(mut runner: T) {
        // call init callback
        runner.init();

        // start game time
        let mut time = Instant::now();

        // init input
        let mut input_events: Vec<InputEvent> = Vec::with_capacity(10);

        // init game loop
        let event_loop = EventLoop::new();

        // // WINDOWS: create context
        // #[cfg(not(target_os = "android"))]
        // {
        //     self.device_ctx = Some(DeviceContext::new(&event_loop));
        //     if let Some(device_ctx) = self.device_ctx.as_mut() {
        //         // call create device callback
        //         self.runner.create_device(&device_ctx.gl);

        //         // call resize device callback
        //         let resolution = device_ctx.window_context.window().inner_size();
        //         self.runner.resize_device(&device_ctx.gl, resolution.width, resolution.height)
        //     }
        // }

        // // WINDOWS: App is starting paused
        // #[cfg(target_os = "android")]
        // let mut paused = true;

        // // WINDOWS: App is starting unpaused
        // #[cfg(not(target_os = "android"))]
        // let mut paused = false;

        // create display builder
        let window_builder = Some(WindowBuilder::new().with_title("A fantastic window!"));
        let display_builder = DisplayBuilder::new().with_window_builder(window_builder);

        // get display configs
        let template = ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(cfg!(cgl_backend));
        let (mut window, gl_config) = display_builder
            .build(&event_loop, template, |configs| {
                // Find the config with the maximum number of samples, so our triangle will
                // be smooth.
                configs
                    .reduce(|accum, config| {
                        let transparency_check = config.supports_transparency().unwrap_or(false) & !accum.supports_transparency().unwrap_or(false);
                        if transparency_check || config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        // create context
        let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());
        let context_attributes = ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(raw_window_handle);
        let mut not_current_gl_context = Some(unsafe { gl_config.display().create_context(&gl_config, &context_attributes).expect("failed to create context") });

        // start event loop
        let mut device_ctx = None;
        event_loop.run(move |event, window_target, control_flow| {
            //control_flow.set_poll();
            match event {
                Event::Resumed => {
                    #[cfg(android_platform)]
                    println!("Android window available");

                    // init surface for given window
                    let window = window.take().unwrap_or_else(|| {
                        let window_builder = WindowBuilder::new().with_transparent(true);
                        glutin_winit::finalize_window(window_target, window_builder, &gl_config).unwrap()
                    });
                    let attrs = window.build_surface_attributes(<_>::default());
                    let gl_surface = unsafe { gl_config.display().create_window_surface(&gl_config, &attrs).unwrap() };

                    // Make it current.
                    // The context needs to be current for the Renderer to set up shaders and
                    // buffers. It also performs function loading, which needs a current context on
                    // WGL.
                    let gl_context = not_current_gl_context.take().unwrap().make_current(&gl_surface).unwrap();

                    // Try setting vsync.
                    if let Err(res) = gl_surface.set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())) {
                        eprintln!("Error setting vsync: {res:?}");
                    }

                    // create openGL instance
                    let gl = Gl::new(gl::Gles2::load_with(|ptr| {
                        let ptr = CString::new(ptr).unwrap();
                        gl_context.display().get_proc_address(ptr.as_c_str()).cast()
                    }));

                    // Create device context
                    assert!(device_ctx.replace((gl, gl_context, gl_surface, window)).is_none());

                    // call create device callback
                    runner.create_device(&device_ctx.as_ref().unwrap().0);
                }
                Event::Suspended | Event::LoopDestroyed => {
                    // This event is only raised on Android, where the backing NativeWindow for a GL
                    // Surface can appear and disappear at any moment.
                    #[cfg(android_platform)]
                    println!("Android window removed");

                    if let Some((gl, gl_context, ..)) = device_ctx.take() {
                        // call destroy device callback
                        runner.destroy_device(&gl);

                        // Destroy the GL Surface and un-current the GL Context before ndk-glue releases
                        // the window back to the system.
                        assert!(not_current_gl_context.replace(gl_context.make_not_current().unwrap()).is_none());

                        // only this reference is allowed
                        assert!(Gl::strong_count(&gl) == 1, "Error! Unreleased OpenGL resources");
                    }
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(size) => {
                        if size.width != 0 && size.height != 0 {
                            // Some platforms like EGL require resizing GL surface to update the size
                            // Notable platforms here are Wayland and macOS, other don't require it
                            // and the function is no-op, but it's wise to resize it for portability
                            // reasons.
                            if let Some((gl, gl_context, gl_surface, ..)) = device_ctx.as_mut() {
                                gl_surface.resize(&gl_context, NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap());

                                // call resize device callback
                                runner.resize_device(&gl, size.width, size.height);
                            }
                        }
                    }
                    WindowEvent::CloseRequested => {
                        control_flow.set_exit();
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        input_events.push(InputEvent::Cursor(CursorEvent { location: position.into() }));
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        input_events.push(InputEvent::Mouse(MouseEvent {
                            state: state.into(),
                            button: button.into(),
                        }));
                    }
                    WindowEvent::Touch(touch) => {
                        input_events.push(InputEvent::Touch(touch.into()));
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        input_events.push(InputEvent::Keyboard(input.into()));
                    }
                    _ => (),
                },
                Event::RedrawEventsCleared => {
                    if let Some((gl, gl_context, gl_surface, window)) = device_ctx.as_mut() {
                        // call render callback
                        runner.render(&gl);

                        // swap buffer
                        window.request_redraw();
                        gl_surface.swap_buffers(&gl_context).unwrap();
                    }
                }
                Event::MainEventsCleared => {
                    // update time
                    let new_time = Instant::now();
                    let elapsed_time = new_time.duration_since(time).as_millis() as f32 / 1000.0;
                    time = new_time;

                    // call input callback
                    runner.input(&input_events);
                    input_events.clear();

                    // call update callback
                    runner.update(elapsed_time);
                }
                _ => (),
            }
        });

        //     // starting game loop
        //     let mut running = true;
        //     while running {
        //         // check glutin events
        //         event_loop.run_return(|event, _event_loop, control_flow| {
        //             *control_flow = ControlFlow::Exit;
        //             match event {
        //                 Event::Resumed => {
        //                     // ANDROID: only create if native window is available
        //                     #[cfg(target_os = "android")]
        //                     {
        //                         // enable immersive mode
        //                         enable_immersive();

        //                         // create graphics context
        //                         if self.device_ctx.is_none() && ndk_glue::native_window().is_some() {
        //                             self.device_ctx = Some(DeviceContext::new(_event_loop));
        //                             if let Some(device_ctx) = self.device_ctx.as_mut() {
        //                                 // call create device callback
        //                                 self.runner.create_device(&device_ctx.gl);
        //                             }
        //                         }
        //                     }

        //                     // call resume callback
        //                     paused = false;
        //                 }
        //                 Event::Suspended => {
        //                     // call pause callback
        //                     paused = true;

        //                     // ANDROID: only destroy if native window is available
        //                     #[cfg(target_os = "android")]
        //                     {
        //                         if self.device_ctx.is_some() && ndk_glue::native_window().is_some() {
        //                             if let Some(device_ctx) = self.device_ctx.as_mut() {
        //                                 // call destroy device callback
        //                                 self.runner.destroy_device(&device_ctx.gl);
        //                             }
        //                             self.device_ctx = None;
        //                         }
        //                     }
        //                 }
        //                 Event::WindowEvent { event, .. } => match event {
        //                     WindowEvent::Resized(physical_size) => {
        //                         if let Some(device_ctx) = self.device_ctx.as_mut() {
        //                             device_ctx.window_context.resize(physical_size);

        //                             // call resize device callback
        //                             self.runner.resize_device(&device_ctx.gl, physical_size.width, physical_size.height);
        //                         }
        //                     }
        //                     WindowEvent::CloseRequested => {
        //                         running = false;
        //                     }
        //                     WindowEvent::CursorMoved { position, .. } => {
        //                         input_events.push(InputEvent::Cursor(CursorEvent { location: position.into() }));
        //                     }
        //                     WindowEvent::MouseInput { state, button, .. } => {
        //                         input_events.push(InputEvent::Mouse(MouseEvent {
        //                             state: state.into(),
        //                             button: button.into(),
        //                         }));
        //                     }
        //                     WindowEvent::Touch(touch) => {
        //                         input_events.push(InputEvent::Touch(touch.into()));
        //                     }
        //                     WindowEvent::KeyboardInput { input, .. } => {
        //                         input_events.push(InputEvent::Keyboard(input.into()));
        //                     }
        //                     _ => (),
        //                 },
        //                 Event::MainEventsCleared => {
        //                     // update time
        //                     let new_time = Instant::now();
        //                     let elapsed_time = new_time.duration_since(time).as_millis() as f32 / 1000.0;
        //                     time = new_time;

        //                     // process input
        //                     self.runner.input(&input_events);
        //                     input_events.clear();

        //                     // if app is paused do not call update and render
        //                     if !paused {
        //                         // call update callback
        //                         self.runner.update(elapsed_time);

        //                         // render call
        //                         if let (true, Some(device_ctx)) = (self.has_render_context(), self.device_ctx.as_mut()) {
        //                             // call render callback
        //                             self.runner.render(&device_ctx.gl);

        //                             // swap buffers
        //                             if device_ctx.window_context.swap_buffers().is_err() {
        //                                 log::warn!("Corrupted render context, try recovering ...");
        //                                 self.runner.destroy_device(&device_ctx.gl);
        //                                 *device_ctx = DeviceContext::new(_event_loop);
        //                                 self.runner.create_device(&device_ctx.gl);
        //                                 log::warn!("... recovering successful!");
        //                             }
        //                         }
        //                     }
        //                 }
        //                 _ => (),
        //             }
        //         });
        //     }

        //     // WINDOWS: destroy context
        //     #[cfg(not(target_os = "android"))]
        //     {
        //         if let Some(device_ctx) = self.device_ctx.as_mut() {
        //             // call destroy device callback
        //             self.runner.destroy_device(&device_ctx.gl);
        //         }
        //         self.device_ctx = None;
        //     }

        //     // call cleanup callback
        //     self.runner.cleanup();
        // }

        // // ANDROID: check if we have render context
        // #[cfg(target_os = "android")]
        // fn has_render_context(&self) -> bool {
        //     self.device_ctx.is_some() && ndk_glue::native_window().is_some()
        // }

        // // WINDOWS: check if we have render context
        // #[cfg(not(target_os = "android"))]
        // fn has_render_context(&self) -> bool {
        //     self.device_ctx.is_some()
        // }
    }
}

//////////////////////////////////////////////////
// Internal Device

struct DeviceContext {
    gl: Gl,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    window: Window,
}

impl DeviceContext {
    fn new(gl_context: PossiblyCurrentContext, gl_surface: Surface<WindowSurface>, window: Window) -> DeviceContext {
        let gl = Gl::new(gl::Gles2::load_with(|ptr| {
            let ptr = CString::new(ptr).unwrap();
            gl_context.display().get_proc_address(ptr.as_c_str()).cast()
        }));
        let device_ctx = DeviceContext { gl, gl_context, gl_surface, window };
        device_ctx.print_context();
        device_ctx
    }

    fn print_context(&self) {
        log::info!("Created OpenGL context:");
        log::info!("- Version: {:?}", self.get_string(gl::VERSION));
        // TODO: print more
    }

    // +++ Starting OpenGL functions

    fn get_string(&self, gl_enum: GLenum) -> String {
        unsafe {
            let data = CStr::from_ptr(self.gl.GetString(gl_enum) as *const _).to_bytes().to_vec();
            String::from_utf8(data).unwrap()
        }
    }
}

impl Deref for DeviceContext {
    type Target = Gl;

    fn deref(&self) -> &Self::Target {
        &self.gl
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
