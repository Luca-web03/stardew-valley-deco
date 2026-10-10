//! Audio playback for the desktop app.
//!
//! [`AudioSystem`] turns the game's [`SoundCmd`]s into decoded buffers (via
//! [`AudioLibrary`]) and sends them to a mixer running on the platform output
//! ([`output`]). If no output device can be opened it logs once and stays
//! silent, without decoding anything. Headless runs never create one.

#[cfg(target_os = "linux")]
mod alsa;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod cpal_out;
pub mod output;

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use engine::{MixerCmd, SoundBuffer, SoundCmd};
use valley_content::audio::{placeholder, wav, AudioLibrary};
use valley_content::ContentManager;

/// Overall gain applied to sound effects and music (leaves headroom so a
/// few overlapping effects do not clip).
const SFX_VOLUME: f32 = 0.8;
const MUSIC_VOLUME: f32 = 0.5;

pub struct AudioSystem {
    lib: AudioLibrary,
    tx: Option<Sender<MixerCmd>>,
    output: Option<output::Output>,
    music: Option<(String, Arc<SoundBuffer>)>,
    pub sfx_volume: f32,
    pub music_volume: f32,
}

impl AudioSystem {
    fn new(mut lib: AudioLibrary, tx: Option<Sender<MixerCmd>>, output: Option<output::Output>) -> Self {
        for w in lib.take_warnings() {
            eprintln!("audio: {w}");
        }
        Self { lib, tx, output, music: None, sfx_volume: SFX_VOLUME, music_volume: MUSIC_VOLUME }
    }

    /// Open the default output device; on failure, log once and run silently.
    pub fn start(lib: AudioLibrary) -> Self {
        eprintln!("audio: {}", lib.describe());
        match output::open() {
            Ok((out, tx)) => {
                eprintln!("audio: output {}", out.describe());
                Self::new(lib, Some(tx), Some(out))
            }
            Err(e) => {
                eprintln!("audio: no output device ({e}); continuing without sound");
                Self::new(lib, None, None)
            }
        }
    }

    /// No output at all (muted or headless).
    pub fn silent(lib: AudioLibrary) -> Self {
        Self::new(lib, None, None)
    }

    /// Feed a channel instead of a device (tests).
    #[cfg(test)]
    pub fn with_sender(lib: AudioLibrary, tx: Sender<MixerCmd>) -> Self {
        Self::new(lib, Some(tx), None)
    }

    pub fn is_audible(&self) -> bool {
        self.tx.is_some()
    }

    /// Handle one frame's worth of commands.
    pub fn process(&mut self, cmds: Vec<SoundCmd>) {
        if self.tx.is_none() {
            return;
        }
        // The same cue several times in one frame (e.g. a tool hitting many
        // tiles) plays once, so it does not stack up and clip.
        let mut played = HashSet::new();
        for cmd in cmds {
            match cmd {
                SoundCmd::Play { cue, volume, pitch } => {
                    if !played.insert((cue.clone(), pitch.to_bits())) {
                        continue;
                    }
                    let (sound, _) = self.lib.sfx(&cue);
                    self.send(MixerCmd::Play { sound, volume: volume * self.sfx_volume, pitch });
                }
                SoundCmd::Music { track, volume } => self.set_music(&track, volume),
                SoundCmd::StopMusic => {
                    self.music = None;
                    self.send(MixerCmd::StopMusic);
                }
            }
        }
        for w in self.lib.take_warnings() {
            eprintln!("audio: {w}");
        }
    }

    fn set_music(&mut self, track: &str, volume: f32) {
        let volume = volume * self.music_volume;
        if track.is_empty() {
            self.music = None;
            self.send(MixerCmd::StopMusic);
            return;
        }
        if let Some((name, sound)) = &self.music {
            if name.eq_ignore_ascii_case(track) {
                let sound = sound.clone();
                self.send(MixerCmd::Music { sound, volume });
                return;
            }
        }
        match self.lib.music(track) {
            Some((sound, _)) => {
                self.music = Some((track.to_string(), sound.clone()));
                self.send(MixerCmd::Music { sound, volume });
            }
            None => {
                self.music = None;
                self.send(MixerCmd::StopMusic);
            }
        }
    }

