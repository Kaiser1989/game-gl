//////////////////////////////////////////////////
// Using

use std::convert::TryInto;
use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::time::Instant;

use glutin::config::{Config, ConfigTemplateBuilder, GetGlConfig, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, NotCurrentGlContext, PossiblyCurrentGlContext, Version};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, SwapInterval};
use glutin_winit::GlWindow;
use log::LevelFilter;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes};

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;
#[cfg(target_os = "android")]
use winit::platform::android::EventLoopBuilderExtAndroid;

use crate::context::game::GameContext;
use crate::context::ContextExt;
use crate::io::{CursorEvent, InputEvent, MouseEvent};
use crate::opengl::{gl, Gl, GlString};

//////////////////////////////////////////////////
// GameLoop

pub trait GameLoop: Default {
    fn title(&self) -> &str;

    fn log_level(&self) -> LevelFilter {
        LevelFilter::Off
    }

    fn init(&mut self, context: GameContext);

    fn cleanup(&mut self);

    fn input(&mut self, input_events: &[InputEvent]);

    fn update(&mut self, elapsed_time: f32);

    fn render(&mut self, gl: &Gl);

    fn create_device(&mut self, gl: &Gl);

    fn destroy_device(&mut self, gl: &Gl);

    fn resize_device(&mut self, gl: &Gl, width: u32, height: u32);
}

pub struct GameLoopData {
    app: Option<App>,
    game_time: Instant,
    input_events: Vec<InputEvent>,
}

pub struct GameLoopWrapper<L: GameLoop> {
    interface: L,
    data: GameLoopData,
    ctx: GameContext,
}

//////////////////////////////////////////////////
// GameLoopRunner

pub trait GameLoopRunner {
    #[cfg(target_os = "android")]
    fn loop_forever(app: AndroidApp);

    #[cfg(not(target_os = "android"))]
    fn loop_forever();
}

//////////////////////////////////////////////////
// Implementation

impl GameLoopData {
    fn new() -> Self {
        Self {
            app: None,
            game_time: Instant::now(),
            input_events: Vec::with_capacity(10),
        }
    }
}

impl<L: GameLoop> GameLoopWrapper<L> {
    pub fn new() -> Self {
        Self {
            interface: Default::default(),
            data: GameLoopData::new(),
            ctx: Default::default(),
        }
    }

    pub fn with_logging(self) -> Self {
        #[cfg(target_os = "android")]
        {
            let filter_level = self.interface.log_level();
            android_logger::init_once(android_logger::Config::default().with_max_level(filter_level));
        }
        #[cfg(not(target_os = "android"))]
        {
            let filter_level = self.interface.log_level();
            env_logger::builder().filter_level(filter_level).parse_default_env().init();
        }
        self
    }

    #[cfg(target_os = "android")]
    fn android(&mut self, android_app: AndroidApp) {
        self.ctx.write(|ctx| ctx.init_android(android_app));
    }

    fn run(&mut self) {
        log::info!("Initializing application...");

        #[cfg(target_os = "android")]
        let event_loop = EventLoop::builder().with_android_app(self.ctx.read(|ctx| ctx.android_app().clone())).build().unwrap();
        #[cfg(not(target_os = "android"))]
        let event_loop = EventLoop::builder().build().unwrap();

        // init application
        let template = glutin::config::ConfigTemplateBuilder::new().with_alpha_size(8).with_transparency(cfg!(cgl_backend));
        let window = winit::window::Window::default_attributes().with_transparent(true).with_title(self.interface.title());
        self.data.app = Some(App::new(template, window));

        // call init callback
        self.interface.init(self.ctx.clone());

        // init game time
        self.data.game_time = Instant::now();

        log::info!("Running game loop...");
        event_loop.run_app(self).unwrap();
    }
}

//////////////////////////////////////////////////
// App

