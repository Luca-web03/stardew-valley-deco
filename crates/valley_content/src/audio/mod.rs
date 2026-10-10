//! Audio content: XACT wave/sound banks, loose audio files and original
//! synthesized placeholders, resolved by cue name.
//!
//! A cue (or music track) name resolves, first match wins, to:
//!
//! 1. a loose file `Audio/<cue>.wav` or `Audio/<cue>.ogg` in the content folder;
//! 2. an entry from `Audio/cues.json` (manual cue -> wave index mapping);
//! 3. the waves the cue plays according to `XACT/Sound Bank.xsb`;
//! 4. a wave bank entry with that name (banks built with entry names);
//! 5. a synthesized placeholder ([`placeholder`]), or a soft click.
//!
//! Wave banks are read through the [`ContentManager`]'s source with seekable
//! access, so only entry tables are loaded up front; waves are decoded on
//! first use and one-shots are cached.

pub mod adpcm;
mod bin;
pub mod placeholder;
pub mod wav;
pub mod xsb;
pub mod xwb;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engine::SoundBuffer;

use crate::manager::ContentManager;
use crate::source::{ContentSource, DirSource, LayeredSource};
pub use xsb::{Cue, SoundBank, WaveRef};
pub use xwb::{WaveBank, WaveBankInfo, WaveCodec};

pub const SOUND_BANK_PATH: &str = "XACT/Sound Bank.xsb";
/// Wave banks tried when no sound bank names them.
pub const DEFAULT_WAVE_BANKS: [&str; 2] = ["Wave Bank", "Wave Bank(1)"];
pub const LOOSE_AUDIO_DIR: &str = "Audio";
pub const OVERRIDE_PATH: &str = "Audio/cues.json";

/// Where a resolved sound came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    File(String),
    Wave { bank: String, index: usize },
    Placeholder,
    Click,
}

struct Bank {
    name: String,
    wave: Option<WaveBank>,
}

/// Resolves cue and music names to decoded audio.
pub struct AudioLibrary {
    source: Arc<dyn ContentSource>,
    banks: Vec<Bank>,
    sound_bank: Option<SoundBank>,
    overrides: HashMap<String, Vec<(usize, usize)>>,
    decoded: HashMap<(usize, usize), Option<Arc<SoundBuffer>>>,
    loose: HashMap<String, Option<(Arc<SoundBuffer>, Origin)>>,
    placeholders: HashMap<String, (Arc<SoundBuffer>, Origin)>,
    rng: u32,
    warnings: Vec<String>,
    reported: HashSet<String>,
}

impl AudioLibrary {
    /// Audio from the content manager's sources. When the user content folder
    /// is an unpacked copy (e.g. `Content (unpacked)`), the packed `Content`
    /// folder next to it is also searched, because unpackers do not copy the
    /// XACT banks.
    pub fn open(cm: &ContentManager) -> Self {
        let mut source = cm.raw_source();
        let extra: Vec<PathBuf> = cm.game_dir.as_deref().map(packed_content_dirs).unwrap_or_default();
        if !extra.is_empty() {
            let mut layered = LayeredSource::default();
            layered.push(Box::new(source));
            for d in extra {
                layered.push(Box::new(DirSource::new(d)));
            }
            source = Arc::new(layered);
        }
        Self::from_source(source)
    }

