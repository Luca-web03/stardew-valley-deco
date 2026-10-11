//! Which music track the game asks the engine to play.
//!
//! Rules (simple, adjustable here):
//! * title and menus before playing: [`TITLE_MUSIC`];
//! * a jukebox set in the location wins;
//! * a location's own music from `Data/Locations` (`Music` entries whose
//!   `Condition` holds, else `MusicDefault`);
//! * outdoors: rain ambience while it rains, silence at night, otherwise a
//!   seasonal track (`<season>1..3`, rotating by day);
//! * indoors without location music: keep what is playing (but do not carry
//!   the title theme into the game).
//!
//! Track names are cue names resolved by the audio backend (sound bank cue,
//! loose `Audio/<name>.ogg`, or a generated placeholder loop).

use engine::Rng;
use valley_content::data::LocationData;
use valley_sim::gsq::{self, QueryCtx};
use valley_sim::world::World;

pub const TITLE_MUSIC: &str = "MainTheme";
pub const RAIN_MUSIC: &str = "rain";

/// Desired track while playing: `Some("")` = silence, `None` = no change.
pub fn track_for_world(w: &World, current: Option<&str>) -> Option<String> {
    if let Some(t) = w.jukebox_track() {
        return Some(t);
    }
    if let Some(t) = w.location_data.get(&w.current).and_then(|d| location_music(d, w)) {
        return Some(t);
    }
    if !w.location().outdoors {
        return if current == Some(TITLE_MUSIC) { Some(String::new()) } else { None };
    }
    if w.weather.is_raining() {
        return Some(RAIN_MUSIC.into());
    }
    if w.is_night() {
        return Some(String::new());
    }
    Some(format!("{}{}", w.clock.season.key(), 1 + (w.clock.day - 1).rem_euclid(3)))
}

fn location_music(d: &LocationData, w: &World) -> Option<String> {
    let silence = |t: &str| if t.eq_ignore_ascii_case("none") || t.eq_ignore_ascii_case("silence") { String::new() } else { t.to_string() };
    if let Some(entries) = d.music.as_ref().and_then(|v| v.as_array()) {
        let mut rng = Rng::new(Rng::seed_from(&[w.seed as i64, w.clock.days_played() as i64]));
        let mut q = QueryCtx { clock: &w.clock, weather: w.weather, mail: &w.farmer.mail_received, events_seen: &w.farmer.events_seen, location: &w.current, rng: &mut rng, world: Some(w) };
        for e in entries {
            let Some(track) = e.get("Track").and_then(|t| t.as_str()) else { continue };
            if gsq::check(e.get("Condition").and_then(|c| c.as_str()), &mut q) {
                return Some(silence(track));
            }
        }
    }
    d.music_default.as_deref().filter(|s| !s.is_empty()).map(silence)
}

#[cfg(test)]
mod tests {
    use valley_content::ContentManager;
    use valley_sim::time::Weather;
    use valley_sim::world::NewGame;
    use valley_sim::TilePos;

    use super::*;

    fn world() -> World {
        let mut cm = ContentManager::dev_only();
        World::new(&mut cm, NewGame::default())
    }

    #[test]
    fn outdoor_music_follows_season_weather_and_time() {
        let mut w = world();
        w.warp_farmer("Farm", TilePos::new(20, 12), None);
        w.weather = Weather::Sun;
        w.clock.time = 900;
        w.clock.day = 1;
        assert_eq!(track_for_world(&w, None).as_deref(), Some("spring1"));
        w.clock.day = 5;
        assert_eq!(track_for_world(&w, None).as_deref(), Some("spring2"));
        w.weather = Weather::Rain;
        assert_eq!(track_for_world(&w, None).as_deref(), Some(RAIN_MUSIC));
        w.weather = Weather::Sun;
        w.clock.time = 2000;
        assert_eq!(track_for_world(&w, None).as_deref(), Some(""));
    }

    #[test]
    fn indoors_keeps_music_but_drops_the_title_theme() {
        let mut w = world();
        w.warp_farmer("FarmHouse", TilePos::new(5, 8), None);
        assert_eq!(track_for_world(&w, Some("spring1")), None);
        assert_eq!(track_for_world(&w, Some(TITLE_MUSIC)).as_deref(), Some(""));
    }

    #[test]
    fn location_data_music_wins() {
        let mut w = world();
        w.warp_farmer("Town", TilePos::new(2, 15), None);
        let d = w.location_data.get_mut("Town").unwrap();
        d.music = Some(serde_json::json!([
            { "Id": "a", "Track": "townWinter", "Condition": "SEASON winter" },
            { "Id": "b", "Track": "townTheme" }
        ]));
        assert_eq!(track_for_world(&w, None).as_deref(), Some("townTheme"));
        let d = w.location_data.get_mut("Town").unwrap();
        d.music = None;
        d.music_default = Some("none".into());
        assert_eq!(track_for_world(&w, None).as_deref(), Some(""));
    }
}
