//! Original placeholder sounds, synthesized in code.
//!
//! Every cue name the game emits gets a short, distinct sound built from
//! simple oscillators and filtered noise ([`engine::synth`]), so the game is
//! audible without any game files. Unknown cues fall back to a soft click.
//! Music tracks get a generated pentatonic loop (or rain noise). None of this
//! is derived from any original audio; only the cue *names* match, so real
//! content replaces these sounds transparently.

use engine::synth::{midi_hz, Note, Synth, Wave};
use engine::SoundBuffer;

pub const RATE: u32 = 22050;

/// Cue names with a dedicated placeholder sound.
pub const KNOWN_CUES: &[&str] = &[
    "hoeHit", "clank", "slosh", "woodyStep", "wateringCan", "swordswipe", "woosh", "axchop", "treecrack", "hammer",
    "cut", "weed_cut", "passOut", "doorClose", "harvest", "Ship", "cancel", "purchase", "purchaseClick", "give_gift",
    "gulp", "eat", "dirtyHit", "coin", "crafting", "pickUpItem", "openChest", "dwop", "stoneStep", "grassyStep",
    "bigSelect", "bigDeSelect", "smallSelect", "toolSwap", "seeds", "drumkit6", "newArtifact", "stoneCrack",
    "dog_bark", "trashcan",
];

use Wave::{Noise, Saw, Sine, Square, Triangle};

fn n(w: Wave, start: f32, dur: f32, f: f32) -> Note {
    Note::new(w, start, dur, f)
}

/// White-ish noise burst; `dark` in [0, 1) darkens it.
fn noise(start: f32, dur: f32, dark: f32) -> Note {
    Note::new(Noise, start, dur, RATE as f32).lowpass(dark)
}

