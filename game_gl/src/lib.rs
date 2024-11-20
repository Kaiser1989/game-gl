//////////////////////////////////////////////////
// Module

pub mod context;
pub mod events;
pub mod game_app;
pub mod game_loop;
pub mod game_state;
pub mod io;
pub mod opengl;
pub mod test;

//////////////////////////////////////////////////
// Prelude

pub mod prelude {
    pub use crate::context::{ContextExt, GameContext};
    pub use crate::game_loop::{GameLoop, GameLoopRunner};
    pub use crate::opengl::{gl, gl::types::*, Gl, GlResource};
    pub use image;
    #[cfg(target_os = "android")]
    pub use winit::platform::android::activity::AndroidApp;
}

pub mod graphics {
    pub mod prelude {}
}
