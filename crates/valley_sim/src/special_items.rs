//! Items whose use is game code in the original (public behaviour): the
//! Stardrop (+34 maximum energy when eaten), the Horse Flute (calls your
//! horse to you outdoors), Golden Animal Crackers (an animal fed one gives
//! double produce for good; `CanEatGoldenCrackers` in `Data/FarmAnimals`),
//! the Treasure Totem (artifact spots appear around you, approx. count),
//! Fairy Dust (finishes a working machine at once) and big chests (70
//! slots instead of 36). Original ids first, placeholders last; a
//! `Port.Special` custom field (`Stardrop`, `HorseFlute`, `AnimalCracker`,
//! `TreasureTotem`, `FairyDust`) marks other items as one of these.

use engine::Rng;

use crate::item::Item;
use crate::objects::{ObjKind, PlacedObject};
use crate::world::World;
use crate::TilePos;

pub const STARDROP: &[&str] = &["434", "Dev_Stardrop"];
pub const HORSE_FLUTE: &[&str] = &["911", "Dev_HorseFlute"];
pub const ANIMAL_CRACKER: &[&str] = &["GoldenAnimalCracker", "Dev_AnimalCracker"];
pub const TREASURE_TOTEM: &[&str] = &["TreasureTotem", "Dev_TreasureTotem"];
pub const FAIRY_DUST: &[&str] = &["872", "Dev_FairyDust"];
pub const BIG_CHESTS: &[&str] = &["BigChest", "BigStoneChest", "Dev_BigChest"];

/// Energy a Stardrop adds.
pub const STARDROP_ENERGY: i32 = 34;
pub const CHEST_SLOTS: usize = 36;
pub const BIG_CHEST_SLOTS: usize = 70;
/// Artifact spots a treasure totem raises (approx.) and their radius.
pub const TOTEM_SPOTS: (i32, i32) = (6, 10);
pub const TOTEM_RADIUS: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Special {
    Stardrop,
    HorseFlute,
    AnimalCracker,
    TreasureTotem,
    FairyDust,
}

impl World {
    /// Which special item an object id is, if any.
    pub fn special_item(&self, id: &str) -> Option<Special> {
        let tag = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.Special")).cloned();
        let is = |list: &[&str], name: &str| list.contains(&id) || tag.as_deref() == Some(name);
        Some(if is(STARDROP, "Stardrop") {
            Special::Stardrop
        } else if is(HORSE_FLUTE, "HorseFlute") {
            Special::HorseFlute
        } else if is(ANIMAL_CRACKER, "AnimalCracker") {
            Special::AnimalCracker
        } else if is(TREASURE_TOTEM, "TreasureTotem") {
            Special::TreasureTotem
        } else if is(FAIRY_DUST, "FairyDust") {
            Special::FairyDust
        } else {
            return None;
        })
    }

    /// Slots of a chest big craftable.
    pub fn chest_capacity(&self, big_id: &str) -> usize {
        if BIG_CHESTS.contains(&big_id) {
            BIG_CHEST_SLOTS
        } else {
            CHEST_SLOTS
        }
    }

    /// Eating a Stardrop: more energy for good.
    pub(crate) fn stardrop_eaten(&mut self) {
        self.farmer.max_stamina += STARDROP_ENERGY;
        self.farmer.stamina = self.farmer.max_stamina as f32;
        *self.farmer.counters.entry("Port.Stardrops".into()).or_insert(0) += 1;
        self.sound("stardrop");
        self.emit(crate::world::WorldEvent::Message("You feel a surge of energy. Your maximum energy has increased.".into()));
    }

    /// Special items used on a target (animals, machines), checked before
    /// the target's own action.
    pub(crate) fn use_targeted_special(&mut self, t: TilePos) -> bool {
        let Some(Item::Object { id, .. }) = self.farmer.current_item() else { return false };
        match self.special_item(id) {
            Some(Special::AnimalCracker) if self.animal_at(t).is_some() => self.use_special_item(t),
            Some(Special::FairyDust) if matches!(self.location().object(t), Some(PlacedObject { kind: ObjKind::Machine { .. }, .. })) => self.use_special_item(t),
            _ => false,
        }
    }