/// Synthesize the placeholder for `cue`, or `None` if it has no dedicated sound.
pub fn sound(cue: &str) -> Option<SoundBuffer> {
    let mut s = Synth::with_seed(RATE, seed_of(cue));
    match cue.to_ascii_lowercase().as_str() {
        "hoehit" => {
            s.add(noise(0.0, 0.09, 0.6).vol(0.7).decay(0.03));
            s.add(n(Sine, 0.0, 0.1, 140.0).glide(60.0).vol(0.6).decay(0.04));
        }
        "clank" => {
            s.add(n(Square, 0.0, 0.25, 820.0).vol(0.25).decay(0.06));
            s.add(n(Sine, 0.0, 0.3, 1230.0).vol(0.3).decay(0.08));
            s.add(noise(0.0, 0.02, 0.1).vol(0.3));
        }
        "slosh" => {
            s.add(noise(0.0, 0.3, 0.85).vol(0.9).attack(0.04).decay(0.1));
            s.add(noise(0.12, 0.25, 0.9).vol(0.7).attack(0.03).decay(0.08));
            s.add(n(Sine, 0.05, 0.15, 300.0).glide(180.0).vol(0.15).vibrato(25.0, 1.0));
        }
        "woodystep" => {
            s.add(n(Triangle, 0.0, 0.07, 260.0).glide(170.0).vol(0.7).decay(0.025));
            s.add(noise(0.0, 0.03, 0.7).vol(0.3).decay(0.01));
        }
        "wateringcan" => {
            s.add(noise(0.0, 0.5, 0.5).vol(0.5).attack(0.08).decay(0.25));
            s.add(noise(0.02, 0.45, 0.1).vol(0.12).attack(0.1).decay(0.2));
            for i in 0..5 {
                let t = 0.05 + i as f32 * 0.08;
                s.add(n(Sine, t, 0.03, 1800.0 + 300.0 * i as f32).glide(2400.0).vol(0.06).decay(0.01));
            }
        }
        "swordswipe" => {
            s.add(noise(0.0, 0.08, 0.85).vol(0.6).attack(0.03));
            s.add(noise(0.05, 0.1, 0.25).vol(0.4).decay(0.04));
            s.add(n(Sine, 0.0, 0.12, 1800.0).glide(600.0).vol(0.08));
        }
        "woosh" => {
            s.add(noise(0.0, 0.22, 0.9).vol(0.9).attack(0.08).decay(0.08));
            s.add(noise(0.06, 0.14, 0.6).vol(0.25).attack(0.03).decay(0.05));
        }
        "axchop" => {
            s.add(noise(0.0, 0.05, 0.4).vol(0.6).decay(0.015));
            s.add(n(Triangle, 0.0, 0.14, 210.0).glide(120.0).vol(0.8).decay(0.05));
        }
        "treecrack" => {
            for (i, t) in [0.0, 0.05, 0.13, 0.17, 0.26, 0.31, 0.4, 0.47].iter().enumerate() {
                s.add(noise(*t, 0.025, 0.3 + 0.05 * i as f32).vol(0.5).decay(0.008));
            }
            s.add(n(Sine, 0.3, 0.6, 90.0).glide(45.0).vol(0.8).decay(0.2));
            s.add(noise(0.3, 0.4, 0.92).vol(0.5).decay(0.15));
        }
        "hammer" | "stonecrack" => {
            s.add(n(Square, 0.0, 0.12, 600.0).glide(560.0).vol(0.25).decay(0.03));
            s.add(n(Sine, 0.0, 0.08, 1500.0).vol(0.2).decay(0.02));
            s.add(noise(0.0, 0.015, 0.2).vol(0.5));
        }
        "cut" => {
            s.add(noise(0.0, 0.06, 0.1).vol(0.5).decay(0.02));
            s.add(n(Saw, 0.0, 0.04, 2400.0).glide(1600.0).vol(0.1));
        }
        "weed_cut" => {
            s.add(noise(0.0, 0.08, 0.3).vol(0.5).decay(0.03));
            s.add(n(Triangle, 0.0, 0.06, 500.0).glide(300.0).vol(0.2).decay(0.03));
        }
        "passout" => {
            s.add(n(Sine, 0.0, 0.9, 440.0).glide(110.0).vol(0.5).decay(0.5).vibrato(5.0, 0.3));
            s.add(n(Triangle, 0.1, 0.8, 330.0).glide(82.0).vol(0.2).decay(0.4));
        }
        "doorclose" => {
            s.add(n(Sine, 0.0, 0.2, 95.0).glide(60.0).vol(0.9).decay(0.07));
            s.add(noise(0.0, 0.04, 0.7).vol(0.5).decay(0.015));
            s.add(n(Square, 0.03, 0.03, 160.0).vol(0.15));
        }
        "harvest" => {
            s.add(noise(0.0, 0.03, 0.5).vol(0.4).decay(0.01));
            s.add(n(Sine, 0.02, 0.12, 520.0).glide(880.0).vol(0.5).decay(0.06));
        }
        "ship" => {
            s.add(n(Sine, 0.0, 0.1, 120.0).glide(80.0).vol(0.6).decay(0.04));
            s.add(n(Triangle, 0.02, 0.12, midi_hz(67.0)).vol(0.35).decay(0.08));
            s.add(n(Triangle, 0.12, 0.3, midi_hz(72.0)).vol(0.35).decay(0.12));
        }
        "cancel" => {
            s.add(n(Square, 0.0, 0.07, 440.0).vol(0.2));
            s.add(n(Square, 0.08, 0.12, 330.0).vol(0.2).decay(0.06));
        }
        "purchase" => {
            for (i, m) in [84.0, 88.0, 91.0].iter().enumerate() {
                let t = i as f32 * 0.06;
                let d = if i == 2 { 0.22 } else { 0.06 };
                s.add(n(Square, t, d, midi_hz(*m)).vol(0.16).decay(0.08));
            }
        }
        "purchaseclick" => {
            s.add(n(Sine, 0.0, 0.025, 1200.0).vol(0.5).decay(0.008));
        }
        "smallselect" => {
            s.add(n(Triangle, 0.0, 0.03, 1500.0).vol(0.4).decay(0.01));
        }
        "give_gift" => {
            for (i, m) in [72.0, 76.0, 79.0, 84.0].iter().enumerate() {
                s.add(n(Triangle, i as f32 * 0.07, 0.25, midi_hz(*m)).vol(0.35).decay(0.1));
            }
        }
        "gulp" => {
            s.add(n(Sine, 0.0, 0.09, 320.0).glide(160.0).vol(0.7).decay(0.04));
            s.add(n(Sine, 0.14, 0.09, 260.0).glide(140.0).vol(0.6).decay(0.04));
        }
        "eat" => {
            for (i, t) in [0.0, 0.11, 0.21].iter().enumerate() {
                s.add(noise(*t, 0.05, 0.15 + 0.2 * i as f32).vol(0.5).decay(0.02));
            }
        }
        "dirtyhit" => {
            s.add(noise(0.0, 0.1, 0.8).vol(0.8).decay(0.04));
            s.add(n(Sine, 0.0, 0.08, 110.0).glide(70.0).vol(0.4).decay(0.03));
        }
        "coin" => {
            s.add(n(Square, 0.0, 0.07, midi_hz(83.0)).vol(0.2));
            s.add(n(Square, 0.07, 0.3, midi_hz(88.0)).vol(0.2).decay(0.12));
        }
        "crafting" => {
            s.add(n(Square, 0.0, 0.06, 700.0).vol(0.2).decay(0.02));
            s.add(n(Square, 0.12, 0.06, 760.0).vol(0.2).decay(0.02));
            s.add(n(Sine, 0.26, 0.35, midi_hz(84.0)).vol(0.35).decay(0.15));
            s.add(n(Sine, 0.26, 0.35, midi_hz(91.0)).vol(0.15).decay(0.1));
        }
        "pickupitem" => {
            s.add(n(Sine, 0.0, 0.08, 600.0).glide(1200.0).vol(0.5).decay(0.05));
        }
        "openchest" => {
            s.add(n(Saw, 0.0, 0.25, 110.0).glide(170.0).vol(0.3).vibrato(18.0, 0.5).lowpass(0.7));
            s.add(n(Sine, 0.22, 0.1, 140.0).glide(90.0).vol(0.6).decay(0.04));
        }
        "dwop" => {
            s.add(n(Sine, 0.0, 0.1, 700.0).glide(260.0).vol(0.6).decay(0.05));
        }
        "stonestep" => {
            s.add(noise(0.0, 0.03, 0.4).vol(0.4).decay(0.01));
        }
        "grassystep" => {
            s.add(noise(0.0, 0.05, 0.2).vol(0.25).decay(0.02));
        }
        "bigselect" => {
            s.add(n(Sine, 0.0, 0.06, 880.0).glide(1320.0).vol(0.4).decay(0.04));
        }
        "bigdeselect" => {
            s.add(n(Sine, 0.0, 0.06, 1320.0).glide(880.0).vol(0.4).decay(0.04));
        }
        "toolswap" => {
            s.add(n(Triangle, 0.0, 0.05, 300.0).glide(500.0).vol(0.4).decay(0.02));
        }
        "seeds" => {
            for t in [0.0, 0.04, 0.09] {
                s.add(noise(t, 0.02, 0.05).vol(0.25).decay(0.006));
            }
        }
        "drumkit6" => {
            s.add(n(Sine, 0.0, 0.15, 150.0).glide(50.0).vol(0.9).decay(0.05));
        }
        "newartifact" => {
            for (i, m) in [79.0, 83.0, 86.0, 91.0].iter().enumerate() {
                s.add(n(Sine, i as f32 * 0.09, 0.4, midi_hz(*m)).vol(0.3).decay(0.15));
            }
        }
        "dog_bark" => {
            for t in [0.0, 0.18] {
                s.add(n(Saw, t, 0.09, 420.0).glide(260.0).vol(0.35).attack(0.005).decay(0.04));
                s.add(noise(t, 0.06, 0.6).vol(0.25).decay(0.03));
            }
        }
        "trashcan" => {
            s.add(n(Square, 0.0, 0.2, 520.0).glide(470.0).vol(0.2).decay(0.07));
            s.add(noise(0.0, 0.18, 0.4).vol(0.4).decay(0.06));
            s.add(noise(0.12, 0.12, 0.5).vol(0.3).decay(0.05));
        }
        _ => return None,
    }
    Some(s.build(0.9))
}

