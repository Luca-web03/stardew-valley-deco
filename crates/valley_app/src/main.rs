//! valley: run the game in a window, or headless with a scripted input
//! sequence for screenshots and smoke tests.
//!
//! Usage:
//!   valley [--content <unpacked Content dir>] [--saves <dir>] [--new <name>] [--seed N] [--mute]
//!   valley --headless [--script "<steps>"] [--screenshot out.png] ...
//!   valley --audio-info | --audio-test [cues] | --export-cue <cue> <out.wav>
//!
//! The content folder can also be set with the VALLEY_CONTENT env var.

mod audio;
mod gamepad;
mod client;
mod headless;
mod window;

use std::path::PathBuf;

use valley_content::ContentManager;
use valley_game::ValleyGame;
use valley_sim::world::NewGame;

#[derive(Default, Clone)]
pub struct Args {
    pub content: Option<PathBuf>,
    /// Folder of content mods layered over the game content.
    pub mods: Option<PathBuf>,
    /// Host co-op on this address or port.
    pub host: Option<String>,
    /// Join a co-op game at this address (client mode).
    pub join: Option<String>,
    /// Player name when joining.
    pub name: Option<String>,
    pub saves: PathBuf,
    pub headless: bool,
    pub script: String,
    pub screenshot: Option<PathBuf>,
    pub new_game: Option<String>,
    pub seed: Option<u64>,
    pub width: u32,
    pub height: u32,
    /// Never open an audio device.
    pub mute: bool,
    pub audio_info: bool,
    pub audio_test: Option<String>,
    pub export_cue: Option<(String, PathBuf)>,
}

fn parse_args() -> Args {
    let mut a = Args {
        content: std::env::var_os("VALLEY_CONTENT").map(PathBuf::from),
        mods: std::env::var_os("VALLEY_MODS").map(PathBuf::from),
        saves: std::env::var_os("VALLEY_SAVES").map(PathBuf::from).unwrap_or_else(default_save_dir),
        mute: std::env::var_os("VALLEY_MUTE").is_some_and(|v| !v.is_empty() && v != "0"),
        width: 1280,
        height: 720,
        ..Default::default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--content" => a.content = it.next().map(PathBuf::from),
            "--mods" => a.mods = it.next().map(PathBuf::from),
            "--host" => a.host = it.next(),
            "--join" => a.join = it.next(),
            "--name" => a.name = it.next(),
            "--saves" => a.saves = it.next().map(PathBuf::from).unwrap_or(a.saves),
            "--headless" => a.headless = true,
            "--script" => a.script = it.next().unwrap_or_default(),
            "--screenshot" => a.screenshot = it.next().map(PathBuf::from),
            "--new" => a.new_game = it.next(),
            "--seed" => a.seed = it.next().and_then(|s| s.parse().ok()),
            "--mute" => a.mute = true,
            "--audio-info" => a.audio_info = true,
            "--audio-test" => a.audio_test = Some(it.next().filter(|s| !s.starts_with("--")).unwrap_or_default()),
            "--export-cue" => {
                if let (Some(cue), Some(out)) = (it.next(), it.next()) {
                    a.export_cue = Some((cue, PathBuf::from(out)));
                }
            }
            "--size" => {
                if let Some((w, h)) = it.next().and_then(|s| s.split_once('x').map(|(w, h)| (w.to_string(), h.to_string()))) {
                    a.width = w.parse().unwrap_or(1280);
                    a.height = h.parse().unwrap_or(720);
                }
            }
            "-h" | "--help" => {
                println!("{}", include_str!("usage.txt"));
                std::process::exit(0);
            }
            other => eprintln!("ignoring unknown argument {other}"),
        }
    }
    a
}

fn default_save_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("valley-rs").join("saves")
}

pub fn build_game(args: &Args) -> ValleyGame {
    let cm = ContentManager::with_mods(args.content.as_deref(), args.mods.as_deref());
    if let Some(dir) = &args.content {
        if !dir.exists() {
            eprintln!("warning: content folder {} does not exist; using placeholder content", dir.display());
        }
    }
    eprintln!("content: {}", cm.describe());
    for m in cm.mods() {
        eprintln!("mod: {} {} {}", m.name, m.version, if m.author.is_empty() { String::new() } else { format!("by {}", m.author) });
    }
    let mut game = ValleyGame::new(cm, args.saves.clone());
    game.new_game_seed = args.seed;
    if let Some(name) = &args.new_game {
        game.start_new_game(NewGame { farmer_name: name.clone(), seed: args.seed.unwrap_or(1), ..Default::default() });
    }
    if let Some(h) = &args.host {
        let addr = if h.contains(':') { h.clone() } else { format!("0.0.0.0:{h}") };
        match game.host(&addr) {
            Ok(port) => eprintln!("hosting co-op on port {port}"),
            Err(e) => eprintln!("could not host on {addr}: {e}"),
        }
    }
    game
}

fn main() {
    let args = parse_args();
    if args.audio_info {
        std::process::exit(audio::print_info(args.content.as_deref()));
    }
    if let Some((cue, out)) = &args.export_cue {
        std::process::exit(audio::export_cue(args.content.as_deref(), cue, out));
    }
    if let Some(cues) = &args.audio_test {
        std::process::exit(audio::play_test(args.content.as_deref(), cues));
    }
    if args.join.is_some() {
        std::process::exit(client::run(args));
    }
    if args.headless {
        std::process::exit(headless::run(&args));
    }
    window::run(args);
}
