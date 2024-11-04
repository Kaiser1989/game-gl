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
    Game::new(app, ExampleGameLoop::default()).with_logging().init();
}

// declared as pub to avoid dead_code warnings from cdylib target build
#[cfg(not(target_os = "android"))]
pub fn main() {
    // start game loop
    Game::new(ExampleGameLoop::default()).with_logging().init();
}
