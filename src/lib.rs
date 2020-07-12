#![cfg(target_os = "android")]

//////////////////////////////////////////////////
// Using

mod context;

use context::{ GameLoop, Runner, DeviceContext };

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

    fn input(&mut self) {
        println!("input");
    }

    fn update(&mut self, elapsed_time: f32) {
        //println!("update (time: {}", elapsed_time);
    }

    fn render(&mut self, _device_ctx: &mut DeviceContext) {
        println!("render");
    }

    fn create_device(&mut self, _device_ctx: &mut DeviceContext) {
        println!("create_device");
    }

    fn resize_device(&mut self, _device_ctx: &mut DeviceContext) {
        println!("resize_device");
    }

    fn destroy_device(&mut self, _device_ctx: &mut DeviceContext) {
        println!("destroy_device");
    }
}