//////////////////////////////////////////////////
// Input

#[derive(Debug, Copy, Clone)]
pub enum InputEvent {
    Cursor(CursorEvent),
    Mouse(MouseEvent),
    Touch(TouchEvent),
    Keyboard(KeyboardEvent),
}

#[derive(Debug, Copy, Clone)]
pub struct CursorEvent {
    pub location: Location,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseEvent {
    pub state: MouseState,
    pub button: MouseButton,
}

#[derive(Debug, Copy, Clone)]
pub enum MouseState {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,
    Other(u16),
}

#[derive(Debug, Copy, Clone)]
pub struct TouchEvent {
    pub state: TouchState,
    pub location: Location,
    pub id: u64,
}

#[derive(Debug, Copy, Clone)]
pub enum TouchState {
    Down,
    Up,
    Move,
    Cancelled,
}

#[derive(Debug, Copy, Clone)]
pub struct KeyboardEvent {
    pub state: KeyState,
    pub key: Key,
}

#[derive(Debug, Copy, Clone)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Copy, Clone)]
pub enum Key {
    Back,
    Unknown,
}

#[derive(Debug, Copy, Clone)]
pub struct Location {
    pub x: f32,
    pub y: f32,
}

impl From<winit::dpi::PhysicalPosition<f64>> for Location {
    fn from(e: winit::dpi::PhysicalPosition<f64>) -> Location {
        Location { x: e.x as f32, y: e.y as f32 }
    }
}

impl From<winit::event::ElementState> for MouseState {
    fn from(e: winit::event::ElementState) -> MouseState {
        match e {
            winit::event::ElementState::Pressed => MouseState::Pressed,
            winit::event::ElementState::Released => MouseState::Released,
        }
    }
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(e: winit::event::MouseButton) -> MouseButton {
        match e {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Back => MouseButton::Back,
            winit::event::MouseButton::Forward => MouseButton::Forward,
            winit::event::MouseButton::Other(x) => MouseButton::Other(x),
        }
    }
}

impl From<winit::event::Touch> for TouchEvent {
    fn from(e: winit::event::Touch) -> TouchEvent {
        let winit::event::Touch { phase, location, id, .. } = e;
        TouchEvent {
            state: phase.into(),
            location: location.into(),
            id,
        }
    }
}

impl From<winit::event::TouchPhase> for TouchState {
    fn from(e: winit::event::TouchPhase) -> TouchState {
        match e {
            winit::event::TouchPhase::Started => TouchState::Down,
            winit::event::TouchPhase::Ended => TouchState::Up,
            winit::event::TouchPhase::Moved => TouchState::Move,
            winit::event::TouchPhase::Cancelled => TouchState::Cancelled,
        }
    }
}

impl From<winit::event::ElementState> for KeyState {
    fn from(e: winit::event::ElementState) -> KeyState {
        match e {
            winit::event::ElementState::Pressed => KeyState::Pressed,
            winit::event::ElementState::Released => KeyState::Released,
        }
    }
}

impl From<winit::event::KeyEvent> for KeyboardEvent {
    fn from(e: winit::event::KeyEvent) -> KeyboardEvent {
        let winit::event::KeyEvent { physical_key, state, .. } = e;
        KeyboardEvent {
            state: state.into(),
            key: match physical_key {
                winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Backspace) => Key::Back,
                _ => Key::Unknown,
            },
        }
    }
}
