#![cfg(target_os = "android")]

//////////////////////////////////////////////////
// Using

use std::ffi::CStr;

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}

mod context;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};
use glutin::platform::android::AndroidEventLoop;


//////////////////////////////////////////////////
// Entry

#[ndk_glue::main(backtrace)]
pub fn main() {
    let mut el = AndroidEventLoop::new();
    let mut context_and_handle: Option<ContextAndHandle> = None;

    let mut running = true;
    while running {
        el.run_return(|event, el, control_flow| {
            *control_flow = ControlFlow::Exit;
    
            match event {
                Event::LoopDestroyed => {},
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        if let Some(context_and_handle) = context_and_handle.as_ref() {
                            context_and_handle.context.resize(physical_size);
                        }
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                        running = false;
                    },
                    _ => (),
                },
                Event::RedrawRequested(_) => {
                    if let Some(context_and_handle) = context_and_handle.as_ref() {
                        unsafe { 
                            context_and_handle.gl.ClearColor(1.0, 0.5, 0.7, 1.0);
                            context_and_handle.gl.Clear(gl::COLOR_BUFFER_BIT);

                            // do your rendering here
                        }
                        context_and_handle.context.swap_buffers().unwrap();
                    }
                },
                Event::Suspended => {
                    if context_and_handle.is_some() && ndk_glue::native_window().is_some() {

                        // destroy your graphics resources here

                        context_and_handle = None;
                    }
                },
                Event::Resumed => {
                    if context_and_handle.is_none() && ndk_glue::native_window().is_some() {
                        context_and_handle = Some(init_context(el));

                        // create your graphics resources here
                    }
                },
                _ => (),
            }
        });

        // update your game here
    }
}

struct ContextAndHandle {
    context: WindowedContext<PossiblyCurrent>,
    gl: gl::Gl,
}

fn init_context(el: &EventLoopWindowTarget<()>) -> ContextAndHandle {
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

    ContextAndHandle { context, gl }
}