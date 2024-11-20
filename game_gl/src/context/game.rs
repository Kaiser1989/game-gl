//////////////////////////////////////////////////
// Using

use std::sync::{Arc, RwLock};

use crate::io::Files;

//////////////////////////////////////////////////
// Definition

pub type GameContext = Arc<RwLock<RawGameContext>>;

#[derive(Debug, Default)]
pub struct RawGameContext {
    #[cfg(target_os = "android")]
    android_app: Option<AndroidApp>,
    request_exit: bool,
}

//////////////////////////////////////////////////
// Implementation

impl RawGameContext {
    #[cfg(target_os = "android")]
    pub(crate) fn init_android(&mut self, android_app: AndroidApp) {
        self.android_app = Some(android_app);
    }

    pub(crate) fn request_exit(&self) -> bool {
        self.request_exit
    }

    #[cfg(target_os = "android")]
    pub fn android_app() -> &AndroidApp {
        self.android_app.as_ref().expect("Android app is not initialized")
    }

    pub fn files(&self) -> Files {
        Files::new()
    }

    pub fn exit(&mut self) {
        self.request_exit = true;
    }
}
