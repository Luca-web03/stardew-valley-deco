//! Linux joystick API (`/dev/input/js*`), read without extra libraries.
//! Axis/button numbers follow the common xpad layout.

use std::fs::File;
use std::io::Read;
use std::time::{Duration, Instant};

use super::PadState;

pub struct LinuxPads {
    file: Option<File>,
    state: PadState,
    last_scan: Option<Instant>,
}

impl LinuxPads {
    pub fn new() -> Self {
        Self { file: None, state: PadState::default(), last_scan: None }
    }

    fn open() -> Option<File> {
        use std::os::unix::fs::OpenOptionsExt;
        for i in 0..4 {
            let path = format!("/dev/input/js{i}");
            // O_NONBLOCK = 0o4000 on Linux.
            if let Ok(f) = std::fs::OpenOptions::new().read(true).custom_flags(0o4000).open(&path) {
                return Some(f);
            }
        }
        None
    }

    pub fn poll(&mut self) -> PadState {
        if self.file.is_none() {
            let due = self.last_scan.map_or(true, |t| t.elapsed() > Duration::from_secs(2));
            if due {
                self.last_scan = Some(Instant::now());
                self.file = Self::open();
                self.state = PadState { connected: self.file.is_some(), ..Default::default() };
            }
            return self.state;
        }
        let mut buf = [0u8; 8];
        loop {
            let Some(f) = self.file.as_mut() else { break };
            match f.read(&mut buf) {
                Ok(8) => {
                    let value = i16::from_le_bytes([buf[4], buf[5]]);
                    let kind = buf[6] & !0x80;
                    let number = buf[7];
                    let v = value as f32 / 32767.0;
                    let s = &mut self.state;
                    match (kind, number) {
                        (1, 0) => s.a = value != 0,
                        (1, 1) => s.b = value != 0,
                        (1, 2) => s.x = value != 0,
                        (1, 3) => s.y = value != 0,
                        (1, 4) => s.lb = value != 0,
                        (1, 5) => s.rb = value != 0,
                        (1, 6) => s.back = value != 0,
                        (1, 7) => s.start = value != 0,
                        (2, 0) => s.lx = v,
                        (2, 1) => s.ly = v,
                        (2, 2) => s.lt = (v + 1.0) / 2.0,
                        (2, 3) => s.rx = v,
                        (2, 4) => s.ry = v,
                        (2, 5) => s.rt = (v + 1.0) / 2.0,
                        (2, 6) => s.dpad_x = v.signum() as i8 * (v.abs() > 0.5) as i8,
                        (2, 7) => s.dpad_y = v.signum() as i8 * (v.abs() > 0.5) as i8,
                        _ => {}
                    }
                }
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => {
                    // Unplugged.
                    self.file = None;
                    self.state = PadState::default();
                    break;
                }
            }
        }
        self.state
    }
}
