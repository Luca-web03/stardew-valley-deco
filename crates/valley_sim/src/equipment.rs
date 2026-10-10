//! Rings and boots, bombs, crab pots.
//!
//! Ring effects use this engine's `Port.*` CustomFields convention on ring
//! objects (Port.Defense, Port.Attack, Port.Magnet, Port.Luck, Port.Speed);
//! boots come from Data/Boots. Bomb radii for the well-known bomb ids follow
//! their documented sizes; other bombs use a `BombRadius` custom field.

use engine::{Rng, Vec2};

use crate::buffs::BuffAttrs;
use crate::item::Item;
use crate::objects::{ObjKind, PlacedObject};
use crate::world::{World, WorldEvent};
use crate::{TilePos, TILE_F};

const CRAB_POTS: &[&str] = &["710", "Dev_CrabPot"];
const BAITS: &[&str] = &["685", "Dev_Bait"];

#[derive(Clone, Debug, PartialEq)]
pub struct LitBomb {
    pub location: String,
    pub tile: TilePos,
    pub radius: i32,
    pub fuse_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Explosion {
    pub location: String,
    pub center: Vec2,
    pub radius: i32,
    pub ms: u32,
}

/// Trap catches parsed from Data/Fish entries whose second field is "trap".
#[derive(Clone, Debug, PartialEq)]
pub struct TrapFish {
    pub id: String,
    pub chance: f64,
}

pub fn parse_trap(id: &str, s: &str) -> Option<TrapFish> {
    let f: Vec<&str> = s.split('/').collect();
    (f.get(1) == Some(&"trap")).then(|| TrapFish { id: id.to_string(), chance: f.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.1) })
}

impl World {
    pub fn is_ring(&self, item: &Item) -> bool {
        item.object_id().and_then(|id| self.items.object(id)).is_some_and(|o| o.data.kind.eq_ignore_ascii_case("Ring") || o.data.category == -96)
    }

    /// Equip the item in inventory `slot` (ring, boots or clothing), swapping out the old one.
    pub fn equip(&mut self, slot: usize) -> bool {
        let Some(item) = self.farmer.inventory.get(slot).cloned() else { return false };
        if matches!(item, Item::Trinket { .. }) {
            let old = self.farmer.trinket.replace(item);
            self.farmer.inventory.slots[slot] = old;
            self.sound("crit");
            return true;
        }
        if let Item::Clothing { slot: kind, .. } = &item {
            let worn = match kind {
                crate::item::ClothingSlot::Hat => &mut self.farmer.hat,
                crate::item::ClothingSlot::Shirt => &mut self.farmer.shirt_item,
                crate::item::ClothingSlot::Pants => &mut self.farmer.pants_item,
            };
            let old = worn.replace(item);
            self.farmer.inventory.slots[slot] = old;
            self.sound("dwop");
            return true;
        }
        if matches!(item, Item::Boots { .. }) {
            let old = self.farmer.boots.replace(item);
            self.farmer.inventory.slots[slot] = old;
            self.sound("sandyStep");
            return true;
        }
        if self.is_ring(&item) {
            let mut one = item.clone();
            one.set_stack(1);
            let idx = self.farmer.rings.iter().position(|r| r.is_none()).unwrap_or(0);
            let old = self.farmer.rings[idx].replace(one);
            self.farmer.inventory.reduce(slot, 1);
            if let Some(o) = old {
                self.farmer.inventory.add(o);
            }
            self.sound("crit");
            return true;
        }
        false
    }

    /// Unequip ring 0/1 (`which` 0..=1), boots (2), hat (3), shirt (4),
    /// pants (5) or trinket (6).
    pub fn unequip(&mut self, which: usize) -> bool {
        let item = match which {
            0 | 1 => self.farmer.rings[which].clone(),
            2 => self.farmer.boots.clone(),
            3 => self.farmer.hat.clone(),
            4 => self.farmer.shirt_item.clone(),
            5 => self.farmer.pants_item.clone(),
            _ => self.farmer.trinket.clone(),
        };
        let Some(item) = item else { return false };
        if !self.farmer.inventory.can_fit(&item) {
            return false;
        }
        match which {
            0 | 1 => self.farmer.rings[which] = None,
            2 => self.farmer.boots = None,
            3 => self.farmer.hat = None,
            4 => self.farmer.shirt_item = None,
            5 => self.farmer.pants_item = None,
            _ => self.farmer.trinket = None,
        }
        self.farmer.inventory.add(item);
        true
    }

