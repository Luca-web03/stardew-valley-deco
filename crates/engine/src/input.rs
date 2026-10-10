use std::collections::HashSet;

use crate::math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    W, A, S, D, E, F, I, M, J, Q, R, T, X, C, Y, N, P, Tab,
    Up, Down, Left, Right,
    Escape, Enter, Space, Backspace, Delete, LeftShift, RightShift, LeftCtrl,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Minus, Equal,
    F1, F2, F3, F4, F5, F9, F10, F11, F12,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Per-tick input snapshot. Backends feed raw events through
/// [`InputState::key_down`]/[`InputState::key_up`] etc., and the host calls
/// [`InputState::end_tick`] after each update so edge-triggered sets reset.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    held: HashSet<Key>,
    pressed: HashSet<Key>,
    released: HashSet<Key>,
    mouse_held: HashSet<MouseButton>,
    mouse_pressed: HashSet<MouseButton>,
    mouse_released: HashSet<MouseButton>,
    pub mouse: Vec2,
    pub wheel: f32,
    pub text: String,
}

impl InputState {
    pub fn key_down(&mut self, k: Key) {
        if self.held.insert(k) {
            self.pressed.insert(k);
        }
    }
    pub fn key_up(&mut self, k: Key) {
        if self.held.remove(&k) {
            self.released.insert(k);
        }
    }
    pub fn mouse_down(&mut self, b: MouseButton) {
        if self.mouse_held.insert(b) {
            self.mouse_pressed.insert(b);
        }
    }
    pub fn mouse_up(&mut self, b: MouseButton) {
        if self.mouse_held.remove(&b) {
            self.mouse_released.insert(b);
        }
    }

    pub fn held(&self, k: Key) -> bool {
        self.held.contains(&k)
    }
    pub fn pressed(&self, k: Key) -> bool {
        self.pressed.contains(&k)
    }
    pub fn released(&self, k: Key) -> bool {
        self.released.contains(&k)
    }
    pub fn mouse_held(&self, b: MouseButton) -> bool {
        self.mouse_held.contains(&b)
    }
    pub fn mouse_pressed(&self, b: MouseButton) -> bool {
        self.mouse_pressed.contains(&b)
    }
    pub fn mouse_released(&self, b: MouseButton) -> bool {
        self.mouse_released.contains(&b)
    }
    pub fn any_pressed(&self) -> bool {
        !self.pressed.is_empty() || !self.mouse_pressed.is_empty()
    }

    /// Consume a key press so later handlers in the same tick don't see it.
    pub fn consume(&mut self, k: Key) {
        self.pressed.remove(&k);
    }
    pub fn consume_mouse(&mut self, b: MouseButton) {
        self.mouse_pressed.remove(&b);
    }

    pub fn end_tick(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_pressed.clear();
        self.mouse_released.clear();
        self.wheel = 0.0;
        self.text.clear();
    }

    /// Release everything (e.g. on focus loss or module switch).
    pub fn reset(&mut self) {
        *self = InputState { mouse: self.mouse, ..Default::default() };
    }
}
