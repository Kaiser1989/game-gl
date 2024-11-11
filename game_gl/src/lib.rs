//////////////////////////////////////////////////
// Module

pub mod app;
pub mod file;
pub mod input;
pub mod opengl;

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
    pub use crate::{input::InputEvent, Game, GameContext, GameLoop, Gl};
    pub use image;
    #[cfg(target_os = "android")]
    pub use winit::platform::android::activity::AndroidApp;
}

//////////////////////////////////////////////////
// Using

use std::convert::TryInto;
use std::rc::Rc;
use std::time::Instant;

use file::Files;
use input::{CursorEvent, MouseEvent};
use log::LevelFilter;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;

use crate::app::App;
use crate::input::InputEvent;

//////////////////////////////////////////////////
// Types

pub type Gl = Rc<gl::Gles2>;

//////////////////////////////////////////////////
// Definition

pub struct Game<L: GameLoop> {
    app: Option<App>,
    game_loop: L,
    game_time: Instant,
    game_context: GameContext,
    input_events: Vec<InputEvent>,
}

pub struct GameContext {
    #[cfg(target_os = "android")]
    android_app: AndroidApp,
    request_quit: bool,
}

pub trait GameLoop: Default {
    fn title(&self) -> &str;

    fn init(&mut self, ctx: &mut GameContext);

    fn cleanup(&mut self, ctx: &mut GameContext);

    fn input(&mut self, ctx: &mut GameContext, input_events: &[InputEvent]);

    fn update(&mut self, ctx: &mut GameContext, elapsed_time: f32);

    fn render(&mut self, ctx: &mut GameContext, gl: &Gl);

    fn create_device(&mut self, ctx: &mut GameContext, gl: &Gl);

    fn destroy_device(&mut self, ctx: &mut GameContext, gl: &Gl);

    fn resize_device(&mut self, ctx: &mut GameContext, gl: &Gl, width: u32, height: u32);
}

//////////////////////////////////////////////////
// Implementation

#[cfg(target_os = "android")]
impl GameContext {
    pub fn new(android_app: AndroidApp) -> Self {
        GameContext { android_app, request_quit: false }
    }

    pub fn files(&self) -> Files {
        Files::new(&self.android_app)
    }
}

#[cfg(not(target_os = "android"))]
impl GameContext {
    pub fn new() -> Self {
        GameContext { request_quit: false }
    }

    pub fn files(&self) -> Files {
        Files::new()
    }
}

impl GameContext {
    pub fn exit(&mut self) {
        self.request_quit = true;
    }

    fn request_quit(&self) -> bool {
        self.request_quit
    }
}

#[cfg(target_os = "android")]
impl<L: GameLoop> Game<L> {
    pub fn new(android_app: AndroidApp, game_loop: L) -> Self {
        Self {
            app: None,
            game_loop,
            game_time: Instant::now(),
            game_context: GameContext::new(android_app),
            input_events: Vec::with_capacity(10),
        }
    }

    pub fn with_logging(self, level_filter: LevelFilter) -> Self {
        android_logger::init_once(android_logger::Config::default().with_max_level(level_filter));
        self
    }
}

#[cfg(not(target_os = "android"))]
impl<L: GameLoop> Game<L> {
    pub fn new(game_loop: L) -> Self {
        Self {
            app: None,
            game_loop,
            game_time: Instant::now(),
            game_context: GameContext::new(),
            input_events: Vec::with_capacity(10),
        }
    }

    pub fn with_logging(self, level_filter: LevelFilter) -> Self {
        env_logger::builder()
            .filter_level(level_filter) // Default Log Level
            .parse_default_env()
            .init();
        self
    }
}

impl<L: GameLoop> Game<L> {
    pub fn init(&mut self) {
        log::info!("Initializing application...");

        #[cfg(target_os = "android")]
        let event_loop = EventLoop::builder().with_android_app(self.game_context.android_app.clone()).build().unwrap();
        #[cfg(not(target_os = "android"))]
        let event_loop = EventLoop::builder().build().unwrap();

        // init application
        let template = glutin::config::ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(cfg!(cgl_backend));
        let window = winit::window::Window::default_attributes().with_transparent(true).with_title(self.game_loop.title());
        self.app = Some(App::new(template, window));

        // call init callback
        self.game_loop.init(&mut self.game_context);

        // init game time
        self.game_time = Instant::now();

        log::info!("Running game loop...");
        event_loop.run_app(self).unwrap();
    }
}

impl<L: GameLoop> ApplicationHandler for Game<L> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Resuming game loop ...");
        if let Some(app) = self.app.as_mut() {
            app.resume(event_loop);
            self.game_loop.create_device(&mut self.game_context, app.renderer());
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Suspending game loop ...");
        let _ = event_loop;

        if let Some(app) = self.app.as_mut() {
            self.game_loop.destroy_device(&mut self.game_context, app.renderer());
            app.suspend();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                if let Some(app) = self.app.as_mut() {
                    if app.has_surface_and_context() {
                        self.game_loop.render(&mut self.game_context, app.renderer());
                        app.swap_buffers();
                    }
                }
            }
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                if let Some(app) = self.app.as_mut() {
                    if app.has_surface_and_context() {
                        app.resize(size);
                        self.game_loop.resize_device(&mut self.game_context, app.renderer(), size.width, size.height);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input_events.push(InputEvent::Cursor(CursorEvent { location: position.into() }));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.input_events.push(InputEvent::Mouse(MouseEvent {
                    state: state.into(),
                    button: button.into(),
                }));
            }
            WindowEvent::Touch(touch) => {
                self.input_events.push(InputEvent::Touch(touch.into()));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Ok(event) = event.try_into() {
                    self.input_events.push(InputEvent::Keyboard(event));
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;

        // update time
        let new_time = Instant::now();
        let elapsed_time = new_time.duration_since(self.game_time).as_millis() as f32 / 1000.0;
        self.game_time = new_time;

        // call input callback
        self.game_loop.input(&mut self.game_context, &self.input_events);
        self.input_events.clear();

        // call update callback
        self.game_loop.update(&mut self.game_context, elapsed_time);

        if self.game_context.request_quit() {
            event_loop.exit();
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Exiting application...");

        let _ = event_loop;

        // call suspend
        self.suspended(event_loop);

        // cleanup
        if let Some(app) = self.app.as_mut() {
            self.game_loop.cleanup(&mut self.game_context);
            app.exit();
        }
        self.app = None;
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
