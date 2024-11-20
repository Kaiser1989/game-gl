use crate::{
    context::ApplicationContext,
    game_app::{GameApplication, GameApplicationRunner, StateEvent},
};

#[derive(Debug, Default)]
pub struct TestApplicaton {
    ctx: Option<ApplicationContext>,
}

pub enum TestStateEvent {
    Init,
}

impl StateEvent for TestStateEvent {}

impl TestApplicaton {
    fn init(&mut self) {}
}

impl GameApplication for TestApplicaton {
    type StateEvent = TestStateEvent;

    fn init(&mut self, context: ApplicationContext) {
        self.ctx = Some(context);
    }

    fn title(&self) -> &str {
        todo!()
    }

    fn start_event(&self) -> Self::StateEvent {
        todo!()
    }

    fn handle_event(&mut self, event: &Self::StateEvent) {
        match event {
            TestStateEvent::Init => self.init(),
        }
    }
}

fn main() {
    TestApplicaton::loop_forever();
}
