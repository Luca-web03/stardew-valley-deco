//! Software audio mixer.
//!
//! [`SoundBuffer`] holds decoded PCM. The [`Mixer`] plays any number of
//! one-shot voices (each with its own volume and pitch, resampled on the fly
//! with linear interpolation) plus one looping music channel that crossfades
//! when the track changes. It renders interleaved stereo `f32` frames at the
//! output rate and hard-clips the sum to `[-1, 1]`.
//!
//! Nothing here touches an audio device: a platform backend owns a `Mixer`
//! on its audio thread, feeds it [`MixerCmd`]s and calls [`Mixer::mix`]. That
//! keeps the mixer deterministic and testable headlessly.

use std::sync::Arc;

/// Number of interleaved channels produced by [`Mixer::mix`].
pub const MIX_CHANNELS: usize = 2;

/// Decoded PCM audio: interleaved signed 16-bit samples, mono or stereo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoundBuffer {
    pub sample_rate: u32,
    /// 1 (mono) or 2 (stereo).
    pub channels: u16,
    pub samples: Vec<i16>,
    /// Loop region in frames `(start, length)`, used when the buffer plays as
    /// looping music. `None` loops the whole buffer.
    pub loop_region: Option<(usize, usize)>,
}

impl SoundBuffer {
    /// Build from interleaved samples with any channel count. Buffers with
    /// more than two channels are folded down to stereo (front left/right
    /// plus a share of the centre channel).
    pub fn from_interleaved(sample_rate: u32, channels: u16, samples: Vec<i16>) -> Self {
        let sample_rate = sample_rate.max(1);
        match channels {
            0 | 1 => Self { sample_rate, channels: 1, samples, loop_region: None },
            2 => {
                let mut samples = samples;
                samples.truncate(samples.len() / 2 * 2);
                Self { sample_rate, channels: 2, samples, loop_region: None }
            }
            n => {
                let n = n as usize;
                let mut out = Vec::with_capacity(samples.len() / n * 2);
                for f in samples.chunks_exact(n) {
                    let c = f[2] as f32 * 0.7;
                    out.push((f[0] as f32 + c).clamp(-32768.0, 32767.0) as i16);
                    out.push((f[1] as f32 + c).clamp(-32768.0, 32767.0) as i16);
                }
                Self { sample_rate, channels: 2, samples: out, loop_region: None }
            }
        }
    }

    /// Mono buffer from `f32` samples in `[-1, 1]` (values outside are clipped).
    pub fn from_f32_mono(sample_rate: u32, samples: &[f32]) -> Self {
        let s = samples.iter().map(|&v| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16).collect();
        Self::from_interleaved(sample_rate, 1, s)
    }

    pub fn with_loop(mut self, start: usize, len: usize) -> Self {
        self.loop_region = Some((start, len));
        self
    }

    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.frames() == 0
    }

    pub fn duration_secs(&self) -> f32 {
        self.frames() as f32 / self.sample_rate as f32
    }

    /// Largest absolute sample value, in `[0, 1]`.
    pub fn peak(&self) -> f32 {
        self.samples.iter().map(|&s| (s as f32 / 32768.0).abs()).fold(0.0, f32::max)
    }

    /// `(left, right)` of frame `i` as floats in `[-1, 1)`.
    #[inline]
    fn frame(&self, i: usize) -> (f32, f32) {
        const S: f32 = 1.0 / 32768.0;
        if self.channels == 1 {
            let v = self.samples[i] as f32 * S;
            (v, v)
        } else {
            (self.samples[2 * i] as f32 * S, self.samples[2 * i + 1] as f32 * S)
        }
    }

    /// Effective loop bounds `(start, end)` in frames.
    fn loop_bounds(&self) -> (usize, usize) {
        let n = self.frames();
        match self.loop_region {
            Some((s, l)) if l > 0 && s.saturating_add(l) <= n => (s, s + l),
            _ => (0, n),
        }
    }
}

/// Commands a game-side audio system sends to the mixer on the audio thread.
#[derive(Clone, Debug)]
pub enum MixerCmd {
    Play { sound: Arc<SoundBuffer>, volume: f32, pitch: f32 },
    /// Switch the looping music track (crossfading from the current one).
    /// Sending the track that is already playing only changes its volume.
    Music { sound: Arc<SoundBuffer>, volume: f32 },
    StopMusic,
    /// Silence everything immediately.
    StopAll,
    SetMasterVolume(f32),
}

