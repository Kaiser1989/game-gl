//////////////////////////////////////////////////
// Using

use shrev::ReaderId;
use specs::{RunNow, System, World, WorldExt};

use crate::{
    context::ApplicationContext,
    events::Events,
    game_app::{GameApplicationState, StateEvent},
};

//////////////////////////////////////////////////
// GameState

pub trait GameStateEvent: Send + Sync + 'static {}

pub trait GameState: Default {
    type StateEvent: StateEvent;
    type GameStateEvent: GameStateEvent;

    fn init(&mut self, world: &mut World);

    fn handle_event(&mut self, event: &Self::GameStateEvent, state_events: &mut Events<Self::StateEvent>);

    fn update_systems(&self) -> Vec<Box<dyn for<'a> GameSystem<'a>>> {
        Vec::new()
    }

    fn render_systems(&self) -> Vec<Box<dyn for<'a> GameSystem<'a>>> {
        Vec::new()
    }
}

pub struct GameStateData<G: GameStateEvent> {
    events: Events<G>,
    reader: ReaderId<G>,
    world: World,
    update_systems: Vec<Box<dyn for<'a> GameSystem<'a>>>,
    render_systems: Vec<Box<dyn for<'a> GameSystem<'a>>>,
}

pub struct GameStateWrapper<S: GameState> {
    interface: S,
    data: GameStateData<S::GameStateEvent>,
}

//////////////////////////////////////////////////
// Implementation

impl<G: GameStateEvent> GameStateData<G> {
    pub fn new() -> Self {
        let mut events = Events::new();
        let reader = events.register();
        let world = World::new();
        let update_systems = Vec::new();
        let render_systems = Vec::new();
        Self {
            events,
            reader,
            world,
            update_systems,
            render_systems,
        }
    }
}

impl<S: GameState> GameApplicationState for GameStateWrapper<S> {
    type StateEvent = S::StateEvent;

    fn init(&mut self, ctx: ApplicationContext) {
        // fetch init
        let update_systems = self.interface.update_systems();
        let render_systems = self.interface.render_systems();

        // init world & systems
        self.data.world = World::new();
        self.data.render_systems = update_systems;
        self.data.update_systems = render_systems;

        // add application context to world
        self.data.world.insert(ctx);

        // init state interface and systems (update + render)
        let world = &mut self.data.world;
        self.interface.init(world);
        self.data.update_systems.iter_mut().for_each(|system| {
            system.init(world);
        });
        self.data.render_systems.iter_mut().for_each(|system| {
            system.init(world);
        });
    }

    fn cleanup(&mut self) {
        // cleanup world & systems
        self.data.render_systems.clear();
        self.data.update_systems.clear();
        self.data.world = World::new();
    }

    fn update(&mut self, _elapsed_time: f32, state_events: &mut Events<S::StateEvent>) {
        // TODO: update resources
        // if let Some(game_time) = self.world.get_mut::<GameTime>() {
        //     game_time.update(elapsed_time);
        // }
        // handle events
        for event in self.data.events.read(&mut self.data.reader) {
            self.interface.handle_event(event, state_events);
        }

        // update systems
        let world = &mut self.data.world;
        self.data.update_systems.iter_mut().for_each(|system| {
            system.update(world);
        });

        // persist lazy updates, remove events
        world.maintain();
    }

    fn draw(&mut self) {
        // render systems
        let world = &mut self.data.world;
        self.data.render_systems.iter_mut().for_each(|system| {
            system.update(world);
        });
    }

    fn parent_update(&self) -> bool {
        false
    }

    fn parent_draw(&self) -> bool {
        false
    }
}

//////////////////////////////////////////////////
// GameSystem

pub trait GameSystem<'a> {
    fn init(&mut self, world: &mut World);

    fn update(&mut self, world: &'a World);
}

impl<'a, S> GameSystem<'a> for S
where
    S: System<'a> + 'static,
{
    fn init(&mut self, world: &mut World) {
        RunNow::setup(self, world);
    }

    fn update(&mut self, world: &'a World) {
        self.run_now(world);
    }
}