struct AppState {
    window: winit::window::Window,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

enum GlDisplayCreationState {
    /// The display was not build yet.
    Build,
    /// The display was already created for the application.
    Init,
}

struct App {
    template: ConfigTemplateBuilder,
    window: WindowAttributes,
    display: GlDisplayCreationState,
    context: Option<glutin::context::PossiblyCurrentContext>,
    state: Option<AppState>,
    renderer: Option<Gl>,
    exit_state: Result<(), Box<dyn Error>>,
}

//////////////////////////////////////////////////
// Implementations

impl App {
    pub fn new(template: ConfigTemplateBuilder, window: WindowAttributes) -> Self {
        Self {
            template,
            window,
            display: GlDisplayCreationState::Build,
            exit_state: Ok(()),
            context: None,
            state: None,
            renderer: None,
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Option<(Window, Config)> {
        let (window, gl_config) = match self.display {
            // We just created the event loop, so initialize the display, pick the config, and
            // create the context.
            GlDisplayCreationState::Build => {
                let display_builder = glutin_winit::DisplayBuilder::new().with_window_attributes(Some(self.window.clone()));
                let (window, gl_config) = match display_builder.build(event_loop, self.template.clone(), gl_config_picker) {
                    Ok((window, gl_config)) => (window.unwrap(), gl_config),
                    Err(err) => {
                        self.exit_state = Err(err);
                        event_loop.exit();
                        return None;
                    }
                };

                log::debug!("Picked a config with {} samples", gl_config.num_samples());

                // Mark the display as initialized to not recreate it on resume, since the
                // display is valid until we explicitly destroy it.
                self.display = GlDisplayCreationState::Init;

                // Create gl context.
                self.context = Some(create_gl_context(&window, &gl_config).treat_as_possibly_current());

                (window, gl_config)
            }
            GlDisplayCreationState::Init => {
                // Pick the config which we already use for the context.
                let gl_config = self.context.as_ref().unwrap().config();
                match glutin_winit::finalize_window(event_loop, self.window.clone(), &gl_config) {
                    Ok(window) => (window, gl_config),
                    Err(err) => {
                        self.exit_state = Err(err.into());
                        event_loop.exit();
                        return None;
                    }
                }
            }
        };
        Some((window, gl_config))
    }

    pub fn create_renderer<D: GlDisplay>(&mut self, gl_display: &D) {
        self.renderer.get_or_insert_with(|| {
            let gl = gl::Gles2::load_with(|ptr| {
                let ptr = CString::new(ptr).unwrap();
                gl_display.get_proc_address(ptr.as_c_str()).cast()
            });

            if let Some(renderer) = GlString::get(&gl, gl::RENDERER) {
                log::debug!("Running on {}", renderer);
            }
            if let Some(version) = GlString::get(&gl, gl::VERSION) {
                log::debug!("OpenGL Version {}", version);
            }

            if let Some(shaders_version) = GlString::get(&gl, gl::SHADING_LANGUAGE_VERSION) {
                log::debug!("Shaders version on {}", shaders_version);
            }
            gl
        });
    }

    pub fn resume(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Window resumed");

        let (window, gl_config) = self.create_window(event_loop).unwrap();
        let attrs = window.build_surface_attributes(Default::default()).expect("Failed to build surface attributes");
        let gl_surface = unsafe { gl_config.display().create_window_surface(&gl_config, &attrs).unwrap() };

        // The context needs to be current for the Renderer to set up shaders and
        // buffers. It also performs function loading, which needs a current context on
        // WGL.
        let gl_context = self.context.as_ref().unwrap();
        gl_context.make_current(&gl_surface).unwrap();

        // Try setting vsync.
        if let Err(res) = gl_surface.set_swap_interval(gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())) {
            log::error!("Error setting vsync: {res:?}");
        }

        self.create_renderer(&gl_config.display());

        assert!(self.state.replace(AppState { surface: gl_surface, window }).is_none());
    }

    pub fn suspend(&mut self) {
        // This event is only raised on Android, where the backing NativeWindow for a GL
        // Surface can appear and disappear at any moment.
        log::debug!("Window removed");

        // Destroy the GL Surface and un-current the GL Context before ndk-glue releases
        // the window back to the system.
        self.state = None;

        // Make context not current.
        self.context = Some(self.context.take().unwrap().make_not_current().unwrap().treat_as_possibly_current());
    }

    pub fn swap_buffers(&mut self) {
        if let Some(AppState { surface, window }) = self.state.as_ref() {
            let gl_context = self.context.as_ref().unwrap();
            window.request_redraw();
            surface.swap_buffers(gl_context).unwrap();
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        // Some platforms like EGL require resizing GL surface to update the size
        // Notable platforms here are Wayland and macOS, other don't require it
        // and the function is no-op, but it's wise to resize it for portability
        // reasons.
        if let Some(AppState { surface, window: _ }) = self.state.as_ref() {
            let gl_context = self.context.as_ref().unwrap();
            surface.resize(gl_context, NonZeroU32::new(size.width).unwrap(), NonZeroU32::new(size.height).unwrap());
        }
    }

    pub fn exit(&mut self) {
        // NOTE: The handling below is only needed due to nvidia on Wayland to not crash
        // on exit due to nvidia driver touching the Wayland display from on
        // `exit` hook.
        let _gl_display = self.context.take().unwrap().display();

        // Clear the window.
        self.state = None;
        #[cfg(egl_backend)]
        #[allow(irrefutable_let_patterns)]
        if let glutin::display::Display::Egl(display) = _gl_display {
            unsafe {
                display.terminate();
            }
        }
    }

    pub fn has_surface_and_context(&self) -> bool {
        self.context.is_some() && self.state.is_some()
    }

    pub fn renderer(&self) -> &Gl {
        self.renderer.as_ref().expect("Renderer is not ready")
    }
}

pub fn gl_config_picker(configs: Box<dyn Iterator<Item = Config> + '_>) -> Config {
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
}

fn create_gl_context(window: &Window, gl_config: &Config) -> NotCurrentContext {
    let raw_window_handle = window.window_handle().ok().map(|wh| wh.as_raw());

    // The context creation part.
    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(raw_window_handle);

    // There are also some old devices that support neither modern OpenGL nor GLES.
    // To support these we can try and create a 2.1 context.
    let legacy_context_attributes = ContextAttributesBuilder::new().with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1)))).build(raw_window_handle);

    // Reuse the uncurrented context from a suspended() call if it exists, otherwise
    // this is the first time resumed() is called, where the context still
    // has to be created.
    let gl_display = gl_config.display();

    unsafe {
        gl_display.create_context(gl_config, &context_attributes).unwrap_or_else(|_| {
            gl_display
                .create_context(gl_config, &fallback_context_attributes)
                .unwrap_or_else(|_| gl_display.create_context(gl_config, &legacy_context_attributes).expect("failed to create context"))
        })
    }
}

