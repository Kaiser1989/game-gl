#![cfg(target_os = "android")]

//////////////////////////////////////////////////
// Using

mod context;

use context::{ GameLoop, Runner, gl, Gl, InputEvent};

//////////////////////////////////////////////////
// Entry

#[ndk_glue::main(backtrace)]
pub fn main() {
    let mut game_loop = GameLoop::new(ExampleRunner{});
    game_loop.run();
}

pub struct ExampleRunner {}

impl Runner for ExampleRunner {

    fn init(&mut self) {
        println!("init");
    }

    fn cleanup(&mut self) {
        println!("cleanup");
    }

    fn pause(&mut self) {
        println!("pause");
    }

    fn resume(&mut self) {
        println!("resume");
    }

    fn input(&mut self, input_events: &[InputEvent]) {
        input_events.iter().for_each(|input_event| {
            match input_event {
                _ => println!("input: {:?}", input_event)
            }
        });
    }

    fn update(&mut self, _elapsed_time: f32) {
        //println!("update (time: {}", elapsed_time);
    }

    fn render(&mut self, gl: &Gl) {
        unsafe { 
            gl.ClearColor(1.0, 0.0, 0.0, 1.0); 
            gl.ClearDepthf(1.0);
            gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }
    }

    fn create_device(&mut self, _gl: &Gl) {
        println!("create_device");
    }

    fn destroy_device(&mut self, _gl: &Gl) {
        println!("destroy_device");
    }

    fn resize_device(&mut self, _gl: &Gl, _width: u32, _height: u32) {
        println!("resize_device");
    }
}