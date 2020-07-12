//////////////////////////////////////////////////
// Modules

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}


//////////////////////////////////////////////////
// Using

use std::ffi::CStr;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};
use glutin::platform::android::AndroidEventLoop;


//////////////////////////////////////////////////
// Callbacks

type OptBoxCallback<T> = Option<Box<T>>;


//////////////////////////////////////////////////
// Runner



//////////////////////////////////////////////////
// Definition

pub trait Runner {}


pub struct ContextBuilder {

    init: OptBoxCallback<dyn Fn()>,
    cleanup: OptBoxCallback<dyn Fn()>,
}


pub struct Context {
    device: Option<Device>
}

pub struct Device {

}

pub fn init() {

}


//////////////////////////////////////////////////
// Implementation

impl ContextBuilder {
    
    pub fn new() -> ContextBuilder {
        ContextBuilder { init: None, cleanup: None }
    }

    pub fn with_init<F>(self, func: F) 
        where F: Fn() -> ()
    -> ContextBuilder {
        self.init = Some(Box::new(func)); 
        self
    }

    pub fn build() -> Context {
        Context::new()
    }
}

impl Context {

    pub fn new() -> Context {
        Context { 
            device: None 
        }
    }

    fn createDevice(&mut self, el: &EventLoopWindowTarget<()>) {
        let wb = WindowBuilder::new().with_title("A fantastic window!");
        let context = ContextBuilder::new()
            .with_gl(GlRequest::Specific(Api::OpenGlEs, (2, 0)))
            .with_gl_debug_flag(false)
            .with_srgb(false)
            .with_vsync(true)
            .build_windowed(wb, &el)
            .unwrap();
        let context = unsafe { context.make_current().unwrap() };
    
        println!("Pixel format of the window's GL context: {:?}", context.get_pixel_format());
    
        let gl = gl::Gl::load_with(|ptr| context.get_proc_address(ptr) as *const _);
    
        let version = unsafe {
            let data = CStr::from_ptr(gl.GetString(gl::VERSION) as *const _)
                .to_bytes()
                .to_vec();
            String::from_utf8(data).unwrap()
        };
    
        println!("OpenGL version {}", version);
    }
}