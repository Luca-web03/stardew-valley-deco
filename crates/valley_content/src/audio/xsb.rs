//! XACT sound bank (`.xsb`) reader, best effort.
//!
//! Only what is needed to map **cue names to wave bank entries** is read;
//! categories, RPC curves, DSP, volume and pitch variation are ignored. The
//! layout follows the publicly documented XACT3 (content version 46) format:
//!
//! ```text
//! header (138 bytes): "SDBK", content_version u16, tool_version u16, crc u16,
//!   last_modified u64, platform u8, simple_cues u16, complex_cues u16, ? u16,
//!   total_cues u16, wave_banks u8, sounds u16, cue_name_table_len u16, ? u16,
//!   simple_cue_offset u32, complex_cue_offset u32, cue_name_offset u32, ? u32,
//!   variation_table_offset u32, ? u32, wave_bank_name_offset u32,
//!   cue_hash_offset u32, cue_hash_values_offset u32, sound_offset u32, name[64]
//! wave bank names: wave_banks x char[64]
//! cue names: NUL-separated, simple cues first, then complex cues
//! simple cue: flags u8, sound_offset u32
//! complex cue: flags u8, then (flags & 4) ? {sound_offset u32, ? u32}
//!   : {variation_table_offset u32, transition_offset u32};
//!   instance_limit u8, fade_in u16, fade_out u16, instance_flags u8
//! sound: flags u8, category u16, volume u8, pitch i16, priority u8, ? u16,
//!   (flags & 1) ? clip_count u8 : {track u16, wave_bank u8},
//!   [flags & 0x0E: RPC block, u16 size incl. itself], [flags & 0x10: DSP block, same],
//!   clips: {volume u8, clip_offset u32, filter u16, filter_freq u16} x clip_count
//! clip: event_count u8, events {info u32 (type = low 5 bits), random_offset u16, data}
//! variation table: entries u16, flags u16 (type = bits 3..5), ? u32, entries
//! ```
//!
//! Every offset and count is bounds-checked; a cue that cannot be resolved
//! is kept with no waves and a warning instead of failing the whole bank.
//! Unknown clip event types end parsing of that clip (wave-playing events
//! normally come first). If a sound bank cannot be parsed at all, the
//! `Audio/cues.json` override documented in `docs/CONTENT.md` maps cues by hand.

use std::collections::HashMap;

use super::bin::{cstr, Bin};

pub const SIGNATURE: &[u8; 4] = b"SDBK";
const NO_OFFSET: u32 = 0xFFFF_FFFF;
const HEADER_LEN: usize = 138;
/// Loop count meaning "loop forever".
pub const LOOP_INFINITE: u8 = 255;

/// A wave reference: entry `track` in the sound bank's wave bank number `bank`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WaveRef {
    pub bank: u8,
    pub track: u16,
}

/// A named cue and every wave it may play (variations are alternatives).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cue {
    pub name: String,
    pub waves: Vec<WaveRef>,
    /// Some event of the cue loops forever (typical for music).
    pub looping: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SoundBank {
    pub name: String,
    pub content_version: u16,
    pub tool_version: u16,
    /// Wave bank names, indexed by [`WaveRef::bank`].
    pub wave_banks: Vec<String>,
    pub cues: Vec<Cue>,
    pub warnings: Vec<String>,
    by_name: HashMap<String, usize>,
}

#[derive(Default, Clone)]
struct Found {
    waves: Vec<WaveRef>,
    looping: bool,
}

impl Found {
    fn merge(&mut self, o: Found) {
        for w in o.waves {
            if !self.waves.contains(&w) {
                self.waves.push(w);
            }
        }
        self.looping |= o.looping;
    }
    fn wave(&mut self, bank: u8, track: u16, loops: u8) {
        let w = WaveRef { bank, track };
        if !self.waves.contains(&w) {
            self.waves.push(w);
        }
        self.looping |= loops == LOOP_INFINITE;
    }
}

struct Parser<'a> {
    data: &'a [u8],
    sounds: HashMap<u32, Found>,
    warnings: Vec<String>,
}

