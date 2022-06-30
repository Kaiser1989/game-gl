//////////////////////////////////////////////////
// Definition

pub struct File {}

//////////////////////////////////////////////////
// Implementations

impl File {
    #[cfg(target_os = "android")]
    pub fn load_bytes(filename: &str) -> Option<Vec<u8>> {
        let asset_manager = ndk_glue::native_activity().asset_manager();
        let mut asset = std::ffi::CString::new(filename).ok().and_then(|filename| asset_manager.open(&filename));
        asset.as_mut().and_then(|asset| asset.get_buffer().ok()).map(|buffer| buffer.to_vec())
    }

    #[cfg(not(target_os = "android"))]
    pub fn load_bytes(filename: &str) -> Option<Vec<u8>> {
        std::fs::read(format!("assets/{}", filename)).ok()
    }

    pub fn load_string(filename: &str) -> Option<String> {
        Self::load_bytes(filename).and_then(|bytes| String::from_utf8(bytes).ok())
    }
}