/// Soft click used for cues with no dedicated sound.
pub fn click() -> SoundBuffer {
    let mut s = Synth::new(RATE);
    s.add(n(Sine, 0.0, 0.015, 1800.0).vol(0.25).decay(0.004));
    s.add(noise(0.0, 0.004, 0.3).vol(0.05));
    s.build(0.9)
}

/// A seamless placeholder music loop for `track`.
pub fn music(track: &str) -> SoundBuffer {
    let seed = seed_of(track);
    let lower = track.to_ascii_lowercase();
    if lower.contains("rain") {
        return rain_loop(seed);
    }
    let mut rng = seed;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng
    };
    const PENTATONIC: [f32; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];
    let beats = 16;
    let beat = 60.0 / (84.0 + (seed % 5) as f32 * 6.0);
    let len = beats as f32 * beat;
    let root = 55.0 + (seed % 7) as f32;
    let mut s = Synth::with_seed(RATE, seed);
    s.pad_to(len);
    let mut degree = 5i32;
    for b in 0..beats {
        let t = b as f32 * beat;
        // Melody: a gentle random walk over two pentatonic octaves, with rests.
        if next() % 5 != 0 {
            degree = (degree + (next() % 5) as i32 - 2).clamp(0, 9);
            let note = root + 12.0 + 12.0 * (degree / 5) as f32 + PENTATONIC[(degree % 5) as usize];
            s.add(n(Triangle, t, beat * 1.4, midi_hz(note)).vol(0.22).attack(0.01).decay(beat * 0.6));
        }
        // Bass on every bar: root, then fifth.
        if b % 4 == 0 {
            let note = root - 12.0 + if (b / 4) % 2 == 1 { 7.0 } else { 0.0 };
            s.add(n(Sine, t, beat * 3.8, midi_hz(note)).vol(0.25).attack(0.02).decay(beat * 2.0));
        }
    }
    looped(s.samples(), len, 0.35)
}

