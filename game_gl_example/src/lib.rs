#![cfg(target_os = "android")]

//////////////////////////////////////////////////
// Using

pub mod game_loop;

use game_gl::prelude::*;

use crate::game_loop::ExampleGameLoop;

//////////////////////////////////////////////////
// Entry point for android

#[no_mangle]
fn android_main(app: AndroidApp) {
    // start game loop
    ExampleGameLoop::loop_forever(app);
}
