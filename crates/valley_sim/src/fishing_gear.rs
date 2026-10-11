//! Fishing gear: bait and tackle attached to the rod, and treasure chests
//! that can surface during the catch minigame. Effect sizes are
//! approximations kept as constants.

use crate::item::{Item, ToolKind};
use crate::world::World;

/// Uses before tackle wears out.
pub const TACKLE_USES: i32 = 20;
/// Bait shortens the wait for a bite by this factor.
pub const BAIT_BITE_FACTOR: f32 = 0.5;
/// Chance a treasure chest appears in the minigame (plus daily luck).
pub const TREASURE_CHANCE: f64 = 0.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TackleEffect {
    /// Fish bite sooner.
    BiteSpeed,
    /// The catch bar is taller.
    BarSize,
    /// Progress drains slower when the fish is outside the bar.
    SlowEscape,
    /// The bar doesn't bounce off the bottom.
    NoBounce,
    /// Treasure progress doesn't drain.
    Treasure,
}

/// What the attached gear does in one fishing attempt.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Gear {
    pub bait: bool,
    pub tackle: Option<TackleEffect>,
}

/// Well-known tackle ids and their effects (approx.).
const TACKLE: &[(&str, TackleEffect)] = &[
    ("686", TackleEffect::BiteSpeed),
    ("695", TackleEffect::BarSize),
    ("694", TackleEffect::SlowEscape),
    ("692", TackleEffect::NoBounce),
    ("693", TackleEffect::Treasure),
];

/// Slots a rod of `level` has: bait from level 1, tackle from level 2.
pub fn rod_slots(level: i32) -> usize {
    match level {
        l if l >= 2 => 2,
        1 => 1,
        _ => 0,
    }
}

impl World {
    pub fn is_bait(&self, item: &Item) -> bool {
        item.object_id().is_some_and(|id| self.items.object(id).is_some_and(|o| o.data.category == -21))
            || self.items.context_tags(item).iter().any(|t| t == "category_bait")
    }

    pub fn tackle_effect(&self, item: &Item) -> Option<TackleEffect> {
        let id = item.object_id()?;
        let custom = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.Tackle")).map(|s| s.as_str());
        let from_field = match custom {
            Some("bite_speed") => Some(TackleEffect::BiteSpeed),
            Some("bar_size") => Some(TackleEffect::BarSize),
            Some("slow_escape") => Some(TackleEffect::SlowEscape),
            Some("no_bounce") => Some(TackleEffect::NoBounce),
            Some("treasure") => Some(TackleEffect::Treasure),
            _ => None,
        };
        from_field.or_else(|| TACKLE.iter().find(|(k, _)| *k == id).map(|(_, e)| *e))
    }

    /// Attach the item in inventory `item_slot` to the rod in `rod_slot`
    /// (swapping out what was there). Returns false if it doesn't fit.
    pub fn attach_to_rod(&mut self, item_slot: usize, rod_slot: usize) -> bool {
        let Some(item) = self.farmer.inventory.get(item_slot).cloned() else { return false };
        let Some(Item::Tool(rod)) = self.farmer.inventory.get(rod_slot).cloned() else { return false };
        if rod.kind != ToolKind::FishingRod {
            return false;
        }
        let which = if self.is_bait(&item) {
            0
        } else if self.tackle_effect(&item).is_some() {
            1
        } else {
            return false;
        };
        if which >= rod_slots(rod.level) {
            self.hud("This rod can't hold that.");
            return false;
        }
        let mut rod = rod;
        rod.attachments.resize(2, None);
        // Bait stacks onto the same bait; tackle takes one piece.
        let mut put = item.clone();
        if which == 1 {
            put.set_stack(1);
        }
        let old = rod.attachments[which].take();
        match old {
            Some(o) if which == 0 && o.object_id() == put.object_id() => {
                let mut merged = o;
                merged.set_stack(merged.stack() + put.stack());
                rod.attachments[0] = Some(merged);
                self.farmer.inventory.slots[item_slot] = None;
            }
            old => {
                rod.attachments[which] = Some(put);
                if which == 1 {
                    rod.tackle_uses = TACKLE_USES;
                    self.farmer.inventory.reduce(item_slot, 1);
                } else {
                    self.farmer.inventory.slots[item_slot] = None;
                }
                if let Some(o) = old {
                    self.give_item(o);
                }
            }
        }
        self.farmer.inventory.slots[rod_slot] = Some(Item::Tool(rod));
        self.sound("button1");
        true
    }

    /// Gear on the held rod.
    pub fn current_gear(&self) -> Gear {
        let Some(Item::Tool(rod)) = self.farmer.current_item() else { return Gear::default() };
        Gear {
            bait: rod.attachments.first().is_some_and(|b| b.is_some()),
            tackle: rod.attachments.get(1).and_then(|t| t.as_ref()).and_then(|t| self.tackle_effect(t)),
        }
    }

    /// Public hook for tests and tools: as if a fish was just hooked.
    pub fn consume_gear_pub(&mut self) {
        self.consume_gear();
    }

    /// After a fish is hooked: use up one bait and one tackle use.
    pub(crate) fn consume_gear(&mut self) {
        let slot = self.farmer.current_slot;
        // A preserving rod keeps its bait half the time (approx.).
        let keep_bait = self.rod_has("Preserving") && engine::Rng::new(engine::Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 4111])).chance(0.5);
        let Some(Item::Tool(rod)) = self.farmer.inventory.slots.get_mut(slot).and_then(|s| s.as_mut()) else { return };
        if let Some(Some(bait)) = rod.attachments.get_mut(0).filter(|_| !keep_bait) {
            let n = bait.stack() - 1;
            if n <= 0 {
                rod.attachments[0] = None;
            } else {
                bait.set_stack(n);
            }
        }
        if rod.attachments.get(1).is_some_and(|t| t.is_some()) {
            rod.tackle_uses -= 1;
            if rod.tackle_uses <= 0 {
                rod.attachments[1] = None;
                rod.tackle_uses = 0;
                self.hud("Your tackle wore out.");
            }
        }
    }

    /// Items in a fishing treasure chest (well-known ids, else placeholders).
    pub(crate) fn fishing_treasure(&mut self, rng: &mut engine::Rng) -> Vec<Item> {
        let pool: Vec<&str> = ["535", "Dev_Geode", "378", "Dev_CopperOre", "380", "Dev_IronOre", "685", "Dev_Bait", "80", "Dev_Quartz"]
            .into_iter()
            .filter(|id| self.items.objects.contains_key(*id))
            .collect();
        let n = rng.range(1, 3) as usize;
        let mut out = Vec::new();
        for _ in 0..n {
            if let Some(id) = rng.pick(&pool).copied() {
                out.push(Item::object(id, rng.range(1, 4)));
            }
        }
        out
    }
}
