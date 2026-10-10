//! valley: run the game in a window, or headless with a scripted input
//! sequence for screenshots and smoke tests.
//!
//! Usage:
//!   valley [--content <unpacked Content dir>] [--saves <dir>] [--new <name>] [--seed N]
//!   valley --headless [--script "<steps>"] [--screenshot out.png] ...
//!
//! The content folder can also be set with the VALLEY_CONTENT env var.

mod headless;
mod window;

use std::path::PathBuf;

use valley_content::ContentManager;
use valley_game::ValleyGame;
use valley_sim::world::NewGame;

#[derive(Default, Clone)]
pub struct Args {
    pub content: Option<PathBuf>,
    pub saves: PathBuf,
    pub headless: bool,
    pub script: String,
    pub screenshot: Option<PathBuf>,
    pub new_game: Option<String>,
    pub seed: Option<u64>,
    pub width: u32,
    pub height: u32,
}

fn parse_args() -> Args {
    let mut a = Args {
        content: std::env::var_os("VALLEY_CONTENT").map(PathBuf::from),
        saves: std::env::var_os("VALLEY_SAVES").map(PathBuf::from).unwrap_or_else(default_save_dir),
        width: 1280,
        height: 720,
        ..Default::default()
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--content" => a.content = it.next().map(PathBuf::from),
            "--saves" => a.saves = it.next().map(PathBuf::from).unwrap_or(a.saves),
            "--headless" => a.headless = true,
            "--script" => a.script = it.next().unwrap_or_default(),
            "--screenshot" => a.screenshot = it.next().map(PathBuf::from),
            "--new" => a.new_game = it.next(),
            "--seed" => a.seed = it.next().and_then(|s| s.parse().ok()),
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
    let cm = ContentManager::with_game_dir(args.content.as_deref());
    if let Some(dir) = &args.content {
        if !dir.exists() {
            eprintln!("warning: content folder {} does not exist; using placeholder content", dir.display());
        }
    }
    eprintln!("content: {}", cm.describe());
    let mut game = ValleyGame::new(cm, args.saves.clone());
    game.new_game_seed = args.seed;
    if let Some(name) = &args.new_game {
        game.start_new_game(NewGame { farmer_name: name.clone(), seed: args.seed.unwrap_or(1), ..Default::default() });
    }
    game
}

fn main() {
    let args = parse_args();
    if args.headless {
        std::process::exit(headless::run(&args));
    }
    window::run(args);
}
