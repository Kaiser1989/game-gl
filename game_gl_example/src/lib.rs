//////////////////////////////////////////////////
// Using

pub mod game_loop;

use game_gl::prelude::*;

use crate::game_loop::ExampleGameLoop;

//////////////////////////////////////////////////
// Entry point for android

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    // start game loop
    ExampleGameLoop::loop_forever(app);
}

// declared as pub to avoid dead_code warnings from cdylib target build
#[cfg(not(target_os = "android"))]
pub fn main() {
    // start game loop
    ExampleGameLoop::loop_forever();
}