    pub fn from_source(source: Arc<dyn ContentSource>) -> Self {
        let mut lib = Self {
            source,
            banks: Vec::new(),
            sound_bank: None,
            overrides: HashMap::new(),
            decoded: HashMap::new(),
            loose: HashMap::new(),
            placeholders: HashMap::new(),
            rng: 0x9E37_79B9,
            warnings: Vec::new(),
            reported: HashSet::new(),
        };
        if let Some(bytes) = lib.source.read(SOUND_BANK_PATH) {
            match SoundBank::parse(&bytes) {
                Ok(sb) => {
                    let n = sb.warnings.len();
                    for w in sb.warnings.iter().take(8) {
                        lib.warnings.push(format!("{SOUND_BANK_PATH}: {w}"));
                    }
                    if n > 8 {
                        lib.warnings.push(format!("{SOUND_BANK_PATH}: ... and {} more warnings", n - 8));
                    }
                    lib.sound_bank = Some(sb);
                }
                Err(e) => lib.warnings.push(format!("{SOUND_BANK_PATH}: {e} (cue names unavailable; see Audio/cues.json)")),
            }
        }
        let named: Vec<String> = match &lib.sound_bank {
            Some(sb) if !sb.wave_banks.is_empty() => sb.wave_banks.clone(),
            _ => DEFAULT_WAVE_BANKS.iter().map(|s| s.to_string()).collect(),
        };
        let from_sound_bank = lib.sound_bank.is_some();
        for name in named {
            let path = format!("XACT/{name}.xwb");
            let wave = match lib.source.open(&path) {
                Some(r) => match WaveBank::open(r) {
                    Ok(w) => Some(w),
                    Err(e) => {
                        lib.warnings.push(format!("{path}: {e}"));
                        None
                    }
                },
                None => {
                    if from_sound_bank {
                        lib.warnings.push(format!("{path}: not found (referenced by the sound bank)"));
                    }
                    None
                }
            };
            lib.banks.push(Bank { name, wave });
        }
        if let Some(bytes) = lib.source.read(OVERRIDE_PATH) {
            let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
            match serde_json::from_slice::<serde_json::Value>(bytes) {
                Ok(v) => lib.load_overrides(&v),
                Err(e) => lib.warnings.push(format!("{OVERRIDE_PATH}: {e}")),
            }
        }
        lib
    }

    fn bank_index(&self, name: &str) -> Option<usize> {
        self.banks.iter().position(|b| b.name.eq_ignore_ascii_case(name) || b.wave.as_ref().is_some_and(|w| w.name().eq_ignore_ascii_case(name)))
    }

    fn load_overrides(&mut self, v: &serde_json::Value) {
        let Some(obj) = v.as_object() else {
            self.warnings.push(format!("{OVERRIDE_PATH}: expected an object of cue -> wave index"));
            return;
        };
        for (cue, spec) in obj {
            let specs: Vec<&serde_json::Value> = match spec.as_array() {
                Some(a) => a.iter().collect(),
                None => vec![spec],
            };
            let mut refs = Vec::new();
            for s in specs {
                let r = if let Some(i) = s.as_u64() {
                    Some((0, i as usize))
                } else if let Some(o) = s.as_object() {
                    let bank = match o.get("bank") {
                        None => Some(0),
                        Some(b) => b.as_u64().map(|i| i as usize).or_else(|| b.as_str().and_then(|n| self.bank_index(n))),
                    };
                    bank.zip(o.get("index").and_then(|i| i.as_u64()).map(|i| i as usize))
                } else {
                    None
                };
                match r {
                    Some(r) => refs.push(r),
                    None => self.warnings.push(format!("{OVERRIDE_PATH}: cannot use entry for cue {cue}: {s}")),
                }
            }
            if !refs.is_empty() {
                self.overrides.insert(cue.to_lowercase(), refs);
            }
        }
    }

    /// True if any wave bank from the game could be opened.
    pub fn has_game_audio(&self) -> bool {
        self.banks.iter().any(|b| b.wave.is_some())
    }

    pub fn sound_bank(&self) -> Option<&SoundBank> {
        self.sound_bank.as_ref()
    }

    /// `(name, entry table if the bank was opened)` for each wave bank slot.
    pub fn wave_banks(&self) -> Vec<(&str, Option<&WaveBankInfo>)> {
        self.banks.iter().map(|b| (b.name.as_str(), b.wave.as_ref().map(|w| &w.info))).collect()
    }

    pub fn override_count(&self) -> usize {
        self.overrides.len()
    }

    pub fn describe(&self) -> String {
        if !self.has_game_audio() {
            return "no game audio found; using built-in placeholder sounds".into();
        }
        let banks: Vec<String> = self
            .banks
            .iter()
            .map(|b| match &b.wave {
                Some(w) => format!("{} ({} waves)", b.name, w.len()),
                None => format!("{} (missing)", b.name),
            })
            .collect();
        let cues = match &self.sound_bank {
            Some(sb) => format!("{} cues", sb.cues.len()),
            None => "no sound bank".into(),
        };
        let ov = if self.overrides.is_empty() { String::new() } else { format!(", {} cue overrides", self.overrides.len()) };
        format!("wave banks: {}; {cues}{ov}", banks.join(", "))
    }

    /// Warnings gathered since the last call (each distinct message once).
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    fn warn_once(&mut self, key: String, msg: String) {
        if self.reported.insert(key) {
            self.warnings.push(msg);
        }
    }