struct Voice {
    sound: Arc<SoundBuffer>,
    /// Read position in source frames.
    pos: f64,
    /// Source frames advanced per output frame (rate ratio x pitch).
    step: f64,
    gain: f32,
    target: f32,
    /// Absolute gain change per output frame while fading.
    fade_step: f32,
    looping: bool,
    /// Remove the voice once a fade to silence completes.
    stop_at_silence: bool,
    done: bool,
}

impl Voice {
    fn new(sound: Arc<SoundBuffer>, out_rate: u32, pitch: f32, gain: f32, looping: bool) -> Self {
        let step = sound.sample_rate as f64 / out_rate as f64 * pitch.clamp(0.05, 16.0) as f64;
        Self { sound, pos: 0.0, step, gain, target: gain, fade_step: 0.0, looping, stop_at_silence: false, done: false }
    }

    fn fade_to(&mut self, target: f32, frames: f32) {
        self.target = target;
        if frames <= 1.0 {
            self.gain = target;
            self.fade_step = 0.0;
        } else {
            self.fade_step = (target - self.gain).abs().max(1e-6) / frames;
        }
    }

    /// Add this voice into `out` (interleaved stereo).
    fn render(&mut self, out: &mut [f32]) {
        let frames = self.sound.frames();
        if frames == 0 {
            self.done = true;
            return;
        }
        let (loop_start, end) = if self.looping { self.sound.loop_bounds() } else { (0, frames) };
        let loop_len = (end - loop_start) as f64;
        for f in out.chunks_exact_mut(MIX_CHANNELS) {
            if self.done {
                break;
            }
            if self.gain != self.target {
                if self.gain < self.target {
                    self.gain = (self.gain + self.fade_step).min(self.target);
                } else {
                    self.gain = (self.gain - self.fade_step).max(self.target);
                }
            }
            if self.stop_at_silence && self.gain <= 0.0 {
                self.done = true;
                break;
            }
            let i = self.pos as usize;
            let frac = (self.pos - i as f64) as f32;
            let (l0, r0) = self.sound.frame(i);
            let next = if i + 1 < end {
                i + 1
            } else if self.looping {
                loop_start
            } else {
                i
            };
            let (l1, r1) = self.sound.frame(next);
            f[0] += (l0 + (l1 - l0) * frac) * self.gain;
            f[1] += (r0 + (r1 - r0) * frac) * self.gain;
            self.pos += self.step;
            if self.pos >= end as f64 {
                if self.looping && loop_len > 0.0 {
                    self.pos = loop_start as f64 + (self.pos - end as f64) % loop_len;
                } else {
                    self.done = true;
                }
            }
        }
    }
}

/// Mixes one-shot sounds and a looping music track into stereo frames.
pub struct Mixer {
    rate: u32,
    sfx: Vec<Voice>,
    /// The last entry is the current track; earlier ones are fading out.
    music: Vec<Voice>,
    pub master_volume: f32,
    /// Maximum simultaneous one-shots; the oldest is dropped beyond this.
    pub max_voices: usize,
    /// Crossfade time for music changes, in seconds (0 = cut).
    pub music_fade_secs: f32,
}

impl Mixer {
    pub fn new(output_rate: u32) -> Self {
        Self { rate: output_rate.max(1), sfx: Vec::new(), music: Vec::new(), master_volume: 1.0, max_voices: 32, music_fade_secs: 0.75 }
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate
    }

    /// Number of one-shot voices still playing.
    pub fn active_voices(&self) -> usize {
        self.sfx.len()
    }

    /// True while a music track is audible or fading.
    pub fn music_playing(&self) -> bool {
        !self.music.is_empty()
    }

    pub fn apply(&mut self, cmd: MixerCmd) {
        match cmd {
            MixerCmd::Play { sound, volume, pitch } => self.play(sound, volume, pitch),
            MixerCmd::Music { sound, volume } => self.set_music(sound, volume),
            MixerCmd::StopMusic => self.stop_music(),
            MixerCmd::StopAll => {
                self.sfx.clear();
                self.music.clear();
            }
            MixerCmd::SetMasterVolume(v) => self.master_volume = v.max(0.0),
        }
    }

