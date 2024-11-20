//////////////////////////////////////////////////
// Using

use crate::io::{CursorEvent, InputEvent, Key, KeyState, KeyboardEvent, MouseButton, MouseEvent, MouseState, TouchEvent, TouchState};
use nalgebra_glm::*;
use std::time::Instant;

//////////////////////////////////////////////////
// Const

const CLICK_TIME: u128 = 250;
const CLICK_MOVE: f32 = 0.01;

//////////////////////////////////////////////////
// Definition

#[derive(Debug, Default)]
pub struct RawInputContext {
    cursor_location: Vec2,
    last_cursor_location: Vec2,
    pressed: bool,
    pressed_time: Option<Instant>,
    pressed_location: Option<Vec2>,
    click: bool,
    fast_click: bool,
    back: bool,
    resolution: Vec2,
}

//////////////////////////////////////////////////
// Implementation

impl RawInputContext {
    pub fn update(&mut self, input_events: &[InputEvent]) {
        // reset inputs
        self.last_cursor_location = self.cursor_location;
        self.click = false;
        self.fast_click = false;
        self.back = false;

        // process inputs
        input_events.iter().for_each(|input_event| match input_event {
            InputEvent::Cursor(CursorEvent { location }) => {
                self.cursor_location = vec2(location.x / self.resolution.x, 1.0 - location.y / self.resolution.y);
            }
            InputEvent::Mouse(MouseEvent { state, button }) => match (state, button) {
                (MouseState::Pressed, MouseButton::Left) => {
                    self.press();
                }
                (MouseState::Released, MouseButton::Left) => {
                    self.release();
                }
                _ => {}
            },
            InputEvent::Touch(TouchEvent { state, location, id: _ }) => {
                self.cursor_location = vec2(location.x / self.resolution.x, 1.0 - location.y / self.resolution.y);
                match state {
                    TouchState::Down => {
                        self.press();
                    }
                    TouchState::Up => {
                        self.release();
                    }
                    TouchState::Cancelled => {
                        self.cancel();
                    }
                    _ => {}
                }
            }
            InputEvent::Keyboard(KeyboardEvent { state, key }) => match (state, key) {
                (KeyState::Released, Key::Escape) => {
                    self.back = true;
                }
                _ => {}
            },
        });
    }

    pub fn change_resolution(&mut self, resolution: Vec2) {
        self.resolution = resolution;
    }

    //////////////////////////////////////////////////
    // Check Input functions

    pub fn back(&self) -> bool {
        self.back
    }

    pub fn click(&self) -> Option<Vec2> {
        if self.click {
            Some(self.cursor_location)
        } else {
            None
        }
    }

    pub fn fast_click(&self) -> Option<Vec2> {
        if self.fast_click {
            Some(self.cursor_location)
        } else {
            None
        }
    }

    pub fn drag(&self) -> Option<(Vec2, Vec2)> {
        // (StartPositiion, Delta)
        if let Some(pressed_location) = self.pressed_location {
            if self.cursor_location != self.last_cursor_location {
                return Some((pressed_location, self.cursor_location - self.last_cursor_location));
            }
        }
        None
    }

    //////////////////////////////////////////////////
    // Internal stuff

    fn press(&mut self) {
        self.pressed = true;
        self.pressed_time = Some(Instant::now());
        self.pressed_location = Some(self.cursor_location);
        self.fast_click = true;
    }

    fn release(&mut self) {
        self.pressed = false;
        if let (Some(pressed_location), Some(pressed_time)) = (self.pressed_location, self.pressed_time) {
            if self.valid_click_move(pressed_location) && self.valid_click_time(pressed_time) {
                self.click = true;
            }
        }
        self.pressed_time = None;
        self.pressed_location = None;
    }

    fn cancel(&mut self) {
        self.pressed = false;
        self.pressed_time = None;
        self.pressed_location = None;
    }

    fn valid_click_time(&self, pressed_time: Instant) -> bool {
        Instant::now().duration_since(pressed_time).as_millis() <= CLICK_TIME
    }

    fn valid_click_move(&self, pressed_location: Vec2) -> bool {
        distance(&pressed_location, &self.cursor_location) <= CLICK_MOVE
    }
}
