//////////////////////////////////////////////////
// Using

use glutin::Api;

//////////////////////////////////////////////////
// Definitions

pub struct Config {}

//////////////////////////////////////////////////
// Android

#[cfg(target_os = "android")]
impl Config {
    pub fn srgb_support() -> bool {
        false
    }
    pub fn vsync_support() -> bool {
        true
    }
    pub fn opengl_api_support() -> Api {
        Api::OpenGlEs
    }
    pub fn opengl_version_support() -> (u8, u8) {
        (3, 0)
    }
}

//////////////////////////////////////////////////
// Windows

#[cfg(not(target_os = "android"))]
impl Config {
    pub fn srgb_support() -> bool {
        true
    }
    pub fn vsync_support() -> bool {
        true
    }
    pub fn opengl_api_support() -> Api {
        Api::OpenGl
    }
    pub fn opengl_version_support() -> (u8, u8) {
        (4, 5)
    }
}
