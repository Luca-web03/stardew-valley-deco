//! Tiny offline synthesizer for procedurally generated sounds.
//!
//! A [`Synth`] accumulates [`Note`]s (an oscillator or noise source with an
//! exponential pitch glide, vibrato, a one-pole low-pass and an
//! attack/decay envelope) into a mono buffer and converts it to a
//! [`SoundBuffer`]. Games use it for original placeholder sound effects and
//! music, so they are audible without any external audio files.

use crate::mixer::SoundBuffer;

/// Oscillator shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Sine,
    Triangle,
    Square,
    Saw,
    /// Sample-and-hold noise: a new random value `freq` times per second
    /// (use a frequency at or above the sample rate for white noise).
    Noise,
}

/// One sound event. Build with [`Note::new`] and the chained setters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Note {
    pub wave: Wave,
    /// Start time in seconds.
    pub start: f32,
    /// Duration in seconds.
    pub dur: f32,
    /// Frequency (Hz) at the start and at the end (exponential glide).
    pub freq: f32,
    pub freq_end: f32,
    pub volume: f32,
    /// Linear fade-in time in seconds.
    pub attack: f32,
    /// Exponential decay time constant in seconds (0 = hold until the end).
    pub decay: f32,
    /// Vibrato `(rate Hz, depth in semitones)`.
    pub vibrato: (f32, f32),
    /// One-pole low-pass amount in `[0, 1)`: 0 = off, closer to 1 = darker.
    pub lowpass: f32,
}

impl Note {
    pub fn new(wave: Wave, start: f32, dur: f32, freq: f32) -> Self {
        Self { wave, start, dur, freq, freq_end: freq, volume: 0.5, attack: 0.002, decay: 0.0, vibrato: (0.0, 0.0), lowpass: 0.0 }
    }
    pub fn glide(mut self, to: f32) -> Self {
        self.freq_end = to;
        self
    }
    pub fn vol(mut self, v: f32) -> Self {
        self.volume = v;
        self
    }
    pub fn attack(mut self, secs: f32) -> Self {
        self.attack = secs;
        self
    }
    pub fn decay(mut self, secs: f32) -> Self {
        self.decay = secs;
        self
    }
    pub fn vibrato(mut self, rate: f32, semitones: f32) -> Self {
        self.vibrato = (rate, semitones);
        self
    }
    pub fn lowpass(mut self, amount: f32) -> Self {
        self.lowpass = amount.clamp(0.0, 0.999);
        self
    }
}

/// Frequency of a MIDI note number (69 = A4 = 440 Hz).
pub fn midi_hz(note: f32) -> f32 {
    440.0 * 2f32.powf((note - 69.0) / 12.0)
}

/// Accumulates notes into a mono `f32` buffer.
#[derive(Clone, Debug)]
pub struct Synth {
    pub rate: u32,
    buf: Vec<f32>,
    seed: u32,
}

impl Synth {
    pub fn new(rate: u32) -> Self {
        Self::with_seed(rate, 0x2545_F491)
    }

    /// Seeded noise, so the same recipe always renders identically.
    pub fn with_seed(rate: u32, seed: u32) -> Self {
        Self { rate: rate.max(1), buf: Vec::new(), seed: seed.max(1) }
    }

    fn noise(&mut self) -> f32 {
        // xorshift32
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Make the buffer at least `secs` long (silence padding).
    pub fn pad_to(&mut self, secs: f32) -> &mut Self {
        let n = (secs.max(0.0) * self.rate as f32) as usize;
        if self.buf.len() < n {
            self.buf.resize(n, 0.0);
        }
        self
    }

    pub fn add(&mut self, n: Note) -> &mut Self {
        let rate = self.rate as f32;
        let start = (n.start.max(0.0) * rate) as usize;
        let len = (n.dur.max(0.0) * rate) as usize;
        if len == 0 {
            return self;
        }
        if self.buf.len() < start + len {
            self.buf.resize(start + len, 0.0);
        }
        let ratio = if n.freq > 0.0 && n.freq_end > 0.0 { n.freq_end / n.freq } else { 1.0 };
        let release = (0.004 * rate).max(1.0);
        let mut phase = 0.0f32;
        let mut held = self.noise();
        let mut lp = 0.0f32;
        for i in 0..len {
            let t = i as f32 / rate;
            let x = i as f32 / len as f32;
            let mut f = n.freq * ratio.powf(x);
            if n.vibrato.1 != 0.0 {
                f *= 2f32.powf(n.vibrato.1 * (std::f32::consts::TAU * n.vibrato.0 * t).sin() / 12.0);
            }
            phase += f / rate;
            let wrapped = phase >= 1.0;
            phase -= phase.floor();
            let s = match n.wave {
                Wave::Sine => (std::f32::consts::TAU * phase).sin(),
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Square => {
                    if phase < 0.5 {
                        0.8
                    } else {
                        -0.8
                    }
                }
                Wave::Saw => 2.0 * phase - 1.0,
                Wave::Noise => {
                    if wrapped || f >= rate {
                        held = self.noise();
                    }
                    held
                }
            };
            let s = if n.lowpass > 0.0 {
                lp += (s - lp) * (1.0 - n.lowpass);
                lp
            } else {
                s
            };
            let mut env = if n.attack > 0.0 { (t / n.attack).min(1.0) } else { 1.0 };
            if n.decay > 0.0 {
                env *= (-t / n.decay).exp();
            }
            env *= ((len - i) as f32 / release).min(1.0);
            self.buf[start + i] += s * env * n.volume;
        }
        self
    }

    pub fn samples(&self) -> &[f32] {
        &self.buf
    }

    /// Convert to a mono buffer, scaling down if the peak exceeds `limit`.
    pub fn build(&self, limit: f32) -> SoundBuffer {
        let peak = self.buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let g = if peak > limit && peak > 0.0 { limit / peak } else { 1.0 };
        let v: Vec<f32> = self.buf.iter().map(|s| s * g).collect();
        SoundBuffer::from_f32_mono(self.rate, &v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_notes_at_the_right_place() {
        let mut s = Synth::new(1000);
        s.add(Note::new(Wave::Square, 0.5, 0.25, 100.0).vol(1.0).attack(0.0));
        let b = s.build(1.0);
        assert_eq!(b.frames(), 750);
        assert!(b.samples[..500].iter().all(|&v| v == 0));
        assert!(b.samples[500..700].iter().any(|&v| v != 0));
    }

    #[test]
    fn build_limits_peak_and_is_deterministic() {
        let mut s = Synth::new(8000);
        s.add(Note::new(Wave::Noise, 0.0, 0.1, 8000.0).vol(3.0));
        let a = s.build(0.9);
        assert!(a.peak() <= 0.901);
        let mut s2 = Synth::new(8000);
        s2.add(Note::new(Wave::Noise, 0.0, 0.1, 8000.0).vol(3.0));
        assert_eq!(a, s2.build(0.9));
    }

    #[test]
    fn midi_reference_pitch() {
        assert!((midi_hz(69.0) - 440.0).abs() < 1e-3);
        assert!((midi_hz(81.0) - 880.0).abs() < 1e-2);
    }
}
