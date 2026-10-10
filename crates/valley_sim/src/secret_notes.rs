//! Secret notes: `Data/SecretNotes` (id -> text). Once notes exist, note
//! scraps occasionally turn up from artifact spots and trash cans; using a
//! scrap reveals the next unread note. Drop chances are approximations.

use engine::Rng;

use crate::item::Item;
use crate::world::{World, WorldEvent};

/// Chance a dig or search also turns up a note scrap.
pub const NOTE_CHANCE: f64 = 0.08;

/// Object ids that act as note scraps (well-known, then placeholder).
const NOTE_ITEMS: &[&str] = &["79", "Dev_NoteScrap"];

impl World {
    /// The object id used for note scraps, if content defines one.
    pub fn note_item(&self) -> Option<&'static str> {
        NOTE_ITEMS.iter().copied().find(|id| self.items.objects.contains_key(*id))
    }

    /// Notes not yet read, in id order (numeric ids sort numerically).
    pub fn unread_notes(&self) -> Vec<String> {
        let mut ids: Vec<&String> = self.secret_notes.keys().filter(|k| !self.farmer.notes_read.contains(k)).collect();
        ids.sort_by_key(|k| (k.parse::<i64>().unwrap_or(i64::MAX), (*k).clone()));
        ids.into_iter().cloned().collect()
    }

    /// Maybe add a note scrap to a dig/search result.
    pub(crate) fn maybe_drop_note(&mut self, rng: &mut Rng, at: engine::Vec2) {
        let Some(id) = self.note_item() else { return };
        // Scraps already carried or lying around count against unread notes.
        let carried: i32 = self.farmer.inventory.count(id);
        let lying: i32 = self.dropped.iter().filter(|d| d.item.object_id() == Some(id)).map(|d| d.item.stack()).sum();
        if self.unread_notes().len() as i32 <= carried + lying || !rng.chance(NOTE_CHANCE) {
            return;
        }
        self.spawn_drop(Item::object(id, 1), at);
    }

    /// Use a note scrap: read the next unread note.
    pub(crate) fn read_note_scrap(&mut self, slot: usize, id: &str) -> bool {
        if self.note_item() != Some(id) {
            return false;
        }
        let Some(note) = self.unread_notes().into_iter().next() else {
            self.emit(WorldEvent::Message("The writing has faded away.".into()));
            self.farmer.inventory.reduce(slot, 1);
            return true;
        };
        self.farmer.inventory.reduce(slot, 1);
        self.farmer.notes_read.push(note.clone());
        let text = self.items.expand_text(&self.secret_notes[&note]).replace('^', "\n");
        self.sound("shwip");
        self.emit(WorldEvent::Message(text));
        true
    }
}