    /// The farmer's look with worn clothing colours applied.
    pub fn worn_appearance(&self) -> crate::farmer::Appearance {
        let mut a = self.farmer.appearance.clone();
        let color = |item: &Option<Item>| match item {
            Some(Item::Clothing { slot, id }) => self.items.clothing(*slot, id).and_then(|c| c.color),
            _ => None,
        };
        if let Some(c) = color(&self.farmer.shirt_item) {
            a.shirt = c;
        }
        if let Some(c) = color(&self.farmer.pants_item) {
            a.pants = c;
        }
        a
    }

    /// Attributes from rings and boots.
    pub fn equipment_attrs(&self) -> BuffAttrs {
        let mut a = BuffAttrs::default();
        for r in self.farmer.rings.iter().flatten() {
            let Some(o) = r.object_id().and_then(|id| self.items.object(id)) else { continue };
            let cf = o.data.custom_fields.clone().unwrap_or_default();
            let g = |k: &str| cf.get(k).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
            a.add(&BuffAttrs {
                defense: g("Port.Defense"),
                attack: g("Port.Attack"),
                magnetic_radius: g("Port.Magnet"),
                luck: g("Port.Luck"),
                speed: g("Port.Speed"),
                ..Default::default()
            });
        }
        if let Some(Item::Boots { id }) = &self.farmer.boots {
            if let Some(b) = self.items.boots.get(id) {
                a.defense += b.defense;
            }
        }
        if let Some(Item::Trinket { id }) = &self.farmer.trinket {
            let spec = self.items.trinkets.get(id).and_then(|t| t.custom_fields.as_ref()).and_then(|f| f.get("Port.TrinketStats")).cloned().unwrap_or_default();
            let toks: Vec<&str> = spec.split_whitespace().collect();
            for pair in toks.chunks(2) {
                let n: i32 = pair.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                match pair[0] {
                    "attack" => a.attack += n,
                    "defense" => a.defense += n,
                    "speed" => a.speed += n,
                    "magnet" => a.magnetic_radius += n,
                    "luck" => a.luck += n,
                    _ => {}
                }
            }
        }
        a
    }

    /// Buffs plus equipment.
    pub fn total_attrs(&self) -> BuffAttrs {
        let mut a = self.farmer.buff_totals();
        a.add(&self.equipment_attrs());
        a
    }

    fn bomb_radius(&self, id: &str) -> Option<i32> {
        match id {
            "286" => return Some(3),
            "287" => return Some(5),
            "288" => return Some(7),
            _ => {}
        }
        self.items.object(id)?.data.custom_fields.as_ref()?.get("BombRadius")?.parse().ok()
    }