impl Parser<'_> {
    fn sound(&mut self, off: u32) -> Result<Found, String> {
        if let Some(f) = self.sounds.get(&off) {
            return Ok(f.clone());
        }
        let mut b = Bin::at(self.data, off as usize)?;
        let flags = b.u8()?;
        b.skip(2 + 1 + 2 + 1 + 2)?; // category, volume, pitch, priority, unknown
        let mut found = Found::default();
        if flags & 0x01 == 0 {
            let track = b.u16()?;
            let bank = b.u8()?;
            found.wave(bank, track, 0);
        } else {
            let clips = b.u8()?;
            for mask in [0x0E, 0x10] {
                if flags & mask != 0 {
                    let size = b.u16()? as usize;
                    b.skip(size.saturating_sub(2))?;
                }
            }
            for _ in 0..clips {
                b.skip(1)?; // volume
                let clip = b.u32()?;
                b.skip(4)?; // filter settings
                match self.clip(clip) {
                    Ok(f) => found.merge(f),
                    Err(e) => self.warnings.push(format!("clip at {clip:#x}: {e}")),
                }
            }
        }
        self.sounds.insert(off, found.clone());
        Ok(found)
    }

    fn clip(&mut self, off: u32) -> Result<Found, String> {
        let mut b = Bin::at(self.data, off as usize)?;
        let events = b.u8()?;
        let mut found = Found::default();
        for _ in 0..events {
            let info = b.u32()?;
            b.skip(2)?; // random offset
            match info & 0x1F {
                // Play wave.
                1 => {
                    b.skip(2)?; // unknown, flags
                    let track = b.u16()?;
                    let bank = b.u8()?;
                    let loops = b.u8()?;
                    b.skip(4)?; // pan angle, arc
                    found.wave(bank, track, loops);
                }
                // Play wave with track variation.
                3 => {
                    b.skip(2)?;
                    let loops = b.u8()?;
                    b.skip(4)?;
                    track_list(&mut b, loops, &mut found)?;
                }
                // Play wave with effect (pitch/volume/filter) variation.
                4 => {
                    b.skip(2)?;
                    let track = b.u16()?;
                    let bank = b.u8()?;
                    let loops = b.u8()?;
                    b.skip(4 + 4 + 2 + 16 + 2)?;
                    found.wave(bank, track, loops);
                }
                // Play wave with track and effect variation.
                6 => {
                    b.skip(2)?;
                    let loops = b.u8()?;
                    b.skip(4 + 4 + 2 + 16 + 2)?;
                    track_list(&mut b, loops, &mut found)?;
                }
                // Other events (stop, pitch, volume, markers) carry no waves
                // and their sizes vary; stop at the first one.
                _ => break,
            }
        }
        Ok(found)
    }

    fn variation_table(&mut self, off: u32) -> Result<Found, String> {
        let mut b = Bin::at(self.data, off as usize)?;
        let entries = b.u16()?;
        let flags = b.u16()?;
        b.skip(4)?;
        let mut found = Found::default();
        let kind = (flags >> 3) & 0x7;
        for _ in 0..entries {
            match kind {
                0 => {
                    let track = b.u16()?;
                    let bank = b.u8()?;
                    b.skip(2)?; // weight range
                    found.wave(bank, track, 0);
                }
                1 | 3 => {
                    let sound = b.u32()?;
                    b.skip(if kind == 1 { 2 } else { 12 })?;
                    match self.sound(sound) {
                        Ok(f) => found.merge(f),
                        Err(e) => self.warnings.push(format!("sound at {sound:#x}: {e}")),
                    }
                }
                4 => {
                    let track = b.u16()?;
                    let bank = b.u8()?;
                    found.wave(bank, track, 0);
                }
                k => return Err(format!("unsupported variation table type {k}")),
            }
        }
        Ok(found)
    }
}

fn track_list(b: &mut Bin, loops: u8, found: &mut Found) -> Result<(), String> {
    let n = b.u16()?;
    b.skip(1 + 2 + 4)?; // flags, variation type, unknown
    for _ in 0..n {
        let track = b.u16()?;
        let bank = b.u8()?;
        b.skip(2)?; // weight range
        found.wave(bank, track, loops);
    }
    Ok(())
}

