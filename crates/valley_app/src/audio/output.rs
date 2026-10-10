//! Platform audio output. Each backend owns an [`engine::Mixer`] on its
//! audio thread and receives [`MixerCmd`]s through a channel.
//!
//! * Linux: ALSA, loaded at runtime ([`super::alsa`]).
//! * Windows / macOS: cpal ([`super::cpal_out`]).
//! * Elsewhere: no output (the game runs silently).

use std::sync::mpsc::{Receiver, TryRecvError};

use engine::{Mixer, MixerCmd};

#[cfg(target_os = "linux")]
pub use super::alsa::{open, Output};
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use super::cpal_out::{open, Output};

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod unsupported {
    use std::sync::mpsc::Sender;

    use engine::MixerCmd;

    pub struct Output;

    impl Output {
        pub fn describe(&self) -> &str {
            "none"
        }
    }

    pub fn open() -> Result<(Output, Sender<MixerCmd>), String> {
        Err("no audio output backend for this platform".into())
    }
}
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub use unsupported::{open, Output};

/// Apply every pending command. Returns false once the sender is gone.
pub fn drain(rx: &Receiver<MixerCmd>, mixer: &mut Mixer) -> bool {
    loop {
        match rx.try_recv() {
            Ok(cmd) => mixer.apply(cmd),
            Err(TryRecvError::Empty) => return true,
            Err(TryRecvError::Disconnected) => return false,
        }
    }
}

pub fn to_i16(s: f32) -> i16 {
    (s.clamp(-1.0, 1.0) * 32767.0) as i16
}

/// Copy interleaved stereo frames to a device buffer with `channels`
/// channels: mono devices get the average, extra channels get silence.
pub fn spread<T: Copy>(stereo: &[f32], out: &mut [T], channels: usize, conv: impl Fn(f32) -> T) {
    let ch = channels.max(1);
    for (frame, src) in out.chunks_exact_mut(ch).zip(stereo.chunks_exact(2)) {
        if ch == 1 {
            frame[0] = conv((src[0] + src[1]) * 0.5);
        } else {
            frame[0] = conv(src[0]);
            frame[1] = conv(src[1]);
            for x in &mut frame[2..] {
                *x = conv(0.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::Arc;

    use engine::SoundBuffer;

    use super::*;

    #[test]
    fn spreads_stereo_to_any_channel_count() {
        let stereo = [0.5, -0.5, 1.0, 0.0];
        let mut mono = [0.0f32; 2];
        spread(&stereo, &mut mono, 1, |s| s);
        assert_eq!(mono, [0.0, 0.5]);
        let mut quad = [9i16; 8];
        spread(&stereo, &mut quad, 4, to_i16);
        assert_eq!(quad, [16383, -16383, 0, 0, 32767, 0, 0, 0]);
    }

    #[test]
    fn drain_applies_commands_and_reports_disconnect() {
        let (tx, rx) = mpsc::channel();
        let mut m = Mixer::new(8000);
        let b = Arc::new(SoundBuffer::from_interleaved(8000, 1, vec![100; 10]));
        tx.send(MixerCmd::Play { sound: b.clone(), volume: 1.0, pitch: 1.0 }).unwrap();
        tx.send(MixerCmd::Play { sound: b, volume: 1.0, pitch: 1.0 }).unwrap();
        assert!(drain(&rx, &mut m));
        assert_eq!(m.active_voices(), 2);
        drop(tx);
        assert!(!drain(&rx, &mut m));
    }
}