    fn send(&mut self, cmd: MixerCmd) {
        if let Some(tx) = &self.tx {
            if tx.send(cmd).is_err() {
                eprintln!("audio: output stopped; continuing without sound");
                self.tx = None;
                self.output = None;
            }
        }
    }
}

// ---------------------------------------------------------------- CLI tools

/// `--audio-info`: what audio was found and how cues resolve.
pub fn print_info(content: Option<&Path>) -> i32 {
    use std::fmt::Write as _;
    let cm = ContentManager::with_game_dir(content);
    let mut lib = AudioLibrary::open(&cm);
    let mut o = String::new();
    let _ = writeln!(o, "audio: {}", lib.describe());
    for (name, info) in lib.wave_banks() {
        let Some(info) = info else {
            let _ = writeln!(o, "  wave bank {name}: not loaded");
            continue;
        };
        let mut formats: BTreeMap<String, usize> = BTreeMap::new();
        for e in &info.entries {
            *formats.entry(e.format.describe()).or_default() += 1;
        }
        let kind = if info.is_streaming() { "streaming" } else { "in-memory" };
        let _ = writeln!(o, "  wave bank {name}: \"{}\" v{} {kind}, {} entries", info.name, info.version, info.entries.len());
        for (f, n) in formats {
            let _ = writeln!(o, "    {n:5} x {f}");
        }
    }
    match lib.sound_bank() {
        Some(sb) => {
            let mapped = sb.cues.iter().filter(|c| !c.waves.is_empty()).count();
            let _ = writeln!(o, "  sound bank \"{}\": {} cues ({mapped} mapped to waves), wave banks {:?}", sb.name, sb.cues.len(), sb.wave_banks);
        }
        None => {
            let _ = writeln!(o, "  no sound bank");
        }
    }
    if lib.override_count() > 0 {
        let _ = writeln!(o, "  {} cue overrides from Audio/cues.json", lib.override_count());
    }
    let _ = writeln!(o, "cues used by the game:");
    for cue in placeholder::KNOWN_CUES {
        let (b, origin) = lib.sfx(cue);
        let _ = writeln!(o, "  {cue:14} {:6.2}s  {origin:?}", b.duration_secs());
    }
    for w in lib.take_warnings() {
        let _ = writeln!(o, "warning: {w}");
    }
    // Ignore a closed pipe (e.g. `| head`).
    let _ = std::io::Write::write_all(&mut std::io::stdout(), o.as_bytes());
    0
}

/// `--export-cue NAME OUT.wav`: decode a cue (or music track) to a WAV file.
pub fn export_cue(content: Option<&Path>, cue: &str, out: &Path) -> i32 {
    let cm = ContentManager::with_game_dir(content);
    let mut lib = AudioLibrary::open(&cm);
    let resolved = lib.load_real(cue, false).or_else(|| if placeholder::sound(cue).is_some() { None } else { lib.music(cue) });
    let (buf, origin) = match resolved {
        Some(r) => r,
        None => lib.sfx(cue),
    };
    for w in lib.take_warnings() {
        eprintln!("warning: {w}");
    }
    match std::fs::write(out, wav::encode_pcm16(&buf)) {
        Ok(()) => {
            eprintln!("wrote {} ({:.2}s, {} Hz, {} ch, from {origin:?})", out.display(), buf.duration_secs(), buf.sample_rate, buf.channels);
            0
        }
        Err(e) => {
            eprintln!("cannot write {}: {e}", out.display());
            1
        }
    }
}

