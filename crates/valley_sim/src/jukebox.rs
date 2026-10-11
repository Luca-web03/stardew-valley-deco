//! Jukeboxes: the songs a player has heard can be picked on a mini jukebox
//! (big craftable `209` / `Dev_MiniJukebox`, sets the music of the location
//! it stands in) or a `Jukebox` tile action. `Data/JukeboxTracks` (public
//! format: `Name`, `Available`, `AlternativeTrackIds`) names tracks, hides
//! unavailable ones and folds alternative ids into one entry. "Random"
//! picks a heard track (changing each in-game hour).

use std::collections::BTreeMap;

use engine::Rng;
use serde::Deserialize;

use crate::world::{World, WorldEvent};
use crate::TilePos;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct JukeboxTrack {
    pub name: Option<String>,
    pub available: bool,
    pub alternative_track_ids: Vec<String>,
}

impl Default for JukeboxTrack {
    fn default() -> Self {
        Self { name: None, available: true, alternative_track_ids: Vec::new() }
    }
}

pub fn load(cm: &mut valley_content::ContentManager) -> BTreeMap<String, JukeboxTrack> {
    cm.load_dict("Data/JukeboxTracks")
}

/// Mini jukebox ids (original first, placeholder last).
pub const MINI_JUKEBOX: &[&str] = &["209", "Dev_MiniJukebox"];

/// Cues that aren't songs.
fn is_song(cue: &str) -> bool {
    !cue.is_empty() && !matches!(cue.to_ascii_lowercase().as_str(), "none" | "silence" | "rain" | "continue" | "jukebox_off")
}

impl World {
    /// Remember a song the player heard.
    pub fn note_song_heard(&mut self, cue: &str) {
        if is_song(cue) && !self.farmer.songs_heard.iter().any(|s| s == cue) {
            self.farmer.songs_heard.push(cue.to_string());
        }
    }

    /// The canonical track id for a cue (alternative ids fold into theirs).
    fn canonical_track(&self, cue: &str) -> String {
        self.jukebox_tracks.iter().find(|(_, t)| t.alternative_track_ids.iter().any(|a| a == cue)).map(|(id, _)| id.clone()).unwrap_or_else(|| cue.to_string())
    }

    /// Tracks the jukebox offers: (id, label), heard order.
    pub fn jukebox_list(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for cue in &self.farmer.songs_heard {
            let id = self.canonical_track(cue);
            let data = self.jukebox_tracks.get(&id);
            if data.is_some_and(|d| !d.available) || out.iter().any(|(i, _)| *i == id) {
                continue;
            }
            let label = data.and_then(|d| d.name.clone()).map(|n| self.items.expand_text(&n)).unwrap_or_else(|| id.clone());
            out.push((id, label));
        }
        out
    }

    /// Open the track list for the current location.
    pub(crate) fn open_jukebox(&mut self) -> bool {
        let mut options = vec![("jukebox:off".to_string(), "Turn off".to_string()), ("jukebox:random".to_string(), "Random".to_string())];
        options.extend(self.jukebox_list().into_iter().map(|(id, label)| (format!("jukebox:{id}"), label)));
        self.emit(WorldEvent::Choice { text: "Choose a track".into(), options });
        true
    }

    pub(crate) fn use_mini_jukebox(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).filter(|o| o.big) else { return false };
        if !MINI_JUKEBOX.contains(&o.item_id.as_str()) {
            return false;
        }
        self.open_jukebox()
    }

    /// A pick from the list.
    pub(crate) fn choose_jukebox(&mut self, id: &str) {
        let loc = self.location_mut();
        loc.state.jukebox = if id == "off" { None } else { Some(id.to_string()) };
    }

    /// What the jukebox here plays, if anything (the location needs a mini
    /// jukebox or a `Jukebox` tile).
    pub fn jukebox_track(&self) -> Option<String> {
        let loc = self.location();
        let choice = loc.state.jukebox.clone()?;
        let has_box = loc.state.objects.values().any(|o| o.big && MINI_JUKEBOX.contains(&o.item_id.as_str())) || loc.has_action("Jukebox");
        if !has_box {
            return None;
        }
        if choice == "random" {
            let list = self.jukebox_list();
            if list.is_empty() {
                return None;
            }
            let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, (self.clock.time / 100) as i64, 88]));
            return rng.pick(&list).map(|(id, _)| id.clone());
        }
        Some(choice)
    }
}