//////////////////////////////////////////////////
// GameLoopRunner

impl<L: GameLoop> GameLoopRunner for L {
    #[cfg(target_os = "android")]
    fn loop_forever(app: AndroidApp) {
        GameLoopWrapper::<L>::new().android(app).with_logging().run();
    }

    #[cfg(not(target_os = "android"))]
    fn loop_forever() {
        GameLoopWrapper::<L>::new().with_logging().run();
    }
}

//////////////////////////////////////////////////
// ApplicationHandler

impl<L: GameLoop> ApplicationHandler for GameLoopWrapper<L> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Resuming game loop ...");
        if let Some(app) = self.data.app.as_mut() {
            app.resume(event_loop);
            self.interface.create_device(app.renderer());
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Suspending game loop ...");
        let _ = event_loop;

        if let Some(app) = self.data.app.as_mut() {
            self.interface.destroy_device(app.renderer());
            app.suspend();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                if let Some(app) = self.data.app.as_mut() {
                    if app.has_surface_and_context() {
                        self.interface.render(app.renderer());
                        app.swap_buffers();
                    }
                }
            }
            WindowEvent::Resized(size) if size.width != 0 && size.height != 0 => {
                if let Some(app) = self.data.app.as_mut() {
                    if app.has_surface_and_context() {
                        app.resize(size);
                        self.interface.resize_device(app.renderer(), size.width, size.height);
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.data.input_events.push(InputEvent::Cursor(CursorEvent { location: position.into() }));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.data.input_events.push(InputEvent::Mouse(MouseEvent {
                    state: state.into(),
                    button: button.into(),
                }));
            }
            WindowEvent::Touch(touch) => {
                self.data.input_events.push(InputEvent::Touch(touch.into()));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Ok(event) = event.try_into() {
                    self.data.input_events.push(InputEvent::Keyboard(event));
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
        let elapsed_time = new_time.duration_since(self.data.game_time).as_millis() as f32 / 1000.0;
        self.data.game_time = new_time;

        // call input callback
        self.interface.input(&self.data.input_events);
        self.data.input_events.clear();

        // call update callback
        self.interface.update(elapsed_time);

        // check for exit request
        if self.ctx.read(|ctx| ctx.request_exit()) {
            event_loop.exit();
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Exiting application...");

        let _ = event_loop;

        // call suspend
        self.suspended(event_loop);

        // cleanup
        if let Some(app) = self.data.app.as_mut() {
            self.interface.cleanup();
            app.exit();
        }
        self.data.app = None;
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