fn rain_loop(seed: u32) -> SoundBuffer {
    let len = 6.0;
    let mut s = Synth::with_seed(RATE, seed);
    s.add(noise(0.0, len + 0.5, 0.75).vol(0.5).attack(0.0));
    let mut t = 0.0f32;
    let mut k = seed;
    while t < len {
        k = k.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let f = 1800.0 + (k >> 20) as f32;
        s.add(n(Sine, t, 0.03, f).glide(f * 0.7).vol(0.12).decay(0.01));
        t += 0.05 + (k % 97) as f32 / 400.0;
    }
    looped(s.samples(), len, 0.3)
}

/// Fold everything past `len` seconds back onto the start (so notes that ring
/// over the loop point continue seamlessly) and normalize to `peak`.
fn looped(samples: &[f32], len: f32, peak: f32) -> SoundBuffer {
    let n = ((len * RATE as f32) as usize).max(1);
    let mut out = vec![0.0f32; n];
    for (i, v) in samples.iter().enumerate() {
        out[i % n] += v;
    }
    let max = out.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    if max > 0.0 {
        for v in &mut out {
            *v *= peak / max;
        }
    }
    SoundBuffer::from_f32_mono(RATE, &out)
}

fn seed_of(name: &str) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for b in name.to_ascii_lowercase().bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_cue_is_short_distinct_and_in_range() {
        let mut seen: Vec<(String, SoundBuffer)> = Vec::new();
        for cue in KNOWN_CUES {
            let b = sound(cue).unwrap_or_else(|| panic!("{cue} has no placeholder"));
            assert_eq!(b.sample_rate, RATE);
            assert!(b.frames() > 0, "{cue}");
            assert!(b.duration_secs() < 1.5, "{cue} is {}s", b.duration_secs());
            assert!(b.peak() > 0.05 && b.peak() <= 0.91, "{cue} peak {}", b.peak());
            if let Some((other, _)) = seen.iter().find(|(_, s)| s == &b) {
                panic!("{cue} sounds identical to {other}");
            }
            seen.push((cue.to_string(), b));
        }
        assert!(sound("HOEHIT").is_some(), "names are case-insensitive");
    }

    #[test]
    fn unknown_cues_fall_back_to_a_soft_click() {
        assert!(sound("someUnknownCue").is_none());
        let c = click();
        assert!(c.duration_secs() < 0.05);
        assert!(c.peak() < 0.35);
    }

    #[test]
    fn music_loops_are_seamless_and_distinct() {
        let a = music("spring1");
        let b = music("summer2");
        assert!(a.duration_secs() > 4.0 && a.duration_secs() < 20.0);
        assert_ne!(a, b);
        assert_eq!(a, music("spring1"), "deterministic");
        assert!(a.peak() <= 0.36);
        let rain = music("rain");
        assert!(rain.duration_secs() >= 5.0);
    }
}