    /// The action button with a special item held (not eaten ones).
    pub(crate) fn use_special_item(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(Item::Object { id, .. }) = self.farmer.current_item().cloned() else { return false };
        match self.special_item(&id) {
            Some(Special::HorseFlute) => self.horse_flute(),
            Some(Special::AnimalCracker) => {
                let Some(i) = self.animal_at(t) else { return false };
                let can = self.animals_data.get(&self.animals[i].kind).map_or(true, |d| d.can_eat_golden_crackers);
                if !can || self.animals[i].golden_cracker {
                    self.hud("It doesn't want that.");
                    return true;
                }
                self.animals[i].golden_cracker = true;
                self.animals[i].emote_ms = 1200;
                self.farmer.inventory.reduce(slot, 1);
                self.sound("give_gift");
                true
            }
            Some(Special::TreasureTotem) => {
                if !self.location().outdoors {
                    self.hud("This needs to be used outdoors.");
                    return true;
                }
                let n = self.treasure_totem();
                self.farmer.inventory.reduce(slot, 1);
                self.sound("wand");
                self.emit(crate::world::WorldEvent::Flash);
                if n == 0 {
                    self.hud("Nothing stirs here.");
                }
                true
            }
            Some(Special::FairyDust) => {
                let ready = match self.location_mut().state.objects.get_mut(&t) {
                    Some(PlacedObject { kind: ObjKind::Machine { output: Some(_), minutes_left, ready }, .. }) if !*ready => {
                        *minutes_left = 0;
                        *ready = true;
                        true
                    }
                    _ => false,
                };
                if ready {
                    self.farmer.inventory.reduce(slot, 1);
                    self.sound("yoba");
                }
                ready
            }
            _ => false,
        }
    }

    /// Call the horse over (outdoors, when you have one and aren't riding).
    fn horse_flute(&mut self) -> bool {
        if !self.location().outdoors || self.farmer.mounted.is_some() {
            self.hud("Your horse can't come here.");
            return true;
        }
        let here = self.current.clone();
        let me = self.farmer.tile();
        let spot = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().map(|(dx, dy)| TilePos::new(me.x + dx, me.y + dy)).find(|p| crate::collision::tile_passable(self.location(), *p));
        let Some(spot) = spot else { return true };
        let Some(h) = self.buildings.iter_mut().find_map(|b| b.horse.as_mut()) else {
            self.hud("You don't have a horse.");
            return true;
        };
        h.location = here;
        h.pos = engine::Vec2::new(spot.x as f32 * crate::TILE_F, spot.y as f32 * crate::TILE_F - 16.0);
        self.sound("horse_flute");
        true
    }

    /// Raise artifact spots around the farmer. Returns how many appeared.
    fn treasure_totem(&mut self) -> usize {
        let spot = crate::orchard::artifact_spot_id(&self.items);
        let Some(spot) = spot else { return 0 };
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 917]));
        let want = rng.range(TOTEM_SPOTS.0, TOTEM_SPOTS.1 + 1) as usize;
        let me = self.farmer.tile();
        let mut placed = 0;
        for _ in 0..want * 6 {
            if placed >= want {
                break;
            }
            let t = TilePos::new(me.x + rng.range(-TOTEM_RADIUS, TOTEM_RADIUS + 1), me.y + rng.range(-TOTEM_RADIUS, TOTEM_RADIUS + 1));
            let loc = self.location();
            if t == me || !loc.is_clear(t) || !loc.is_diggable(t) || !crate::collision::tile_passable(loc, t) {
                continue;
            }
            self.location_mut().state.objects.insert(t, PlacedObject::new(&spot, false, ObjKind::ArtifactSpot));
            placed += 1;
        }
        placed
    }
}