/// `--audio-test [cue,cue,...]`: play cues on the output device, one after
/// another, to check that sound works.
pub fn play_test(content: Option<&Path>, cues: &str) -> i32 {
    let cm = ContentManager::with_game_dir(content);
    let mut sys = AudioSystem::start(AudioLibrary::open(&cm));
    if !sys.is_audible() {
        return 1;
    }
    let list: Vec<String> = if cues.is_empty() { placeholder::KNOWN_CUES.iter().map(|s| s.to_string()).collect() } else { cues.split(',').map(|s| s.trim().to_string()).collect() };
    for cue in list {
        let (buf, origin) = sys.lib.sfx(&cue);
        eprintln!("{cue}: {origin:?}");
        sys.process(vec![SoundCmd::Play { cue, volume: 1.0, pitch: 1.0 }]);
        std::thread::sleep(std::time::Duration::from_secs_f32(buf.duration_secs().min(3.0) + 0.25));
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    0
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use valley_content::MemSource;

    use super::*;

    fn system() -> (AudioSystem, mpsc::Receiver<MixerCmd>) {
        // No device: commands go to a channel the test inspects.
        let lib = AudioLibrary::from_source(Arc::new(MemSource::new("empty")));
        let (tx, rx) = mpsc::channel();
        (AudioSystem::with_sender(lib, tx), rx)
    }

    #[test]
    fn plays_cues_with_volume_and_pitch() {
        let (mut sys, rx) = system();
        sys.process(vec![
            SoundCmd::Play { cue: "hoeHit".into(), volume: 0.5, pitch: 1.5 },
            SoundCmd::Play { cue: "hoeHit".into(), volume: 0.5, pitch: 1.5 },
            SoundCmd::Play { cue: "mystery".into(), volume: 1.0, pitch: 1.0 },
        ]);
        let cmds: Vec<MixerCmd> = rx.try_iter().collect();
        assert_eq!(cmds.len(), 2, "duplicate cue in one frame plays once");
        match &cmds[0] {
            MixerCmd::Play { sound, volume, pitch } => {
                assert!(!sound.is_empty());
                assert!((volume - 0.5 * SFX_VOLUME).abs() < 1e-6);
                assert_eq!(*pitch, 1.5);
            }
            c => panic!("unexpected {c:?}"),
        }
        // Next frame the same cue plays again.
        sys.process(vec![SoundCmd::Play { cue: "hoeHit".into(), volume: 1.0, pitch: 1.0 }]);
        assert_eq!(rx.try_iter().count(), 1);
    }

    #[test]
    fn music_switches_only_when_the_track_changes() {
        let (mut sys, rx) = system();
        sys.process(vec![SoundCmd::Music { track: "spring1".into(), volume: 1.0 }]);
        let first = match rx.try_recv().unwrap() {
            MixerCmd::Music { sound, .. } => sound,
            c => panic!("unexpected {c:?}"),
        };
        sys.process(vec![SoundCmd::Music { track: "spring1".into(), volume: 0.5 }]);
        match rx.try_recv().unwrap() {
            MixerCmd::Music { sound, volume } => {
                assert!(Arc::ptr_eq(&sound, &first), "same buffer, so the mixer only changes volume");
                assert!((volume - 0.5 * MUSIC_VOLUME).abs() < 1e-6);
            }
            c => panic!("unexpected {c:?}"),
        }
        sys.process(vec![SoundCmd::Music { track: String::new(), volume: 1.0 }]);
        assert!(matches!(rx.try_recv().unwrap(), MixerCmd::StopMusic));
        sys.process(vec![SoundCmd::StopMusic]);
        assert!(matches!(rx.try_recv().unwrap(), MixerCmd::StopMusic));
    }

    #[test]
    fn silent_system_ignores_commands() {
        let lib = AudioLibrary::from_source(Arc::new(MemSource::new("empty")));
        let mut sys = AudioSystem::silent(lib);
        assert!(!sys.is_audible());
        sys.process(vec![SoundCmd::Play { cue: "coin".into(), volume: 1.0, pitch: 1.0 }, SoundCmd::Music { track: "x".into(), volume: 1.0 }]);
        assert!(sys.music.is_none());
    }

    #[test]
    fn mixer_end_to_end_without_a_device() {
        // What an output backend does with the commands: mix them.
        let (mut sys, rx) = system();
        sys.process(vec![SoundCmd::Play { cue: "coin".into(), volume: 1.0, pitch: 1.0 }, SoundCmd::Music { track: "spring1".into(), volume: 1.0 }]);
        let mut mixer = engine::Mixer::new(44100);
        assert!(output::drain(&rx, &mut mixer));
        assert_eq!(mixer.active_voices(), 1);
        assert!(mixer.music_playing());
        let out = mixer.render(4410);
        assert!(out.iter().any(|&s| s.abs() > 0.01));
        assert!(out.iter().all(|&s| (-1.0..=1.0).contains(&s)));
    }
}
