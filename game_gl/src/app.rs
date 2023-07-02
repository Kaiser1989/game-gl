//////////////////////////////////////////////////
// Using

use std::ffi::CString;
use std::num::NonZeroU32;
use std::rc::Rc;

use raw_window_handle::{HasRawWindowHandle, RawDisplayHandle, RawWindowHandle};

use winit::event_loop::EventLoopWindowTarget;
#[cfg(glx_backend)]
use winit::platform::unix;

use glutin::config::{Config, ConfigSurfaceTypes, ConfigTemplate, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentContext};
use glutin::display::{Display, DisplayApiPreference};
use glutin::prelude::*;
use glutin::surface::{SurfaceAttributesBuilder, WindowSurface};

use crate::gl;
use crate::opengl::GlString;

//////////////////////////////////////////////////
// Types

pub type Gl = Rc<gl::Gles2>;

//////////////////////////////////////////////////
// Definition

struct SurfaceState {
    window: winit::window::Window,
    surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

pub struct App {
    winsys_display: RawDisplayHandle,
    glutin_display: Option<Display>,
    surface_state: Option<SurfaceState>,
    context: Option<glutin::context::PossiblyCurrentContext>,
    renderer: Option<Gl>,
}

//////////////////////////////////////////////////
// Implementations

impl App {
    pub fn new(winsys_display: RawDisplayHandle) -> Self {
        Self {
            winsys_display,
            glutin_display: None,
            surface_state: None,
            context: None,
            renderer: None,
        }
    }
}

impl App {
    #[allow(unused_variables)]
    pub fn create_display(raw_display: RawDisplayHandle, raw_window_handle: RawWindowHandle) -> Display {
        #[cfg(egl_backend)]
        let preference = DisplayApiPreference::Egl;

        #[cfg(glx_backend)]
        let preference = DisplayApiPreference::Glx(Box::new(unix::register_xlib_error_hook));

        #[cfg(cgl_backend)]
        let preference = DisplayApiPreference::Cgl;

        #[cfg(wgl_backend)]
        let preference = DisplayApiPreference::Wgl(Some(raw_window_handle));

        #[cfg(all(egl_backend, wgl_backend))]
        let preference = DisplayApiPreference::WglThenEgl(Some(raw_window_handle));

        #[cfg(all(egl_backend, glx_backend))]
        let preference = DisplayApiPreference::GlxThenEgl(Box::new(unix::register_xlib_error_hook));

        // Create connection to underlying OpenGL client Api.
        unsafe { Display::new(raw_display, preference).unwrap() }
    }

    pub fn ensure_glutin_display(&mut self, window: &winit::window::Window) {
        if self.glutin_display.is_none() {
            let raw_window_handle = window.raw_window_handle();
            self.glutin_display = Some(Self::create_display(self.winsys_display, raw_window_handle));
        }
    }