    /// Start a one-shot. `pitch` is a playback-rate multiplier (2.0 = an
    /// octave up and half as long).
    pub fn play(&mut self, sound: Arc<SoundBuffer>, volume: f32, pitch: f32) {
        if sound.is_empty() || volume <= 0.0 || !pitch.is_finite() {
            return;
        }
        while self.max_voices > 0 && self.sfx.len() >= self.max_voices {
            self.sfx.remove(0);
        }
        self.sfx.push(Voice::new(sound, self.rate, pitch, volume, false));
    }

    fn fade_frames(&self) -> f32 {
        self.music_fade_secs.max(0.0) * self.rate as f32
    }

    /// Switch the looping music track, crossfading from whatever plays now.
    pub fn set_music(&mut self, sound: Arc<SoundBuffer>, volume: f32) {
        let fade = self.fade_frames();
        if let Some(cur) = self.music.last_mut() {
            if !cur.stop_at_silence && Arc::ptr_eq(&cur.sound, &sound) {
                cur.fade_to(volume.max(0.0), fade);
                return;
            }
        }
        self.stop_music();
        let mut v = Voice::new(sound, self.rate, 1.0, 0.0, true);
        v.fade_to(volume.max(0.0), fade);
        self.music.push(v);
    }

    /// Fade the music out.
    pub fn stop_music(&mut self) {
        let fade = self.fade_frames();
        for v in &mut self.music {
            v.stop_at_silence = true;
            v.fade_to(0.0, fade);
        }
    }

    /// Render interleaved stereo frames into `out` (overwriting it).
    pub fn mix(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        for v in self.sfx.iter_mut().chain(self.music.iter_mut()) {
            v.render(out);
        }
        self.sfx.retain(|v| !v.done);
        self.music.retain(|v| !v.done);
        let g = self.master_volume;
        for s in out.iter_mut() {
            *s = (*s * g).clamp(-1.0, 1.0);
        }
    }

