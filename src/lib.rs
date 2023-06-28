//////////////////////////////////////////////////
// Module

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
use std::rc::Rc;
use std::time::Instant;

use winit::event::{Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};

use raw_window_handle::HasRawWindowHandle;

use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::*;
use glutin::surface::{Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};

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

        // create window, gl_config & gl_context
        let (mut window, gl_config) = Self::create_window(&event_loop);
        let mut not_current_gl_context = Self::create_gl_context(ContextApi::Gles(None), &gl_config, &window);
        let mut device_ctx = None;

        // start event loop
        event_loop.run(move |event, window_target, control_flow| {
            //control_flow.set_poll();
            match event {
                Event::Resumed => {
                    #[cfg(android_platform)]
                    println!("Android window available");

                    // create surface
                    let (gl_surface, window) = Self::create_gl_surface(&mut window, &gl_config, window_target);

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
                    Self::print_context_info(&gl);

                    // Create device context (used as dump storage)
                    assert!(device_ctx.replace((gl, gl_context, gl_surface, window)).is_none());

                    // call create device callback
                    runner.create_device(&device_ctx.as_ref().unwrap().0);
                }
                Event::Suspended => {
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
                        assert!(Gl::strong_count(&gl) == 1, "Error! Unreleased OpenGL resources");
                    }
                }
                Event::LoopDestroyed => {
                    if let Some((gl, gl_context, ..)) = device_ctx.take() {
                        // call destroy device callback
                        runner.destroy_device(&gl);

                        // Destroy the GL Surface and un-current the GL Context before ndk-glue releases
                        // the window back to the system.
                        assert!(not_current_gl_context.replace(gl_context.make_not_current().unwrap()).is_none());
                        assert!(Gl::strong_count(&gl) == 1, "Error! Unreleased OpenGL resources");
                    }

                    // call cleanup callback
                    runner.cleanup();
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
    }

    fn create_window<T>(event_loop: &EventLoop<T>) -> (Option<Window>, Config) {
        let window_builder = Some(WindowBuilder::new().with_title("A fantastic window!"));
        let display_builder = DisplayBuilder::new().with_window_builder(window_builder);
        let template = ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(cfg!(cgl_backend));
        display_builder
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
            .unwrap()
    }

    fn create_gl_context(api: ContextApi, config: &Config, window: &Option<Window>) -> Option<NotCurrentContext> {
        let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());
        let context_attributes = ContextAttributesBuilder::new().with_context_api(api).build(raw_window_handle);
        Some(unsafe { config.display().create_context(config, &context_attributes).expect("failed to create context") })
    }

    fn create_gl_surface<T>(window: &mut Option<Window>, config: &Config, window_target: &EventLoopWindowTarget<T>) -> (Surface<WindowSurface>, Window) {
        let window = window.take().unwrap_or_else(|| {
            let window_builder = WindowBuilder::new().with_transparent(true);
            glutin_winit::finalize_window(window_target, window_builder, &config).unwrap()
        });
        let attrs = window.build_surface_attributes(<_>::default());
        let gl_surface = unsafe { config.display().create_window_surface(config, &attrs).unwrap() };
        (gl_surface, window)
    }

    fn print_context_info(gl: &Gl) {
        log::info!("Created OpenGL context:");
        log::info!("- Version: {:?}", Self::get_gl_string(gl, gl::VERSION));
        // TODO: print more
    }

    fn get_gl_string(gl: &Gl, gl_enum: GLenum) -> String {
        unsafe {
            let data = CStr::from_ptr(gl.GetString(gl_enum) as *const _).to_bytes().to_vec();
            String::from_utf8(data).unwrap()
        }
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
