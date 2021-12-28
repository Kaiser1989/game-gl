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

impl From<glutin::dpi::PhysicalPosition<f64>> for Location {
    fn from(e: glutin::dpi::PhysicalPosition<f64>) -> Location {
        Location {
            x: e.x as f32,
            y: e.y as f32,
        }
    }
}

impl From<glutin::event::ElementState> for MouseState {
    fn from(e: glutin::event::ElementState) -> MouseState {
        match e {
            glutin::event::ElementState::Pressed => MouseState::Pressed,
            glutin::event::ElementState::Released => MouseState::Released,
        }
    }
}

impl From<glutin::event::MouseButton> for MouseButton {
    fn from(e: glutin::event::MouseButton) -> MouseButton {
        match e {
            glutin::event::MouseButton::Left => MouseButton::Left,
            glutin::event::MouseButton::Middle => MouseButton::Middle,
            glutin::event::MouseButton::Right => MouseButton::Right,
            glutin::event::MouseButton::Other(x) => MouseButton::Other(x),
        }
    }
}

impl From<glutin::event::Touch> for TouchEvent {
    fn from(e: glutin::event::Touch) -> TouchEvent {
        let glutin::event::Touch {
            phase,
            location,
            id,
            ..
        } = e;
        TouchEvent {
            state: phase.into(),
            location: location.into(),
            id,
        }
    }
}

impl From<glutin::event::TouchPhase> for TouchState {
    fn from(e: glutin::event::TouchPhase) -> TouchState {
        match e {
            glutin::event::TouchPhase::Started => TouchState::Down,
            glutin::event::TouchPhase::Ended => TouchState::Up,
            glutin::event::TouchPhase::Moved => TouchState::Move,
            glutin::event::TouchPhase::Cancelled => TouchState::Cancelled,
        }
    }
}

impl From<glutin::event::ElementState> for KeyState {
    fn from(e: glutin::event::ElementState) -> KeyState {
        match e {
            glutin::event::ElementState::Pressed => KeyState::Pressed,
            glutin::event::ElementState::Released => KeyState::Released,
        }
    }
}

impl From<glutin::event::KeyboardInput> for KeyboardEvent {
    fn from(e: glutin::event::KeyboardInput) -> KeyboardEvent {
        let glutin::event::KeyboardInput {
            virtual_keycode,
            state,
            ..
        } = e;
        KeyboardEvent {
            state: state.into(),
            key: match virtual_keycode {
                Some(glutin::event::VirtualKeyCode::Back) => Key::Back,
                _ => Key::Unknown,
            },
        }
    }
}
