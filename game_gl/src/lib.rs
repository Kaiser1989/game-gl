//////////////////////////////////////////////////
// Module

pub mod app;
pub mod file;
pub mod input;
pub mod opengl;
pub mod runner;

//////////////////////////////////////////////////
// OpenGL binding

pub mod gl {
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

//////////////////////////////////////////////////
// Prelude

pub mod prelude {
    pub use crate::gl;
    pub use crate::gl::types::*;
    pub use crate::{input::InputEvent, GameLoop, Gl, Runner};
    pub use image;
    #[cfg(target_os = "android")]
    pub use winit::platform::android::activity::AndroidApp;
}

//////////////////////////////////////////////////
// Using

use std::rc::Rc;
use std::time::Instant;

use glutin::config::ConfigTemplateBuilder;
use glutin_winit::DisplayBuilder;
use raw_window_handle::{HasDisplayHandle, HasRawDisplayHandle};

use runner::Runner;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};

#[cfg(target_os = "android")]
use once_cell::sync::OnceCell;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::window::Window;

use crate::app::App;
use crate::input::{CursorEvent, InputEvent, MouseEvent};

//////////////////////////////////////////////////
// Types

pub type Gl = Rc<gl::Gles2>;

//////////////////////////////////////////////////
// Types

#[cfg(target_os = "android")]
pub static ANDROID_APP: OnceCell<AndroidApp> = OnceCell::new();

//////////////////////////////////////////////////
// Game loop

pub struct GameLoop {}

#[cfg(target_os = "android")]
impl GameLoop {
    pub fn start<R: Runner + 'static>(app: AndroidApp, runner: R) {
        ANDROID_APP.set(app.clone()).unwrap();
        let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
        GameLoop::run(event_loop, runner);
    }

    pub fn stop() {
        // TODO: android exit
        std::process::exit(0);
    }
}

#[cfg(not(target_os = "android"))]
impl GameLoop {
    pub fn start<R: Runner + 'static>(runner: R) {
        let event_loop = EventLoop::builder().build().unwrap();
        GameLoop::run(event_loop, runner);
    }

    pub fn stop() {
        std::process::exit(0);
    }
}

impl GameLoop {
    pub fn run<R: Runner + 'static>(event_loop: EventLoop<()>, mut runner: R) {
        log::trace!("Initializing application...");

        // init application
        let template = glutin::config::ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(cfg!(cgl_backend));
        let window = winit::window::Window::default_attributes()
            .with_transparent(true)
            .with_title("Glutin triangle gradient example (press Escape to exit)");
        let display_builder = glutin_winit::DisplayBuilder::new().with_window_attributes(Some(window));

        let mut app = App::new(template, display_builder);

        // call init callback
        runner.init();

        // init input
        let mut input_events: Vec<InputEvent> = Vec::with_capacity(10);

        // start game time
        let mut time = Instant::now();

        log::trace!("Running mainloop...");
        event_loop.run_app(&mut app);
        event_loop.run(move |event, event_loop, control_flow| {
            log::trace!("Received Winit event: {event:?}");

            *control_flow = ControlFlow::Wait;
            match event {
                Event::Resumed => {
                    app.resume(event_loop);
                    runner.create_device(app.renderer());
                }
                Event::Suspended => {
                    runner.destroy_device(app.renderer());
                    app.suspend();
                }
                Event::RedrawRequested(_) => {
                    log::trace!("Handling Redraw Request");
                    if app.has_surface_and_context() {
                        if app.has_renderer() {
                            // call init callback
                            runner.render(app.renderer());

                            // swap buffers
                            app.swap_buffers();
                        }
                        app.queue_redraw();
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
                Event::LoopDestroyed => {
                    // non android device does not get a suspend event
                    #[cfg(not(target_os = "android"))]
                    {
                        runner.destroy_device(app.renderer());
                        app.suspend();
                    }
                    runner.cleanup();
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(size) => {
                        if size.width != 0 && size.height != 0 {
                            runner.resize_device(app.renderer(), size.width, size.height);
                        }
                        app.queue_redraw();
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
                    WindowEvent::CloseRequested => {
                        control_flow.set_exit();
                    }
                    _ => (),
                },
                _ => {}
            }
        });
    }
}

pub struct AppHandler {
    app: App,
}

impl AppHandler {
    pub fn new(app: App) -> Self {
        AppHandler { app }
    }
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.resume(event_loop);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
        self.suspend();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                self.resize(size);

                // TODO: RESIZE
            }
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event: KeyEvent {
                    logical_key: Key::Named(NamedKey::Escape),
                    ..
                },
                ..
            } => event_loop.exit(),
            _ => (),
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;

        // CLEAN UP

        self.exit();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;

        // DRAW

        self.swap_buffers();
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

// #[cfg(target_os = "android")]
// fn enable_immersive() {
//     let vm_ptr = ndk_glue::native_activity().vm();
//     let vm = unsafe { jni::JavaVM::from_raw(vm_ptr) }.unwrap();
//     let env = vm.attach_current_thread_permanently().unwrap();
//     let activity = ndk_glue::native_activity().activity();
//     let window = env.call_method(activity, "getWindow", "()Landroid/view/Window;", &[]).unwrap().l().unwrap();
//     let view = env.call_method(window, "getDecorView", "()Landroid/view/View;", &[]).unwrap().l().unwrap();
//     let view_class = env.find_class("android/view/View").unwrap();
//     let flag_fullscreen = env.get_static_field(view_class, "SYSTEM_UI_FLAG_FULLSCREEN", "I").unwrap().i().unwrap();
//     let flag_hide_navigation = env.get_static_field(view_class, "SYSTEM_UI_FLAG_HIDE_NAVIGATION", "I").unwrap().i().unwrap();
//     let flag_immersive_sticky = env.get_static_field(view_class, "SYSTEM_UI_FLAG_IMMERSIVE_STICKY", "I").unwrap().i().unwrap();
//     let flag = flag_fullscreen | flag_hide_navigation | flag_immersive_sticky;
//     match env.call_method(view, "setSystemUiVisibility", "(I)V", &[jni::objects::JValue::Int(flag)]) {
//         Err(_) => log::warn!("Failed to enable immersive mode"),
//         Ok(_) => {}
//     }
//     env.exception_clear().unwrap();
// }
