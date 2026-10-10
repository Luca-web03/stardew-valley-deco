//! Windows / macOS gamepads via gilrs.

use gilrs::{Axis, Button, Gilrs};

use super::PadState;

pub struct GilrsPads {
    gilrs: Option<Gilrs>,
}

impl GilrsPads {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("gamepad: unavailable ({e})");
                None
            }
        };
        Self { gilrs }
    }

    pub fn poll(&mut self) -> PadState {
        let Some(g) = self.gilrs.as_mut() else { return PadState::default() };
        while g.next_event().is_some() {}
        let Some((_, pad)) = g.gamepads().next() else { return PadState::default() };
        let axis = |a: Axis| pad.axis_data(a).map(|d| d.value()).unwrap_or(0.0);
        let btn = |b: Button| pad.is_pressed(b);
        let trig = |b: Button| pad.button_data(b).map(|d| d.value()).unwrap_or(0.0);
        PadState {
            lx: axis(Axis::LeftStickX),
            ly: -axis(Axis::LeftStickY),
            rx: axis(Axis::RightStickX),
            ry: -axis(Axis::RightStickY),
            lt: trig(Button::LeftTrigger2),
            rt: trig(Button::RightTrigger2),
            dpad_x: btn(Button::DPadRight) as i8 - btn(Button::DPadLeft) as i8,
            dpad_y: btn(Button::DPadDown) as i8 - btn(Button::DPadUp) as i8,
            a: btn(Button::South),
            b: btn(Button::East),
            x: btn(Button::West),
            y: btn(Button::North),
            lb: btn(Button::LeftTrigger),
            rb: btn(Button::RightTrigger),
            start: btn(Button::Start),
            back: btn(Button::Select),
            connected: true,
        }
    }
}