    /// Wave entries `(bank slot, entry)` a cue can play, by priority source.
    pub fn candidates(&self, cue: &str) -> Vec<(usize, usize)> {
        if let Some(r) = self.overrides.get(&cue.to_lowercase()) {
            return r.clone();
        }
        if let Some(c) = self.sound_bank.as_ref().and_then(|sb| sb.cue(cue)) {
            if !c.waves.is_empty() {
                return c.waves.iter().map(|w| (w.bank as usize, w.track as usize)).collect();
            }
        }
        for (i, b) in self.banks.iter().enumerate() {
            if let Some(idx) = b.wave.as_ref().and_then(|w| w.find(cue)) {
                return vec![(i, idx)];
            }
        }
        Vec::new()
    }

    fn next_rand(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    fn decode_wave(&mut self, slot: usize, entry: usize, cache: bool) -> Option<Arc<SoundBuffer>> {
        if let Some(c) = self.decoded.get(&(slot, entry)) {
            return c.clone();
        }
        let bank = self.banks.get_mut(slot)?;
        let name = bank.name.clone();
        let r = match bank.wave.as_mut() {
            Some(w) => w.decode(entry),
            None => Err("wave bank not loaded".into()),
        };
        let out = match r {
            Ok(b) if !b.is_empty() => Some(Arc::new(b)),
            Ok(_) => {
                self.warn_once(format!("w{slot}:{entry}"), format!("{name} #{entry}: empty wave"));
                None
            }
            Err(e) => {
                self.warn_once(format!("w{slot}:{entry}"), format!("{name} #{entry}: {e}"));
                None
            }
        };
        if cache || out.is_none() {
            self.decoded.insert((slot, entry), out.clone());
        }
        out
    }

    fn loose_file(&mut self, cue: &str, cache: bool) -> Option<(Arc<SoundBuffer>, Origin)> {
        let key = cue.to_lowercase();
        if let Some(c) = self.loose.get(&key) {
            return c.clone();
        }
        let mut found = None;
        for ext in ["wav", "ogg"] {
            let path = format!("{LOOSE_AUDIO_DIR}/{cue}.{ext}");
            let Some(bytes) = self.source.read(&path) else { continue };
            let r = if ext == "wav" { wav::decode(&bytes) } else { wav::decode_ogg(&bytes) };
            match r {
                Ok(b) if !b.is_empty() => {
                    found = Some((Arc::new(b), Origin::File(path)));
                    break;
                }
                Ok(_) => self.warn_once(path.clone(), format!("{path}: no audio data")),
                Err(e) => self.warn_once(path.clone(), format!("{path}: {e}")),
            }
        }
        if cache || found.is_none() {
            self.loose.insert(key, found.clone());
        }
        found
    }

    /// Resolve `cue` from game content only (loose file, override, sound
    /// bank, wave names); `None` if nothing usable was found. Variations are
    /// picked at random; with `cache`, decoded waves are kept for reuse.
    pub fn load_real(&mut self, cue: &str, cache: bool) -> Option<(Arc<SoundBuffer>, Origin)> {
        if let Some(r) = self.loose_file(cue, cache) {
            return Some(r);
        }
        let cands = self.candidates(cue);
        if cands.is_empty() {
            return None;
        }
        let start = self.next_rand() as usize % cands.len();
        for k in 0..cands.len() {
            let (slot, entry) = cands[(start + k) % cands.len()];
            if let Some(b) = self.decode_wave(slot, entry, cache) {
                let bank = self.banks[slot].name.clone();
                return Some((b, Origin::Wave { bank, index: entry }));
            }
        }
        None
    }

    /// A one-shot sound for `cue`; never fails (placeholder or click fallback).
    pub fn sfx(&mut self, cue: &str) -> (Arc<SoundBuffer>, Origin) {
        if let Some(r) = self.load_real(cue, true) {
            return r;
        }
        let key = cue.to_lowercase();
        if let Some(p) = self.placeholders.get(&key) {
            return p.clone();
        }
        let p = match placeholder::sound(cue) {
            Some(b) => (Arc::new(b), Origin::Placeholder),
            None => (Arc::new(placeholder::click()), Origin::Click),
        };
        self.placeholders.insert(key, p.clone());
        p
    }

    /// A music track. Game tracks are decoded fresh (not cached); without any
    /// game audio a generated placeholder loop is returned instead.
    pub fn music(&mut self, track: &str) -> Option<(Arc<SoundBuffer>, Origin)> {
        if let Some(r) = self.load_real(track, false) {
            return Some(r);
        }
        if self.has_game_audio() {
            self.warn_once(format!("music:{track}"), format!("music track {track}: no playable audio found"));
            return None;
        }
        let key = format!("music:{}", track.to_lowercase());
        if let Some(p) = self.placeholders.get(&key) {
            return Some(p.clone());
        }
        let p = (Arc::new(placeholder::music(track)), Origin::Placeholder);
        self.placeholders.insert(key, p.clone());
        Some(p)
    }
}

/// Packed `Content` folders to search for XACT banks besides the user's
/// content folder: the folder itself if it contains `Content/XACT` (the game
/// folder was given), or a sibling `Content` (an unpacked copy was given).
fn packed_content_dirs(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if dir.join("XACT").is_dir() {
        return out;
    }
    let child = dir.join("Content");
    if child.join("XACT").is_dir() {
        out.push(child);
    }
    if let Some(parent) = dir.parent() {
        let sib = parent.join("Content");
        if sib.join("XACT").is_dir() && sib != dir {
            out.push(sib);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::MemSource;

    fn library(files: &[(&str, Vec<u8>)]) -> AudioLibrary {
        let mut m = MemSource::new("test");
        for (p, b) in files {
            m.insert(p, b.clone());
        }
        AudioLibrary::from_source(Arc::new(m))
    }

    fn game_files() -> Vec<(&'static str, Vec<u8>)> {
        let (wb0, _) = xwb::tests::sample_bank();
        let tone: Vec<i16> = (0..64).map(|i| (i * 100) as i16).collect();
        let wb1 = xwb::tests::build_xwb("Wave Bank(1)", &[xwb::tests::pcm16(44100, 1, &tone), xwb::tests::pcm16(44100, 2, &[1, 2, 3, 4])], false);
        vec![("XACT/Sound Bank.xsb", xsb::tests::sample_sound_bank()), ("XACT/Wave Bank.xwb", wb0), ("XACT/Wave Bank(1).xwb", wb1)]
    }

    #[test]
    fn without_game_audio_uses_placeholders() {
        let mut lib = library(&[]);
        assert!(!lib.has_game_audio());
        assert!(lib.describe().contains("placeholder"));
        let (b, o) = lib.sfx("hoeHit");
        assert_eq!(o, Origin::Placeholder);
        assert!(!b.is_empty());
        let (b2, _) = lib.sfx("hoeHit");
        assert!(Arc::ptr_eq(&b, &b2), "placeholders are cached");
        assert_eq!(lib.sfx("totallyUnknown").1, Origin::Click);
        let (m, o) = lib.music("spring1").unwrap();
        assert_eq!(o, Origin::Placeholder);
        assert!(m.duration_secs() > 1.0);
        assert!(lib.take_warnings().is_empty());
    }

    #[test]
    fn resolves_cues_through_sound_and_wave_banks() {
        let mut lib = library(&game_files());
        assert!(lib.has_game_audio());
        assert!(lib.describe().contains("6 cues"), "{}", lib.describe());
        // hoeHit -> bank 0 track 3, which is xWMA: undecodable, so the
        // placeholder is used and the problem reported once.
        assert_eq!(lib.sfx("hoeHit").1, Origin::Placeholder);
        lib.sfx("hoeHit");
        let w = lib.take_warnings();
        assert_eq!(w.iter().filter(|m| m.contains("xWMA")).count(), 1, "{w:?}");

        // coin -> variations bank 0 tracks 0 and 2 (PCM ramp / ADPCM sine).
        for _ in 0..10 {
            let (b, o) = lib.sfx("coin");
            match o {
                Origin::Wave { bank, index } => {
                    assert_eq!(bank, "Wave Bank");
                    assert!(index == 0 || index == 2);
                    assert!(b.frames() == 100 || b.frames() == 700);
                }
                o => panic!("unexpected origin {o:?}"),
            }
        }
        // Music lives in the second bank and is not cached.
        let (m, o) = lib.music("spring1").unwrap();
        assert_eq!(o, Origin::Wave { bank: "Wave Bank(1)".into(), index: 1 });
        assert_eq!((m.channels, m.frames()), (2, 2));
        let (m2, _) = lib.music("spring1").unwrap();
        assert!(!Arc::ptr_eq(&m, &m2));
        // Unknown music with game audio present: silence, reported.
        assert!(lib.music("noSuchTrack").is_none());
        assert!(lib.take_warnings().iter().any(|w| w.contains("noSuchTrack")));
    }

    #[test]
    fn loose_files_and_json_overrides_take_priority() {
        let mut files = game_files();
        let loose = SoundBuffer::from_interleaved(16000, 1, vec![5, 6, 7, 8, 9]);
        files.push(("Audio/coin.wav", wav::encode_pcm16(&loose)));
        let json = r#"{ "hoeHit": 0, "spring1": { "bank": "Wave Bank(1)", "index": 0 }, "Ship": [ { "bank": 0, "index": 2 } ], "bad": "x" }"#;
        files.push(("Audio/cues.json", json.as_bytes().to_vec()));
        let mut lib = library(&files);
        assert_eq!(lib.override_count(), 3);
        assert!(lib.take_warnings().iter().any(|w| w.contains("cue bad")));

        let (b, o) = lib.sfx("coin");
        assert_eq!(o, Origin::File("Audio/coin.wav".into()));
        assert_eq!(*b, loose);
        assert_eq!(lib.sfx("COIN").1, Origin::File("Audio/coin.wav".into()));

        assert_eq!(lib.sfx("hoeHit").1, Origin::Wave { bank: "Wave Bank".into(), index: 0 });
        assert_eq!(lib.sfx("ship").1, Origin::Wave { bank: "Wave Bank".into(), index: 2 });
        let (m, o) = lib.music("spring1").unwrap();
        assert_eq!(o, Origin::Wave { bank: "Wave Bank(1)".into(), index: 0 });
        assert_eq!(m.frames(), 64);
    }

    #[test]
    fn wave_bank_entry_names_resolve_without_a_sound_bank() {
        let (wb0, _) = xwb::tests::sample_bank();
        let mut lib = library(&[("XACT/Wave Bank.xwb", wb0)]);
        assert!(lib.sound_bank().is_none());
        assert_eq!(lib.sfx("ramp16").1, Origin::Wave { bank: "Wave Bank".into(), index: 0 });
        assert_eq!(lib.wave_banks().len(), 2);
        assert!(lib.wave_banks()[1].1.is_none());
        assert!(lib.take_warnings().is_empty(), "missing default banks are not an error");
    }

    #[test]
    fn reads_banks_from_disk_including_a_sibling_packed_content_folder() {
        // Layout written by unpackers: <game>/Content (packed, has XACT) next
        // to <game>/Content (unpacked) (what the user points the engine at).
        let root = std::env::temp_dir().join(format!("valley-audio-disk-{}", std::process::id()));
        let packed = root.join("Content").join("XACT");
        let unpacked = root.join("Content (unpacked)");
        std::fs::create_dir_all(&packed).unwrap();
        std::fs::create_dir_all(unpacked.join("audio")).unwrap();
        for (name, bytes) in game_files() {
            // Different case than the asset path: lookups are case-insensitive.
            let file = name.trim_start_matches("XACT/").to_lowercase();
            std::fs::write(packed.join(file), bytes).unwrap();
        }
        let loose = SoundBuffer::from_interleaved(8000, 1, vec![1, 2, 3]);
        std::fs::write(unpacked.join("audio").join("Dwop.WAV"), wav::encode_pcm16(&loose)).unwrap();

        let cm = ContentManager::with_game_dir(Some(&unpacked));
        let mut lib = AudioLibrary::open(&cm);
        assert!(lib.has_game_audio(), "{:?}", lib.take_warnings());
        assert_eq!(lib.sound_bank().map(|s| s.cues.len()), Some(6));
        let (b, o) = lib.sfx("coin");
        assert!(matches!(o, Origin::Wave { .. }));
        assert!(!b.is_empty());
        assert_eq!(*lib.sfx("dwop").0, loose);
        assert!(cm.read_raw("Audio/dwop.wav").is_some());
        assert!(cm.read_raw("XACT/Sound Bank.xsb").is_none(), "the packed folder is only searched for audio");

        // Pointing at the game folder itself also finds Content/XACT.
        let cm = ContentManager::with_game_dir(Some(&root));
        assert!(AudioLibrary::open(&cm).has_game_audio());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn broken_sound_bank_is_reported_not_fatal() {
        let (wb0, _) = xwb::tests::sample_bank();
        let mut lib = library(&[("XACT/Sound Bank.xsb", b"SDBK garbage".to_vec()), ("XACT/Wave Bank.xwb", wb0)]);
        assert!(lib.has_game_audio());
        assert!(lib.take_warnings().iter().any(|w| w.contains("Sound Bank.xsb")));
        assert_eq!(lib.sfx("hoeHit").1, Origin::Placeholder);
    }
}
