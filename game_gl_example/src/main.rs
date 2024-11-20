//////////////////////////////////////////////////
// Using

pub mod game_loop;

use game_gl::prelude::*;

use crate::game_loop::ExampleGameLoop;

//////////////////////////////////////////////////
// Entry point

pub fn main() {
    // start game loop
    ExampleGameLoop::loop_forever();
}