    pub fn create_compatible_gl_context(glutin_display: &Display, raw_window_handle: RawWindowHandle, config: &Config) -> NotCurrentContext {
        let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

        // Since glutin by default tries to create OpenGL core context, which may not be
        // present we should try gles.
        let fallback_context_attributes = ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None)).build(Some(raw_window_handle));
        unsafe {
            glutin_display
                .create_context(&config, &context_attributes)
                .unwrap_or_else(|_| glutin_display.create_context(config, &fallback_context_attributes).expect("failed to create context"))
        }
    }

    /// Create template to find OpenGL config.
    pub fn config_template(raw_window_handle: RawWindowHandle) -> ConfigTemplate {
        let builder = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .compatible_with_native_window(raw_window_handle)
            .with_surface_type(ConfigSurfaceTypes::WINDOW);

        #[cfg(cgl_backend)]
        let builder = builder.with_transparency(true).with_multisampling(8);

        builder.build()
    }

    pub fn ensure_surface_and_context<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) {
        let window = winit::window::Window::new(&event_loop).unwrap();
        let raw_window_handle = window.raw_window_handle();

        // Lazily initialize, egl, wgl, glx etc
        self.ensure_glutin_display(&window);
        let glutin_display = self.glutin_display.as_ref().expect("Can't ensure surface + context without a Glutin Display connection");

        let template = Self::config_template(raw_window_handle);
        let config = unsafe {
            glutin_display
                .find_configs(template)
                .unwrap()
                .reduce(|accum, config| {
                    // Find the config with the maximum number of samples.
                    //
                    // In general if you're not sure what you want in template you can request or
                    // don't want to require multisampling for example, you can search for a
                    // specific option you want afterwards.
                    //
                    // XXX however on macOS you can request only one config, so you should do
                    // a search with the help of `find_configs` and adjusting your template.
                    if config.num_samples() > accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .unwrap()
        };
        println!("Picked a config with {} samples", config.num_samples());

        // XXX: Winit is missing a window.surface_size() API and the inner_size may be the wrong
        // size to use on some platforms!
        let (width, height): (u32, u32) = window.inner_size().into();
        let raw_window_handle = window.raw_window_handle();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(raw_window_handle, NonZeroU32::new(width).unwrap(), NonZeroU32::new(height).unwrap());
        let surface = unsafe { glutin_display.create_window_surface(&config, &attrs).unwrap() };
        let surface_state = SurfaceState { window, surface };

        let prev_ctx = self.context.take();
        match prev_ctx {
            Some(ctx) => {
                let not_current_context = ctx.make_not_current().expect("Failed to make GL context not current");
                self.context = Some(not_current_context.make_current(&surface_state.surface).expect("Failed to make GL context current"));
            }
            None => {
                let not_current_context = Self::create_compatible_gl_context(glutin_display, raw_window_handle, &config);
                self.context = Some(not_current_context.make_current(&surface_state.surface).expect("Failed to make GL context current"));
            }
        }

        self.surface_state = Some(surface_state);
    }

    pub fn ensure_renderer(&mut self) {
        let glutin_display = self.glutin_display.as_ref().expect("Can't ensure renderer without a Glutin Display connection");
        self.renderer.get_or_insert_with(|| {
            let gl = Gl::new(gl::Gles2::load_with(|ptr| {
                let ptr = CString::new(ptr).unwrap();
                glutin_display.get_proc_address(ptr.as_c_str()).cast()
            }));

            if let Some(renderer) = GlString::get(&gl, gl::RENDERER) {
                println!("Running on {}", renderer);
            }
            if let Some(version) = GlString::get(&gl, gl::VERSION) {
                println!("OpenGL Version {}", version);
            }

            if let Some(shaders_version) = GlString::get(&gl, gl::SHADING_LANGUAGE_VERSION) {
                println!("Shaders version on {}", shaders_version);
            }
            gl
        });
    }

    pub fn queue_redraw(&self) {
        if let Some(surface_state) = &self.surface_state {
            log::trace!("Making Redraw Request");
            surface_state.window.request_redraw();
        }
    }

    pub fn swap_buffers(&self) {
        let context = self.context.as_ref().expect("Can't swap buffers without context");
        let surface_state = self.surface_state.as_ref().expect("Can't swap buffers without surface");
        if let Err(err) = surface_state.surface.swap_buffers(context) {
            log::error!("Failed to swap buffers: {}", err);
        }
    }

    pub fn resume<T>(&mut self, event_loop: &EventLoopWindowTarget<T>) {
        log::trace!("Resumed, creating render state...");
        self.ensure_surface_and_context(event_loop);
        self.ensure_renderer();
        self.queue_redraw();
    }

    pub fn suspend(&mut self) {
        log::trace!("Suspended, dropping surface state...");
        self.surface_state = None;
    }

    pub fn has_surface_and_context(&self) -> bool {
        self.context.is_some() && self.surface_state.is_some()
    }

    pub fn has_renderer(&self) -> bool {
        self.renderer.is_some()
    }

    pub fn renderer(&self) -> &Gl {
        self.renderer.as_ref().expect("Renderer is not ready")
    }
}
