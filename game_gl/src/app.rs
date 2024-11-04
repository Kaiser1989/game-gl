//////////////////////////////////////////////////
// Using

use std::error::Error;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::rc::Rc;

use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;

use winit::dpi::PhysicalSize;
use winit::event_loop::ActiveEventLoop;
#[cfg(glx_backend)]
use winit::platform::unix;

use glutin::config::{Config, ConfigTemplateBuilder, GetGlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::SwapInterval;
use winit::window::{Window, WindowAttributes};

use crate::gl;
use crate::opengl::GlString;

//////////////////////////////////////////////////
// Types

pub type Gl = Rc<gl::Gles2>;

//////////////////////////////////////////////////
// Definition

struct AppState {
    window: winit::window::Window,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

pub struct App {
    template: ConfigTemplateBuilder,
    display: GlDisplayCreationState,
    context: Option<glutin::context::PossiblyCurrentContext>,
    state: Option<AppState>,
    renderer: Option<Gl>,
    exit_state: Result<(), Box<dyn Error>>,
}

enum GlDisplayCreationState {
    /// The display was not build yet.
    Builder(DisplayBuilder),
    /// The display was already created for the application.
    Init,
}

//////////////////////////////////////////////////
// Implementations

impl App {
    pub fn new(template: ConfigTemplateBuilder, display_builder: DisplayBuilder) -> Self {
        Self {
            template,
            display: GlDisplayCreationState::Builder(display_builder),
            exit_state: Ok(()),
            context: None,
            state: None,
            renderer: None,
        }
    }
}

impl App {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) -> Option<(Window, Config)> {
        let (window, gl_config) = match &self.display {
            // We just created the event loop, so initialize the display, pick the config, and
            // create the context.
            GlDisplayCreationState::Builder(display_builder) => {
                let (window, gl_config) = match display_builder.clone().build(event_loop, self.template.clone(), gl_config_picker) {
                    Ok((window, gl_config)) => (window.unwrap(), gl_config),
                    Err(err) => {
                        self.exit_state = Err(err);
                        event_loop.exit();
                        return None;
                    }
                };

                println!("Picked a config with {} samples", gl_config.num_samples());

                // Mark the display as initialized to not recreate it on resume, since the
                // display is valid until we explicitly destroy it.
                self.display = GlDisplayCreationState::Init;

                // Create gl context.
                self.context = Some(create_gl_context(&window, &gl_config).treat_as_possibly_current());

                (window, gl_config)
            }
            GlDisplayCreationState::Init => {
                println!("Recreating window in `resumed`");
                // Pick the config which we already use for the context.
                let gl_config = self.context.as_ref().unwrap().config();
                match glutin_winit::finalize_window(event_loop, window_attributes(), &gl_config) {
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
            let gl = Gl::new(gl::Gles2::load_with(|ptr| {
                let ptr = CString::new(ptr).unwrap();
                gl_display.get_proc_address(ptr.as_c_str()).cast()
            }));

            if let Some(renderer) = GlString::get(&gl, gl::RENDERER) {
                log::info!("Running on {}", renderer);
            }
            if let Some(version) = GlString::get(&gl, gl::VERSION) {
                log::info!("OpenGL Version {}", version);
            }

            if let Some(shaders_version) = GlString::get(&gl, gl::SHADING_LANGUAGE_VERSION) {
                log::info!("Shaders version on {}", shaders_version);
            }
            gl
        });
    }

    pub fn resume(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Android window resumed");

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
        log::debug!("Android window removed");

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

    pub fn has_renderer(&self) -> bool {
        self.renderer.is_some()
    }

    pub fn renderer(&self) -> &Gl {
        self.renderer.as_ref().expect("Renderer is not ready")
    }
}

fn window_attributes() -> WindowAttributes {
    Window::default_attributes()
        .with_transparent(true)
        .with_title("Glutin triangle gradient example (press Escape to exit)")
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