impl SoundBank {
    pub fn parse(data: &[u8]) -> Result<SoundBank, String> {
        if data.len() < HEADER_LEN {
            return Err("file too short to be a sound bank".into());
        }
        if &data[0..4] == b"KBDS" {
            return Err("big-endian (console) sound banks are not supported".into());
        }
        if &data[0..4] != SIGNATURE {
            return Err("not an XACT sound bank (missing SDBK signature)".into());
        }
        let mut b = Bin::at(data, 4)?;
        let content_version = b.u16()?;
        let tool_version = b.u16()?;
        b.skip(2 + 8 + 1)?; // crc, last modified, platform
        let simple = b.u16()? as usize;
        let complex = b.u16()? as usize;
        b.skip(2)?;
        let total = b.u16()? as usize;
        let bank_count = b.u8()? as usize;
        b.skip(2)?; // sound count
        let name_table_len = b.u16()? as usize;
        b.skip(2)?;
        let simple_off = b.u32()?;
        let complex_off = b.u32()?;
        let names_off = b.u32()?;
        b.skip(4 + 4 + 4)?; // unknown, variation tables, unknown
        let banks_off = b.u32()?;
        b.skip(4 + 4 + 4)?; // cue hash table, hash values, sounds
        let name = b.fixed_str(64)?;

        let mut warnings = Vec::new();
        if content_version != 46 {
            warnings.push(format!("sound bank content version {content_version} (expected 46); parsing may be incomplete"));
        }
        if total != simple + complex {
            warnings.push(format!("cue count mismatch: {simple} simple + {complex} complex != {total}"));
        }

        let mut wave_banks = Vec::with_capacity(bank_count);
        let mut nb = Bin::at(data, banks_off as usize)?;
        for _ in 0..bank_count {
            wave_banks.push(nb.fixed_str(64)?);
        }

        if names_off == NO_OFFSET {
            return Err("sound bank was built without cue names".into());
        }
        let raw_names = Bin::at(data, names_off as usize)?.bytes(name_table_len)?;
        let names: Vec<String> = raw_names.split(|&c| c == 0).map(cstr).collect();
        if names.len() < simple + complex {
            return Err(format!("cue name table holds {} names for {} cues", names.len(), simple + complex));
        }

        let mut p = Parser { data, sounds: HashMap::new(), warnings: Vec::new() };
        let mut cues = Vec::with_capacity(simple + complex);
        if simple > 0 {
            let mut t = Bin::at(data, simple_off as usize)?;
            for name in names.iter().take(simple) {
                t.skip(1)?;
                let sound = t.u32()?;
                let found = p.sound(sound).unwrap_or_else(|e| {
                    p.warnings.push(format!("cue {name}: {e}"));
                    Found::default()
                });
                cues.push(Cue { name: name.clone(), waves: found.waves, looping: found.looping });
            }
        }
        if complex > 0 {
            let mut t = Bin::at(data, complex_off as usize)?;
            for name in names.iter().skip(simple).take(complex) {
                let flags = t.u8()?;
                let target = t.u32()?;
                t.skip(4)?;
                t.skip(1 + 2 + 2 + 1)?; // instance limit, fades, instance flags
                let r = if flags & 0x04 != 0 { p.sound(target) } else { p.variation_table(target) };
                let found = r.unwrap_or_else(|e| {
                    p.warnings.push(format!("cue {name}: {e}"));
                    Found::default()
                });
                cues.push(Cue { name: name.clone(), waves: found.waves, looping: found.looping });
            }
        }

        // Drop references to wave banks that do not exist.
        let mut dropped = 0;
        for c in &mut cues {
            let before = c.waves.len();
            c.waves.retain(|w| (w.bank as usize) < bank_count);
            dropped += before - c.waves.len();
        }
        if dropped > 0 {
            warnings.push(format!("{dropped} wave reference(s) point at missing wave banks"));
        }
        warnings.extend(p.warnings);
        let by_name = cues.iter().enumerate().map(|(i, c)| (c.name.to_lowercase(), i)).collect();
        Ok(SoundBank { name, content_version, tool_version, wave_banks, cues, warnings, by_name })
    }