    /// Convenience: render `frames` stereo frames into a new vector.
    pub fn render(&mut self, frames: usize) -> Vec<f32> {
        let mut v = vec![0.0; frames * MIX_CHANNELS];
        self.mix(&mut v);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dc(rate: u32, frames: usize, value: i16) -> Arc<SoundBuffer> {
        Arc::new(SoundBuffer::from_interleaved(rate, 1, vec![value; frames]))
    }

    /// Output frames rendered until the mixer has no one-shot voices left.
    fn frames_until_silent(m: &mut Mixer) -> usize {
        let mut n = 0;
        while m.active_voices() > 0 {
            m.render(1);
            n += 1;
            assert!(n < 1_000_000, "voice never finished");
        }
        n
    }

    #[test]
    fn mixes_two_sources_additively() {
        let mut m = Mixer::new(22050);
        m.play(dc(22050, 100, 8192), 1.0, 1.0); // 0.25
        m.play(dc(22050, 50, -4096), 1.0, 1.0); // -0.125
        let out = m.render(100);
        assert!((out[0] - 0.125).abs() < 1e-4, "{}", out[0]);
        assert!((out[1] - 0.125).abs() < 1e-4, "stereo duplicate of mono");
        // After the short source ends only the first remains.
        assert!((out[2 * 60] - 0.25).abs() < 1e-4, "{}", out[120]);
        assert_eq!(m.active_voices(), 0);
    }

    #[test]
    fn volume_scales_and_stereo_is_preserved() {
        let mut m = Mixer::new(8000);
        let st = Arc::new(SoundBuffer::from_interleaved(8000, 2, vec![16384, -16384, 16384, -16384]));
        m.play(st, 0.5, 1.0);
        let out = m.render(2);
        assert!((out[0] - 0.25).abs() < 1e-4);
        assert!((out[1] + 0.25).abs() < 1e-4);
    }

    #[test]
    fn pitch_resampling_changes_length() {
        for (pitch, expected) in [(1.0, 1000), (2.0, 500), (0.5, 2000), (4.0, 250)] {
            let mut m = Mixer::new(22050);
            m.play(dc(22050, 1000, 1000), 1.0, pitch);
            assert_eq!(frames_until_silent(&mut m), expected, "pitch {pitch}");
        }
        // Rate conversion: a 22.05 kHz buffer lasts twice as many frames at 44.1 kHz.
        let mut m = Mixer::new(44100);
        m.play(dc(22050, 1000, 1000), 1.0, 1.0);
        assert_eq!(frames_until_silent(&mut m), 2000);
    }

    #[test]
    fn resampling_interpolates_linearly() {
        let mut m = Mixer::new(2);
        let ramp = Arc::new(SoundBuffer::from_interleaved(1, 1, vec![0, 16384, 0]));
        m.play(ramp, 1.0, 1.0);
        let out = m.render(4);
        let left: Vec<f32> = out.chunks(2).map(|f| f[0]).collect();
        assert!((left[0] - 0.0).abs() < 1e-4);
        assert!((left[1] - 0.25).abs() < 1e-4);
        assert!((left[2] - 0.5).abs() < 1e-4);
        assert!((left[3] - 0.25).abs() < 1e-4);
    }

    #[test]
    fn output_is_clipped() {
        let mut m = Mixer::new(8000);
        m.play(dc(8000, 10, i16::MAX), 1.0, 1.0);
        m.play(dc(8000, 10, i16::MAX), 1.0, 1.0);
        m.play(dc(8000, 10, i16::MIN), 0.1, 1.0);
        let out = m.render(10);
        assert!(out.iter().all(|&s| s == 1.0), "{out:?}");
        m.play(dc(8000, 10, i16::MIN), 1.0, 1.0);
        m.play(dc(8000, 10, i16::MIN), 1.0, 1.0);
        assert!(m.render(10).iter().all(|&s| s == -1.0));
        m.master_volume = 0.25;
        m.play(dc(8000, 10, i16::MAX), 1.0, 1.0);
        m.play(dc(8000, 10, i16::MAX), 1.0, 1.0);
        assert!(m.render(10).iter().all(|&s| (s - 0.5).abs() < 1e-3));
    }

    #[test]
    fn voice_limit_drops_oldest() {
        let mut m = Mixer::new(8000);
        m.max_voices = 3;
        for _ in 0..5 {
            m.play(dc(8000, 100, 100), 1.0, 1.0);
        }
        assert_eq!(m.active_voices(), 3);
    }

    #[test]
    fn music_loops_and_crossfades() {
        let mut m = Mixer::new(1000);
        m.music_fade_secs = 0.0;
        let a = dc(1000, 10, 16384);
        m.set_music(a.clone(), 1.0);
        // Loops far past the buffer length.
        let out = m.render(100);
        assert!(out.iter().all(|&s| (s - 0.5).abs() < 1e-4));
        assert!(m.music_playing());
        // Same track again: no restart, just a volume change.
        m.set_music(a, 0.5);
        assert!((m.render(1)[0] - 0.25).abs() < 1e-4);

        m.music_fade_secs = 0.1; // 100 frames
        let b = dc(1000, 10, -16384);
        m.set_music(b, 1.0);
        let out = m.render(200);
        let mid = out[2 * 50];
        assert!(mid < 0.25 && mid > -0.5, "mid-crossfade {mid}");
        assert!((out[2 * 150] + 0.5).abs() < 1e-3, "new track at full volume");

        m.stop_music();
        m.render(200);
        assert!(!m.music_playing());
    }

    #[test]
    fn music_honours_loop_region() {
        let mut m = Mixer::new(10);
        m.music_fade_secs = 0.0;
        let s: Vec<i16> = vec![100, 200, 300, 400];
        let buf = Arc::new(SoundBuffer::from_interleaved(10, 1, s).with_loop(2, 2));
        m.set_music(buf, 1.0);
        let out: Vec<i16> = m.render(8).chunks(2).map(|f| (f[0] * 32768.0).round() as i16).collect();
        assert_eq!(out, vec![100, 200, 300, 400, 300, 400, 300, 400]);
    }

    #[test]
    fn multichannel_is_folded_to_stereo() {
        let b = SoundBuffer::from_interleaved(48000, 6, vec![1000, 2000, 100, 0, 0, 0, 1000, 2000, 100, 0, 0, 0]);
        assert_eq!(b.channels, 2);
        assert_eq!(b.frames(), 2);
        assert_eq!(&b.samples[..2], &[1070, 2070]);
    }
}
