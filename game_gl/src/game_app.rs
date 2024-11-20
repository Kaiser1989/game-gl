//////////////////////////////////////////////////
// Using

use std::sync::Arc;

use nalgebra_glm::vec2;
use shrev::ReaderId;

use crate::context::game::GameContext;
use crate::context::{ApplicationContext, ContextExt};
use crate::events::Events;
use crate::game_loop::{GameLoop, GameLoopRunner};
use crate::io::InputEvent;
use crate::opengl::Gl;

//////////////////////////////////////////////////
// FAKE

#[derive(Default)]
pub struct ResourceContext {}

//////////////////////////////////////////////////
// GameApplication

pub trait StateEvent: Send + Sync + 'static {}

pub trait GameApplication: Default {
    type StateEvent: StateEvent;

    fn title(&self) -> &str;

    fn init(&mut self, context: ApplicationContext);

    fn start_event(&self) -> Self::StateEvent;

    fn handle_event(&mut self, events: &Self::StateEvent);
}

pub struct GameApplicationData<E: StateEvent> {
    states: Vec<Box<dyn GameApplicationState<StateEvent = E>>>,
    events: Events<E>,
    reader: ReaderId<E>,
}

pub struct GameApplicationWrapper<A: GameApplication> {
    interface: A,
    data: GameApplicationData<A::StateEvent>,
    ctx: ApplicationContext,
}

pub(crate) trait GameApplicationState {
    type StateEvent: StateEvent;

    fn init(&mut self, ctx: ApplicationContext);

    fn cleanup(&mut self);

    fn update(&mut self, elapsed_time: f32, state_events: &mut Events<Self::StateEvent>);

    fn draw(&mut self);

    fn parent_update(&self) -> bool;

    fn parent_draw(&self) -> bool;
}

//////////////////////////////////////////////////
// GameApplicationRunner

pub trait GameApplicationRunner {
    #[cfg(target_os = "android")]
    fn loop_forever(app: AndroidApp);

    #[cfg(not(target_os = "android"))]
    fn loop_forever();
}

//////////////////////////////////////////////////
// Implementation

impl<A: GameApplication> GameApplicationWrapper<A> {
    fn new() -> Self {
        Self {
            interface: Default::default(),
            data: Default::default(),
            ctx: Default::default(),
        }
    }
}

impl<A: GameApplication> GameLoop for GameApplicationWrapper<A> {
    fn title(&self) -> &str {
        "Morph"
    }

    fn init(&mut self, game_context: GameContext) {
        Arc::get_mut(&mut self.ctx).map(|ctx| ctx.init(game_context)).expect("Context is not shared yet");

        // trigger start event
        let start_event = self.interface.start_event();
        self.interface.handle_event(&start_event);

        // update all states
        let ctx = &self.ctx;
        self.data.states.iter_mut().for_each(|state| {
            state.init(ctx.clone());
        });
    }

    fn cleanup(&mut self) {
        // update all states
        self.data.states.iter_mut().for_each(|state| {
            state.cleanup();
        });
    }

    fn input(&mut self, input_events: &[InputEvent]) {
        // update input context
        self.ctx.input().write(|ctx| ctx.update(input_events));
    }

    fn update(&mut self, elapsed_time: f32) {
        //let data = self.data();
        //println!("FPS: {}", 1.0 / elapsed_time);

        // update delayed events
        self.data.events.update_delayed(elapsed_time);

        // check state changes
        for event in self.data.events.read(&mut self.data.reader) {
            self.interface.handle_event(event);
        }

        // find all states to be updated
        let update_index = self.data.states.iter().rposition(|state| !state.parent_update());
        for state in (&mut self.data.states[update_index.unwrap_or(0)..]).iter_mut() {
            state.update(elapsed_time, &mut self.data.events);
        }
    }

    fn render(&mut self, _gl: &Gl) {
        // clear frame
        self.ctx.graphics().write(|ctx| ctx.clear());

        // find all states to be drawn
        let draw_index = self.data.states.iter().rposition(|state| !state.parent_draw());
        (&mut self.data.states[draw_index.unwrap_or(0)..]).iter_mut().for_each(|state| {
            state.draw();
        });
    }

    fn create_device(&mut self, gl: &Gl) {
        // create device context
        self.ctx.graphics().write(|ctx| ctx.create(gl));

        // load package graphics
        // TODO: WHERE TO LOAD PACKAGES????
        // if let Some(package_info) = data.resource.package_info() {
        //     data.graphics.lock().unwrap().load_package_textures(ctx, package_info);
        // }
    }

    fn destroy_device(&mut self, _gl: &Gl) {
        // unload package graphics
        // TODO: WHERE TO UNLOAD PACKAGES????
        //data.graphics.lock().unwrap().unload_package_textures();

        // destroy device context
        self.ctx.graphics().write(|ctx| ctx.destroy());
    }

    fn resize_device(&mut self, _gl: &Gl, width: u32, height: u32) {
        // resize device
        self.ctx.graphics().write(|ctx| ctx.resize(width, height));

        // update input context
        self.ctx.input().write(|ctx| ctx.change_resolution(vec2(width as f32, height as f32)));
    }
}

impl<A: GameApplication> Default for GameApplicationWrapper<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: StateEvent> Default for GameApplicationData<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: StateEvent> GameApplicationData<E> {
    pub fn new() -> Self {
        let states = Vec::new();
        let mut events = Events::new();
        let reader = events.register();
        Self { states, events, reader }
    }

    // pub fn change_state(&mut self, state: impl GameStateImpl<S> + 'static) {
    //     self.pop_state();
    //     self.push_state(state);
    // }

    // pub fn push_state(&mut self, state: impl GameStateImpl<S> + 'static) {
    //     self.states.push(Box::new(state));
    //     if let Some(state) = self.states.last_mut() {
    //         // init state
    //         state.init(&self.resource);

    //         // create state device
    //         state.create_device(&mut self.graphics);
    //     }
    // }

    // pub fn pop_state(&mut self) {
    //     if let Some(mut state) = self.states.pop() {
    //         // destroy state device
    //         state.destroy_device(&mut self.graphics);

    //         // clear state
    //         state.cleanup(&self.resource);
    //     }
    // }
}

//////////////////////////////////////////////////
// GameRunner

impl<A: GameApplication> GameApplicationRunner for A {
    #[cfg(target_os = "android")]
    fn loop_forever(app: AndroidApp) {
        GameApplicationWrapper::<A>::loop_forever(app);
    }

    #[cfg(not(target_os = "android"))]
    fn loop_forever() {
        GameApplicationWrapper::<A>::loop_forever();
    }
}