    /// Light the held bomb at `t`.
    pub(crate) fn place_bomb(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(id) = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from) else { return false };
        let Some(radius) = self.bomb_radius(&id) else { return false };
        if self.location().has_tile("Buildings", t) || self.location().is_water(t) {
            return true;
        }
        self.farmer.inventory.reduce(slot, 1);
        self.bombs.push(LitBomb { location: self.current.clone(), tile: t, radius, fuse_ms: 1500 });
        self.sound("fuse");
        true
    }

    pub(crate) fn update_bombs(&mut self, dt: u32) {
        for e in self.explosions.iter_mut() {
            e.ms = e.ms.saturating_sub(dt);
        }
        self.explosions.retain(|e| e.ms > 0);
        let mut boom = Vec::new();
        for b in self.bombs.iter_mut() {
            b.fuse_ms = b.fuse_ms.saturating_sub(dt);
            if b.fuse_ms == 0 {
                boom.push(b.clone());
            }
        }
        self.bombs.retain(|b| b.fuse_ms > 0);
        for b in boom {
            self.explode(&b);
        }
    }

    fn explode(&mut self, b: &LitBomb) {
        let center = b.tile.world_center();
        let r = b.radius;
        self.explosions.push(Explosion { location: b.location.clone(), center, radius: r, ms: 400 });
        self.sound("explosion");
        if b.location != self.current {
            return;
        }
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy > r * r {
                    continue;
                }
                let t = TilePos::new(b.tile.x + dx, b.tile.y + dy);
                if self.hit_mine_rock(t, 99) {
                    continue;
                }
                if let Some(PlacedObject { kind: ObjKind::Debris { debris, .. }, .. }) = self.location().object(t).cloned() {
                    self.location_mut().state.objects.remove(&t);
                    if debris == crate::objects::DebrisKind::Stone {
                        let stone = self.mine_items.stone.clone();
                        self.spawn_drop(Item::object(&stone, 1), t.world_center());
                    }
                }
            }
        }
        let reach = r as f32 * TILE_F;
        let dmg = 20 + r * 8;
        let here = self.current.clone();
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 77]));
        let mut killed = Vec::new();
        for (i, m) in self.monsters.iter_mut().enumerate() {
            if m.location == here && m.center().distance(center) <= reach {
                m.health -= dmg + rng.range(0, 10);
                m.knock = (m.center() - center).normalized() * 10.0;
                if m.health <= 0 {
                    killed.push(i);
                }
            }
        }
        for i in killed.into_iter().rev() {
            let m = self.monsters.remove(i);
            self.farmer.skills.gain(crate::farmer::Skill::Combat, m.info.exp);
            self.monsters_killed += 1;
        }
        if self.farmer.center().distance(center) <= reach * 0.75 {
            self.damage_farmer(dmg / 3, center);
        }
    }

    /// Place a crab pot on water, bait it, or collect its catch.
    pub(crate) fn crab_pot_action(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let held = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from);
        if let Some(PlacedObject { kind: ObjKind::CrabPot { bait, catch }, .. }) = self.location().object(t).cloned() {
            if let Some(c) = catch {
                if self.farmer.inventory.can_fit(&c) {
                    self.farmer.inventory.add(c.clone());
                    if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                        o.kind = ObjKind::CrabPot { bait: false, catch: None };
                    }
                    self.sound("coin");
                    self.emit(WorldEvent::Pickup(c));
                }
                return true;
            }
            if !bait && held.as_deref().is_some_and(|h| BAITS.contains(&h)) {
                self.farmer.inventory.reduce(slot, 1);
                if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                    o.kind = ObjKind::CrabPot { bait: true, catch: None };
                }
                self.sound("Ship");
            }
            return true;
        }
        let Some(h) = held else { return false };
        if !CRAB_POTS.contains(&h.as_str()) {
            return false;
        }
        if !self.location().is_water(t) || self.location().object(t).is_some() {
            self.hud("Crab pots go in the water.");
            return true;
        }
        self.farmer.inventory.reduce(slot, 1);
        self.location_mut().state.objects.insert(t, PlacedObject::new(&h, false, ObjKind::CrabPot { bait: false, catch: None }));
        self.sound("dropItemInWater");
        true
    }

    pub(crate) fn crab_pots_overnight(&mut self) {
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 710]));
        let traps = self.trap_fish.clone();
        let trash = ["168", "Dev_Driftwood"].iter().find(|id| self.items.objects.contains_key(**id)).map(|s| s.to_string());
        for loc in self.locations.values_mut() {
            for o in loc.state.objects.values_mut() {
                let ObjKind::CrabPot { bait: true, catch: None } = &o.kind else { continue };
                let mut got = None;
                for t in &traps {
                    if rng.chance(t.chance) {
                        got = Some(Item::object(&t.id, 1));
                        break;
                    }
                }
                let got = got.or_else(|| trash.as_ref().map(|id| Item::object(id, 1)));
                o.kind = ObjKind::CrabPot { bait: false, catch: got };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_trap_entries() {
        assert_eq!(parse_trap("X", "Shellback/trap/.2/x/freshwater/1/9/false"), Some(TrapFish { id: "X".into(), chance: 0.2 }));
        assert!(parse_trap("Y", "Perch/20/mixed/1/9").is_none());
    }
}
