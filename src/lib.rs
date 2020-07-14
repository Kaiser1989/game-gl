//////////////////////////////////////////////////
// Modules

pub mod gl {
    pub use self::Gles2 as Gl;
    include!(concat!(env!("OUT_DIR"), "/gl_bindings.rs"));
}


//////////////////////////////////////////////////
// Using

use std::ffi::CStr;
use std::time::Instant;

use glutin::event::{Event, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoopWindowTarget};
use glutin::window::WindowBuilder;
use glutin::{Api, ContextBuilder, GlRequest, PossiblyCurrent, WindowedContext};

#[cfg(target_os = "android")]
use glutin::platform::android::AndroidEventLoop as EventLoop;
#[cfg(not(target_os = "android"))]
use glutin::event_loop::EventLoop;


use gl::types::GLenum;

pub use gl::Gl;


//////////////////////////////////////////////////
// Traits

pub trait Runner {

    fn init(&mut self);

    fn cleanup(&mut self);

    fn pause(&mut self);

    fn resume(&mut self);

    fn input(&mut self, input_events: &[InputEvent]);

    fn update(&mut self, elapsed_time: f32);

    fn render(&mut self, gl: &Gl);

    fn create_device(&mut self, gl: &Gl);

    fn destroy_device(&mut self, gl: &Gl);

    fn resize_device(&mut self, gl: &Gl, width: u32, height: u32);
}


//////////////////////////////////////////////////
// Game loop

pub struct GameLoop<T: Runner> {
    runner: T,
    device_ctx: Option<DeviceContext>,
}

impl<T: Runner> GameLoop<T> {

    pub fn new(runner: T) -> GameLoop<T> {
        Self { runner, device_ctx: None }
    }

    pub fn run(&mut self) {
        // call init callback
        self.runner.init();

        // start game time
        let mut time = Instant::now();

        // init input
        let mut input_events: Vec<InputEvent> = Vec::with_capacity(10);

        // init game loop
        let mut event_loop = EventLoop::new();

        // WINDOWS: create context
        #[cfg(not(target_os = "android"))]
        {
            self.device_ctx = Some(DeviceContext::new(&event_loop));
            if let Some(device_ctx) = self.device_ctx.as_mut() {
                // call create device callback
                self.runner.create_device(&device_ctx.gl);
            }
        }

        // starting game loop
        let mut running = true;
        while running {

            // check glutin events
            event_loop.run_return(|event, _event_loop, control_flow| {
                *control_flow = ControlFlow::Exit;
        
                match event {
                    Event::Resumed => {

                        // ANDROID: only create if native window is available
                        #[cfg(target_os = "android")]
                        {
                            if self.device_ctx.is_none() && ndk_glue::native_window().is_some() {
                                self.device_ctx = Some(DeviceContext::new(_event_loop));
                                if let Some(device_ctx) = self.device_ctx.as_mut() {
                                    // call create device callback
                                    self.runner.create_device(&device_ctx.gl);
                                }
                            }
                        }

                        // call resume callback
                        self.runner.resume();

                    },
                    Event::Suspended => {

                        // call pause callback
                        self.runner.pause();

                        // ANDROID: only destroy if native window is available
                        #[cfg(target_os = "android")]
                        {
                            if self.device_ctx.is_some() && ndk_glue::native_window().is_some() {
                                if let Some(device_ctx) = self.device_ctx.as_mut() {
                                    // call destroy device callback
                                    self.runner.destroy_device(&device_ctx.gl);
                                }
                                self.device_ctx = None;
                            }
                        }

                    },
                    Event::RedrawRequested(_) => {

                        if let Some(device_ctx) = self.device_ctx.as_mut() {
                            unsafe { 
                                device_ctx.gl.ClearColor(1.0, 0.0, 0.0, 1.0); 
                                device_ctx.gl.ClearDepthf(1.0);
                                device_ctx.gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                            }

                            // call render callback
                            self.runner.render(&device_ctx.gl);

                            device_ctx.window_context.swap_buffers().unwrap();
                        }
                    },
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::Resized(physical_size) => {
                            if let Some(device_ctx) = self.device_ctx.as_mut() {
                                device_ctx.window_context.resize(physical_size);

                                // call resize device callback
                                self.runner.resize_device(&device_ctx.gl, physical_size.width, physical_size.height);
                            }
                        },
                        WindowEvent::CloseRequested => {
                            running = false;
                        },
                        WindowEvent::CursorMoved { position, ..} => {
                            input_events.push(
                                InputEvent::Cursor(
                                    CursorEvent { location: position.into() }
                                )
                            );
                        },
                        WindowEvent::MouseInput { state, button, ..} => {
                            input_events.push(
                                InputEvent::Mouse(
                                    MouseEvent { state: state.into(), button: button.into() }
                                )
                            );
                        },
                        WindowEvent::Touch(touch) => {
                            input_events.push(
                                InputEvent::Touch(
                                    touch.into()
                                )
                            );
                        },
                        WindowEvent::KeyboardInput { input, ..} => {
                            input_events.push(
                                InputEvent::Keyboard(
                                    input.into()
                                )
                            );
                        }
                        _ => (),
                    },
                    Event::MainEventsCleared => {
                        // update time
                        let new_time = Instant::now();
                        let elapsed_time = new_time.duration_since(time).as_millis() as f32 / 1000.0;
                        time = new_time;

                        // process input
                        self.runner.input(&input_events);
                        input_events.clear();

                        // call update callback
                        self.runner.update(elapsed_time);

                        // render call
                        if let Some(device_ctx) = self.device_ctx.as_mut() {

                            // call render callback
                            self.runner.render(&device_ctx.gl);

                            device_ctx.window_context.swap_buffers().unwrap();
                        }
                    },
                    _ => (),
                }
            });
        }

