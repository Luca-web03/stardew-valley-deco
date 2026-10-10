//! Mastery and books (engine-defined; the original's perks and book powers
//! are game code, so these are this engine's own versions):
//!
//! * Mastery XP is experience earned past level 10 in each skill. Mastery
//!   levels come at fixed thresholds; each level is a point to spend at a
//!   pedestal (`Port.Mastery` tile action) on one perk per skill.
//! * Books are objects with `Port.BookXp` ("<Skill> <amount>") and/or
//!   `Port.BookPower` (a `Data/Powers` id); reading one uses it up.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::farmer::{Skill, XP_LEVELS};
use crate::world::{World, WorldEvent};

/// Mastery XP needed for each mastery level.
pub const MASTERY_LEVELS: [i32; 5] = [10_000, 25_000, 45_000, 70_000, 100_000];

pub const SKILLS: [Skill; 5] = [Skill::Farming, Skill::Fishing, Skill::Foraging, Skill::Mining, Skill::Combat];

/// What each skill's mastery perk does (shown on the pedestal).
pub fn perk_text(s: Skill) -> &'static str {
    match s {
        Skill::Farming => "Harvests sometimes yield one extra crop.",
        Skill::Fishing => "The catch bar is a little taller.",
        Skill::Foraging => "Max energy +20.",
        Skill::Mining => "Ore nodes drop one extra ore.",
        Skill::Combat => "Weapons deal 10% more damage.",
    }
}

/// Chance of the farming perk's extra crop.
pub const FARMING_PERK_CHANCE: f64 = 0.1;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PowerData {
    pub display_name: String,
    pub description: String,
}

pub fn parse_skill(s: &str) -> Option<Skill> {
    match s.to_ascii_lowercase().as_str() {
        "farming" => Some(Skill::Farming),
        "fishing" => Some(Skill::Fishing),
        "foraging" => Some(Skill::Foraging),
        "mining" => Some(Skill::Mining),
        "combat" => Some(Skill::Combat),
        _ => None,
    }
}

impl World {
    /// Experience earned past level 10 across all skills.
    pub fn mastery_xp(&self) -> i32 {
        let cap = XP_LEVELS[9];
        SKILLS.iter().map(|s| (self.farmer.skills.xp(*s) - cap).max(0)).sum()
    }

    pub fn mastery_level(&self) -> i32 {
        let xp = self.mastery_xp();
        MASTERY_LEVELS.iter().take_while(|&&t| xp >= t).count() as i32
    }

    /// Unspent mastery points.
    pub fn mastery_points(&self) -> i32 {
        self.mastery_level() - self.farmer.mastery_perks.len() as i32
    }

    pub fn has_mastery(&self, s: Skill) -> bool {
        self.farmer.mastery_perks.contains(&s)
    }

    /// Spend a point on a skill's perk.
    pub fn claim_mastery(&mut self, s: Skill) -> Result<(), String> {
        if self.has_mastery(s) {
            return Err("Already mastered.".into());
        }
        if self.mastery_points() <= 0 {
            return Err("You need more mastery experience.".into());
        }
        self.farmer.mastery_perks.push(s);
        if s == Skill::Foraging {
            self.farmer.max_stamina += 20;
        }
        self.sound("newArtifact");
        self.emit(WorldEvent::Hud(format!("{s:?} mastery: {}", perk_text(s))));
        Ok(())
    }

    /// The pedestal: offer the skills that can still be mastered.
    pub(crate) fn open_mastery(&mut self) -> bool {
        let skills: Vec<Skill> = SKILLS.iter().copied().filter(|s| !self.has_mastery(*s)).collect();
        if skills.is_empty() {
            self.emit(WorldEvent::Message("You've mastered everything.".into()));
        } else {
            self.emit(WorldEvent::MasteryMenu { points: self.mastery_points(), skills });
        }
        true
    }

    /// Read the held book. Returns true if the item was a book.
    pub(crate) fn read_book(&mut self, slot: usize, id: &str) -> bool {
        let Some(fields) = self.items.object(id).and_then(|o| o.data.custom_fields.clone()) else { return false };
        let xp = fields.get("Port.BookXp").cloned();
        let power = fields.get("Port.BookPower").cloned();
        if xp.is_none() && power.is_none() {
            return false;
        }
        let mut lines = Vec::new();
        if let Some(spec) = xp {
            let t: Vec<&str> = spec.split_whitespace().collect();
            if let (Some(s), Some(n)) = (t.first().and_then(|s| parse_skill(s)), t.get(1).and_then(|n| n.parse::<i32>().ok())) {
                if let Some(l) = self.farmer.skills.gain(s, n) {
                    self.emit(WorldEvent::LevelUp(s, l));
                }
                lines.push(format!("You feel more skilled at {s:?} (+{n} XP)."));
            }
        }
        if let Some(p) = power {
            if self.farmer.powers.contains(&p) {
                lines.push("You already know everything in this book.".into());
            } else {
                let name = self.powers.get(&p).map(|d| self.items.expand_text(&d.display_name)).filter(|n| !n.is_empty()).unwrap_or(p.clone());
                let desc = self.powers.get(&p).map(|d| self.items.expand_text(&d.description)).unwrap_or_default();
                lines.push(format!("New power: {name}. {desc}"));
                self.farmer.powers.push(p);
            }
        }
        self.farmer.inventory.reduce(slot, 1);
        self.sound("book_read");
        self.emit(WorldEvent::Message(lines.join("\n")));
        true
    }
}

pub fn load_powers(cm: &mut valley_content::ContentManager) -> BTreeMap<String, PowerData> {
    cm.load_dict("Data/Powers")
}
