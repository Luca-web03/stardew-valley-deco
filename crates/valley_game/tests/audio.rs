//! The game module emits sound and music requests through the engine queue.

use engine::{Key, ModuleHost, SoundCmd};
use valley_content::ContentManager;
use valley_game::music::TITLE_MUSIC;
use valley_game::ValleyGame;
use valley_sim::world::NewGame;

fn music_cmds(cmds: &[SoundCmd]) -> Vec<String> {
    cmds.iter()
        .filter_map(|c| match c {
            SoundCmd::Music { track, .. } => Some(track.clone()),
            SoundCmd::StopMusic => Some(String::new()),
            _ => None,
        })
        .collect()
}

fn playing_host(tag: &str) -> ModuleHost {
    let saves = std::env::temp_dir().join(format!("valley-audio-{tag}-{}", std::process::id()));
    let mut game = ValleyGame::new(ContentManager::dev_only(), saves);
    game.start_new_game(NewGame { farmer_name: "T".into(), seed: 3, ..Default::default() });
    let mut host = ModuleHost::new(640, 360);
    host.add(Box::new(game));
    host
}

#[test]
fn title_music_is_requested_once() {
    let saves = std::env::temp_dir().join(format!("valley-audio-title-{}", std::process::id()));
    let mut host = ModuleHost::new(640, 360);
    host.add(Box::new(ValleyGame::new(ContentManager::dev_only(), saves)));
    host.tick();
    assert_eq!(music_cmds(&host.ctx.audio.drain()), vec![TITLE_MUSIC.to_string()]);
    host.tick();
    assert!(music_cmds(&host.ctx.audio.drain()).is_empty(), "not re-requested every tick");
}

#[test]
fn a_new_game_indoors_requests_no_music() {
    let mut host = playing_host("indoors");
    host.tick();
    assert!(music_cmds(&host.ctx.audio.drain()).is_empty());
}

#[test]
fn tool_use_emits_sound_cues() {
    let mut host = playing_host("tool");
    host.tick();
    host.ctx.audio.drain();
    host.ctx.input.key_down(Key::C);
    host.tick();
    host.ctx.input.key_up(Key::C);
    for _ in 0..30 {
        host.tick();
    }
    let cues: Vec<String> = host.ctx.audio.drain().into_iter().filter_map(|c| if let SoundCmd::Play { cue, .. } = c { Some(cue) } else { None }).collect();
    assert!(!cues.is_empty(), "using a tool plays at least one cue");
}
