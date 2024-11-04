//////////////////////////////////////////////////
// Using

#[cfg(target_os = "android")]
use ndk::asset::AssetManager;
#[cfg(target_os = "android")]
use std::ffi::CString;
#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

//////////////////////////////////////////////////
// Definition

pub struct Files {
    #[cfg(target_os = "android")]
    asset_manager: AssetManager,
}

//////////////////////////////////////////////////
// Implementations

#[cfg(target_os = "android")]
impl Files {
    pub fn new(android_app: &AndroidApp) -> Self {
        Files {
            asset_manager: android_app.asset_manager(),
        }
    }

    pub fn load_bytes(&self, filename: &str) -> Option<Vec<u8>> {
        let mut asset = CString::new(filename).ok().and_then(|filename| self.asset_manager.open(&filename));
        asset.as_mut().and_then(|asset| asset.buffer().ok()).map(|buffer| buffer.to_vec())
    }
}

#[cfg(not(target_os = "android"))]
impl Files {
    pub fn new() -> Self {
        Files {}
    }

    pub fn load_bytes(&self, filename: &str) -> Option<Vec<u8>> {
        std::fs::read(format!("assets/{}", filename)).ok()
    }
}

impl Files {
    pub fn load_string(&self, filename: &str) -> Option<String> {
        self.load_bytes(filename).and_then(|bytes| String::from_utf8(bytes).ok())
    }
}