        // WINDOWS: destroy context
        #[cfg(not(target_os = "android"))]
        {
            if let Some(device_ctx) = self.device_ctx.as_mut() {
                // call destroy device callback
                self.runner.destroy_device(&device_ctx.gl);
            }
            self.device_ctx = None;
        }

        // call cleanup callback
        self.runner.cleanup();
    }
}


//////////////////////////////////////////////////
// Input

#[derive(Debug)]
pub enum InputEvent {
    Cursor(CursorEvent),
    Mouse(MouseEvent),
    Touch(TouchEvent),
    Keyboard(KeyboardEvent)
}

#[derive(Debug)]
pub struct CursorEvent {
    pub location: Location,
}

#[derive(Debug)]
pub struct MouseEvent {
    pub state: MouseState,
    pub button: MouseButton,
}

#[derive(Debug, Clone)]
pub enum MouseState {
    Pressed,
    Released,
}

#[derive(Debug, Clone)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Other(u8),
}

#[derive(Debug)]
pub struct TouchEvent {
    pub state: TouchState,
    pub location: Location,
    pub id: u64,
}

#[derive(Debug, Clone)]
pub enum TouchState {
    Down,
    Up,
    Move,
    Cancelled,
}

#[derive(Debug)]
pub struct KeyboardEvent {
    pub state: KeyState,
    pub key: u32,
}

#[derive(Debug, Clone)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

impl From<glutin::dpi::PhysicalPosition<f64>> for Location {
    fn from(e: glutin::dpi::PhysicalPosition<f64>) -> Location {
        Location { x: e.x as f32, y: e.y as f32 }
    }
}

impl From<glutin::event::ElementState> for MouseState {
    fn from(e: glutin::event::ElementState) -> MouseState {
        match e { 
            glutin::event::ElementState::Pressed => MouseState::Pressed,
            glutin::event::ElementState::Released => MouseState::Released,
        }
    }
}

impl From<glutin::event::MouseButton> for MouseButton {
    fn from(e: glutin::event::MouseButton) -> MouseButton {
        match e {
            glutin::event::MouseButton::Left => MouseButton::Left,
            glutin::event::MouseButton::Middle => MouseButton::Middle,
            glutin::event::MouseButton::Right => MouseButton::Right,
            glutin::event::MouseButton::Other(x) => MouseButton::Other(x),
        }
    }
}

impl From<glutin::event::Touch> for TouchEvent {
    fn from(e: glutin::event::Touch) -> TouchEvent {
        let glutin::event::Touch { phase, location, id, .. } = e;
        TouchEvent { state: phase.into(), location: location.into(), id }
    }
}

impl From<glutin::event::TouchPhase> for TouchState {
    fn from(e: glutin::event::TouchPhase) -> TouchState {
        match e {
            glutin::event::TouchPhase::Started => TouchState::Down,
            glutin::event::TouchPhase::Ended => TouchState::Up,
            glutin::event::TouchPhase::Moved => TouchState::Move,
            glutin::event::TouchPhase::Cancelled => TouchState::Cancelled,
        }
    }
}

impl From<glutin::event::ElementState> for KeyState {
    fn from(e: glutin::event::ElementState) -> KeyState {
        match e { 
            glutin::event::ElementState::Pressed => KeyState::Pressed,
            glutin::event::ElementState::Released => KeyState::Released,
        }
    }
}

impl From<glutin::event::KeyboardInput> for KeyboardEvent {
    fn from(e: glutin::event::KeyboardInput) -> KeyboardEvent {
        let glutin::event::KeyboardInput { scancode, state, ..} = e;
        KeyboardEvent { state: state.into(), key: scancode }
    }
}


//////////////////////////////////////////////////
// Internal Device

struct DeviceContext {
    gl: Gl,
    window_context: WindowedContext<PossiblyCurrent>,
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
        let gl = Gl::load_with(|ptr| window_context.get_proc_address(ptr) as *const _);
        let device_ctx = DeviceContext{ gl, window_context };
        device_ctx.print_context();
        device_ctx
    }

    fn print_context(&self) {
        println!("OpenGL device:\n-> {:?}\n-> {:?}", self.window_context.get_pixel_format(), self.get_string(gl::VERSION));
    }

    // +++ Starting OpenGL functions

    fn get_string(&self, gl_enum: GLenum) -> String {
        unsafe {
            let data = CStr::from_ptr(self.gl.GetString(gl_enum) as *const _).to_bytes().to_vec();
            String::from_utf8(data).unwrap()
        }
    }
}