//////////////////////////////////////////////////
// Using

//////////////////////////////////////////////////
// GameState

use shrev::ReaderId;

use super::Events;

pub trait GameStateEvent: Sized + Clone + Send + Sync + 'static {}

pub trait GameState {
    type Event: GameStateEvent;

    fn data(&mut self) -> &mut GameStateData<Self::Event>;

    fn handle_event(&mut self, event: Self::Event, state_events: &mut Events<StateEvent>);

    fn gui(&self, _resource: &ResourceContext) -> Option<GuiBuilder<Self::Event>> {
        None
    }

    fn scene(&self, _resource: &ResourceContext) -> Option<SceneBuilder> {
        None
    }
}

//////////////////////////////////////////////////
// GameStateData

pub struct GameStateData<E: GameStateEvent> {
    config: Config,
    events: Events<E>,
    reader: ReaderId<E>,
    gui: Option<Gui<E>>,
    scene: Option<Scene<E>>,
}

impl<E: GameStateEvent> GameStateData<E> {
    pub fn new(config: &Config) -> Self {
        let config = config.clone();
        let mut events = Events::new();
        let reader = events.register();
        let gui = None;
        let scene = None;
        Self { config, events, reader, gui, scene }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}