    /// Look up a cue by name (exact first, then case-insensitive).
    pub fn cue(&self, name: &str) -> Option<&Cue> {
        self.cues.iter().find(|c| c.name == name).or_else(|| self.by_name.get(&name.to_lowercase()).map(|&i| &self.cues[i]))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Assembles a sound bank following the documented layout.
    #[derive(Default)]
    pub(crate) struct XsbBuilder {
        body: Vec<u8>,
    }

    impl XsbBuilder {
        /// Current absolute offset (the body starts right after the header).
        pub fn here(&self) -> u32 {
            (HEADER_LEN + self.body.len()) as u32
        }
        pub fn u8(&mut self, v: u8) -> &mut Self {
            self.body.push(v);
            self
        }
        pub fn u16(&mut self, v: u16) -> &mut Self {
            self.body.extend_from_slice(&v.to_le_bytes());
            self
        }
        pub fn u32(&mut self, v: u32) -> &mut Self {
            self.body.extend_from_slice(&v.to_le_bytes());
            self
        }
        pub fn zeros(&mut self, n: usize) -> &mut Self {
            self.body.extend(std::iter::repeat_n(0, n));
            self
        }
        /// Simple sound playing one wave.
        pub fn simple_sound(&mut self, bank: u8, track: u16) -> u32 {
            let at = self.here();
            self.u8(0).u16(0).u8(0).u16(0).u8(0).u16(0).u16(track).u8(bank);
            at
        }
        /// Complex sound with an RPC block and the given clips.
        pub fn complex_sound(&mut self, clips: &[u32]) -> u32 {
            let at = self.here();
            self.u8(0x01 | 0x02).u16(0).u8(0).u16(0).u8(0).u16(0).u8(clips.len() as u8);
            self.u16(5).u8(1).u16(0); // RPC block: size 5 (incl. size field)
            for &c in clips {
                self.u8(0).u32(c).u16(0).u16(0);
            }
            at
        }
        pub fn play_wave(&mut self, bank: u8, track: u16, loops: u8) -> &mut Self {
            self.u32(1).u16(0).u8(0).u8(0).u16(track).u8(bank).u8(loops).u16(0).u16(0)
        }
        pub fn play_track_variation(&mut self, tracks: &[(u8, u16)]) -> &mut Self {
            self.u32(3).u16(0).u8(0).u8(0).u8(0).u16(0).u16(0).u16(tracks.len() as u16).u8(0).u16(0).u32(0);
            for &(bank, track) in tracks {
                self.u16(track).u8(bank).u8(0).u8(100);
            }
            self
        }

        pub fn finish(&self, wave_banks: &[&str], simple: &[(&str, u32)], complex: &[(&str, u8, u32)]) -> Vec<u8> {
            let mut tail = Vec::new();
            let base = self.here() as usize;
            let banks_off = base + tail.len();
            for n in wave_banks {
                let mut b = n.as_bytes().to_vec();
                b.resize(64, 0);
                tail.extend_from_slice(&b);
            }
            let names_off = base + tail.len();
            for (n, _) in simple {
                tail.extend_from_slice(n.as_bytes());
                tail.push(0);
            }
            for (n, _, _) in complex {
                tail.extend_from_slice(n.as_bytes());
                tail.push(0);
            }
            let names_len = base + tail.len() - names_off;
            let simple_off = base + tail.len();
            for (_, s) in simple {
                tail.push(0);
                tail.extend_from_slice(&s.to_le_bytes());
            }
            let complex_off = base + tail.len();
            for (_, flags, target) in complex {
                tail.push(*flags);
                tail.extend_from_slice(&target.to_le_bytes());
                tail.extend_from_slice(&[0; 4 + 6]);
            }
            let mut h = Vec::new();
            h.extend_from_slice(SIGNATURE);
            for v in [46u16, 43, 0] {
                h.extend_from_slice(&v.to_le_bytes());
            }
            h.extend_from_slice(&[0; 8]);
            h.push(1);
            let total = (simple.len() + complex.len()) as u16;
            for v in [simple.len() as u16, complex.len() as u16, 0, total] {
                h.extend_from_slice(&v.to_le_bytes());
            }
            h.push(wave_banks.len() as u8);
            for v in [0u16, names_len as u16, 0] {
                h.extend_from_slice(&v.to_le_bytes());
            }
            for v in [simple_off as u32, complex_off as u32, names_off as u32, NO_OFFSET, NO_OFFSET, NO_OFFSET, banks_off as u32, NO_OFFSET, NO_OFFSET, HEADER_LEN as u32] {
                h.extend_from_slice(&v.to_le_bytes());
            }
            let mut name = b"Test Sounds".to_vec();
            name.resize(64, 0);
            h.extend_from_slice(&name);
            assert_eq!(h.len(), HEADER_LEN);
            h.extend_from_slice(&self.body);
            h.extend_from_slice(&tail);
            h
        }
    }

    pub(crate) fn sample_sound_bank() -> Vec<u8> {
        let mut x = XsbBuilder::default();
        let hoe = x.simple_sound(0, 3);
        let clip_music = x.here();
        x.u8(1).play_wave(1, 1, LOOP_INFINITE);
        let music = x.complex_sound(&[clip_music]);
        // A clip with a track variation, then an effect variation event, then
        // an unknown event that stops parsing before a bogus play event.
        let clip_fx = x.here();
        x.u8(4).play_track_variation(&[(0, 5), (0, 6)]);
        x.u32(4).u16(0).u8(0).u8(0).u16(7).u8(0).u8(0).zeros(4 + 4 + 2 + 16 + 2);
        x.u32(8).u16(0).zeros(16);
        x.play_wave(0, 99, 0);
        let fx = x.complex_sound(&[clip_fx]);
        let single = x.simple_sound(0, 4);
        let bad_bank = x.simple_sound(7, 1);
        let var_waves = x.here();
        x.u16(2).u16(0).u32(0).u16(0).u8(0).u8(0).u8(50).u16(2).u8(0).u8(50).u8(100);
        let var_sounds = x.here();
        x.u16(2).u16(1 << 3).u32(0).u32(single).u8(0).u8(50).u32(fx).u8(50).u8(100);
        x.finish(
            &["Wave Bank", "Wave Bank(1)"],
            &[("hoeHit", hoe), ("broken", bad_bank)],
            &[("spring1", 0x04, music), ("coin", 0x00, var_waves), ("rand", 0x00, var_sounds), ("missing", 0x04, 0xFFFF)],
        )
    }

    #[test]
    fn maps_cue_names_to_waves() {
        let sb = SoundBank::parse(&sample_sound_bank()).unwrap();
        assert_eq!(sb.name, "Test Sounds");
        assert_eq!(sb.content_version, 46);
        assert_eq!(sb.wave_banks, vec!["Wave Bank".to_string(), "Wave Bank(1)".to_string()]);
        assert_eq!(sb.cues.len(), 6);
        let w = |b: u8, t: u16| WaveRef { bank: b, track: t };

        let hoe = sb.cue("hoeHit").unwrap();
        assert_eq!(hoe.waves, vec![w(0, 3)]);
        assert!(!hoe.looping);
        assert_eq!(sb.cue("HOEHIT").unwrap().name, "hoeHit");

        let music = sb.cue("spring1").unwrap();
        assert_eq!(music.waves, vec![w(1, 1)]);
        assert!(music.looping);

        assert_eq!(sb.cue("coin").unwrap().waves, vec![w(0, 0), w(0, 2)]);
        assert_eq!(sb.cue("rand").unwrap().waves, vec![w(0, 4), w(0, 5), w(0, 6), w(0, 7)]);

        assert!(sb.cue("broken").unwrap().waves.is_empty(), "bank 7 does not exist");
        assert!(sb.cue("missing").unwrap().waves.is_empty());
        assert!(sb.warnings.iter().any(|w| w.contains("missing wave banks")), "{:?}", sb.warnings);
        assert!(sb.warnings.iter().any(|w| w.contains("missing")), "{:?}", sb.warnings);
        assert!(sb.cue("nope").is_none());
    }

    #[test]
    fn rejects_non_sound_banks() {
        assert!(SoundBank::parse(b"SDBK").is_err());
        assert!(SoundBank::parse(&[0u8; 200]).is_err());
        let mut bytes = sample_sound_bank();
        bytes.truncate(HEADER_LEN + 10);
        assert!(SoundBank::parse(&bytes).is_err());
    }
}
