//////////////////////////////////////////////////
// Modules

pub mod input;

pub mod graphics;

pub mod resource;

pub mod game;
pub use game::GameContext;

//////////////////////////////////////////////////
// Using

use std::sync::{Arc, RwLock};

use graphics::RawGraphicsContext;
use input::RawInputContext;
use resource::RawResourceContext;

//////////////////////////////////////////////////
// Context Traits

pub trait ContextExt<T> {
    fn read<R>(&self, exec: impl Fn(&T) -> R) -> R;

    fn write<R>(&self, exec: impl Fn(&mut T) -> R) -> R;
}

impl<T> ContextExt<T> for Arc<RwLock<T>> {
    fn read<R>(&self, exec: impl Fn(&T) -> R) -> R {
        let t = self.as_ref().read().unwrap();
        exec(&t)
    }

    fn write<R>(&self, exec: impl Fn(&mut T) -> R) -> R {
        let mut t = self.as_ref().write().unwrap();
        exec(&mut t)
    }
}

//////////////////////////////////////////////////
// ApplicationContext

pub type ResourceContext = Arc<RwLock<RawResourceContext>>;
pub type GraphicsContext = Arc<RwLock<RawGraphicsContext>>;
pub type InputContext = Arc<RwLock<RawInputContext>>;

#[derive(Debug, Default)]
pub struct RawApplicationContext {
    game: Option<GameContext>,
    resource: ResourceContext,
    graphics: GraphicsContext,
    input: InputContext,
}

pub type ApplicationContext = Arc<RawApplicationContext>;

//////////////////////////////////////////////////
// Implementation

impl RawApplicationContext {
    pub(crate) fn init(&mut self, game_context: GameContext) {
        self.game = Some(game_context);
    }

    pub fn game(&self) -> &GameContext {
        self.game.as_ref().expect("GameContext is not initialized")
    }

    pub fn resource(&self) -> &ResourceContext {
        &self.resource
    }

    pub fn graphics(&self) -> &GraphicsContext {
        &self.graphics
    }

    pub fn input(&self) -> &InputContext {
        &self.input
    }
}
