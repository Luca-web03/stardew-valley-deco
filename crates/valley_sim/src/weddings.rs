//! The wedding ceremony from `Data/Weddings` (public format): an
//! `EventScript` per spouse (or `default`) played in town on the wedding
//! morning, with `[SpouseActor]` replaced by the spouse's name,
//! `[SetupContextualWeddingAttendees]` by the `Setup` of each attendee
//! whose `Condition` passes (and who is around, unless
//! `IgnoreUnlockConditions`), and `[ContextualWeddingCelebrations]` by their
//! `Celebration` commands. Afterwards the couple wakes at home. Without the
//! data a short message stands in.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::world::World;
use crate::{Dir, TilePos};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct WeddingData {
    pub event_script: BTreeMap<String, String>,
    pub attendees: BTreeMap<String, WeddingAttendee>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct WeddingAttendee {
    pub id: Option<String>,
    pub setup: String,
    pub celebration: Option<String>,
    pub condition: Option<String>,
    pub ignore_unlock_conditions: bool,
}

pub fn load(cm: &mut valley_content::ContentManager) -> Option<WeddingData> {
    cm.load_json("Data/Weddings")
}

/// Where the wedding is held.
pub const WEDDING_LOCATION: &str = "Town";

impl World {
    /// The ceremony script for `spouse`, tokens filled in.
    pub fn wedding_script(&mut self, spouse: &str) -> Option<String> {
        let data = self.weddings.clone()?;
        let raw = data.event_script.get(spouse).or_else(|| data.event_script.get("default"))?.clone();
        let mut setup = Vec::new();
        let mut celebrate = Vec::new();
        let mut rng = self.day_rng(77);
        for (name, a) in &data.attendees {
            if name == spouse {
                continue;
            }
            let npc = self.npcs.iter().find(|n| &n.name == name);
            let around = npc.is_some_and(|n| !n.location.is_empty());
            if !a.ignore_unlock_conditions && !around {
                continue;
            }
            if !self.check_query(a.condition.as_deref(), None, None, &mut rng) {
                continue;
            }
            if !a.setup.trim().is_empty() {
                setup.push(a.setup.trim().to_string());
            }
            if let Some(c) = a.celebration.as_ref().filter(|c| !c.trim().is_empty()) {
                celebrate.push(c.trim().to_string());
            }
        }
        let script = raw
            .replace("[SpouseActor]", spouse)
            .replace("[SetupContextualWeddingAttendees]", &setup.join(" "))
            .replace("[ContextualWeddingCelebrations]", &celebrate.join("/"));
        // Leftover separators from empty lists.
        let script = script.replace("//", "/").replace("  ", " ");
        Some(self.items.expand_text(&script))
    }

    /// Play the ceremony (wedding morning). True if an event started.
    pub(crate) fn play_wedding(&mut self, spouse: &str) -> bool {
        if !self.locations.contains_key(WEDDING_LOCATION) {
            return false;
        }
        let Some(script) = self.wedding_script(spouse) else { return false };
        // The ceremony runs in town; it ends back home.
        self.current = WEDDING_LOCATION.to_string();
        self.farmer.set_tile(TilePos::new(0, 0));
        self.farmer.facing = Dir::Down;
        self.start_event("Port.Wedding", &script);
        if let Some(e) = self.event.as_mut() {
            e.end = crate::events::EndAction::Bed;
            e.skippable = true;
        }
        self.event.is_some()
    }
}
