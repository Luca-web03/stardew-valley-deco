//! Gamepad support. Platform backends fill a [`PadState`]; [`PadMapper`]
//! turns it into virtual key / mouse input for the engine, so games need
//! no gamepad-specific code.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(not(target_os = "linux"))]
mod gilrs_pad;

use engine::{InputState, Key, MouseButton, Vec2};

/// Normalized pad state: sticks and triggers in -1..1 / 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PadState {
    pub lx: f32,
    pub ly: f32,
    pub rx: f32,
    pub ry: f32,
    pub lt: f32,
    pub rt: f32,
    pub dpad_x: i8,
    pub dpad_y: i8,
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub lb: bool,
    pub rb: bool,
    pub start: bool,
    pub back: bool,
    pub connected: bool,
}

pub const DEADZONE: f32 = 0.3;
pub const CURSOR_SPEED: f32 = 14.0;

/// Virtual keys driven by the pad (and what drives them).
fn virtual_keys(s: &PadState) -> [(Key, bool); 9] {
    [
        (Key::A, s.lx < -DEADZONE || s.dpad_x < 0),
        (Key::D, s.lx > DEADZONE || s.dpad_x > 0),
        (Key::W, s.ly < -DEADZONE || s.dpad_y < 0),
        (Key::S, s.ly > DEADZONE || s.dpad_y > 0),
        (Key::X, s.a),
        (Key::C, s.x),
        (Key::Escape, s.b || s.start),
        (Key::E, s.y),
        (Key::F, s.back),
    ]
}

/// Converts pad state changes into engine input events.
#[derive(Default)]
pub struct PadMapper {
    prev: PadState,
    prev_keys: [bool; 9],
}

impl PadMapper {
    pub fn apply(&mut self, s: &PadState, input: &mut InputState, screen: (f32, f32)) {
        if !s.connected && !self.prev.connected {
            return;
        }
        for (i, (k, down)) in virtual_keys(s).iter().enumerate() {
            if *down && !self.prev_keys[i] {
                input.key_down(*k);
            } else if !*down && self.prev_keys[i] {
                input.key_up(*k);
            }
            self.prev_keys[i] = *down;
        }
        // Bumpers scroll the toolbar.
        if s.lb && !self.prev.lb {
            input.wheel = 1.0;
        }
        if s.rb && !self.prev.rb {
            input.wheel = -1.0;
        }
        // Right stick moves the cursor; triggers click.
        if s.rx.abs() > DEADZONE || s.ry.abs() > DEADZONE {
            let m = input.mouse + Vec2::new(s.rx, s.ry) * CURSOR_SPEED;
            input.mouse = Vec2::new(m.x.clamp(0.0, screen.0 - 1.0), m.y.clamp(0.0, screen.1 - 1.0));
        }
        for (now, before, b) in [(s.rt > 0.5, self.prev.rt > 0.5, MouseButton::Left), (s.lt > 0.5, self.prev.lt > 0.5, MouseButton::Right)] {
            if now && !before {
                input.mouse_down(b);
            } else if !now && before {
                input.mouse_up(b);
            }
        }
        self.prev = *s;
    }
}

/// Polls the platform for the first connected pad.
pub struct Gamepads {
    #[cfg(target_os = "linux")]
    inner: linux::LinuxPads,
    #[cfg(not(target_os = "linux"))]
    inner: gilrs_pad::GilrsPads,
    pub mapper: PadMapper,
}

impl Gamepads {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            inner: linux::LinuxPads::new(),
            #[cfg(not(target_os = "linux"))]
            inner: gilrs_pad::GilrsPads::new(),
            mapper: PadMapper::default(),
        }
    }

    pub fn poll(&mut self, input: &mut InputState, screen: (f32, f32)) {
        let s = self.inner.poll();
        self.mapper.apply(&s, input, screen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_stick_buttons_and_triggers() {
        let mut m = PadMapper::default();
        let mut i = InputState::default();
        let mut s = PadState { connected: true, lx: -0.9, a: true, rt: 1.0, rx: 1.0, ..Default::default() };
        m.apply(&s, &mut i, (1280.0, 720.0));
        assert!(i.held(Key::A));
        assert!(i.pressed(Key::X));
        assert!(i.mouse_pressed(MouseButton::Left));
        assert_eq!(i.mouse.x, CURSOR_SPEED);
        i.end_tick();
        s.lx = 0.0;
        s.a = false;
        s.rt = 0.0;
        m.apply(&s, &mut i, (1280.0, 720.0));
        assert!(!i.held(Key::A));
        assert!(i.released(Key::X));
        assert!(i.mouse_released(MouseButton::Left));
        // Disconnected pads don't touch input.
        let mut m2 = PadMapper::default();
        let mut j = InputState::default();
        m2.apply(&PadState { a: true, ..Default::default() }, &mut j, (10.0, 10.0));
        assert!(!j.any_pressed());
    }
}
