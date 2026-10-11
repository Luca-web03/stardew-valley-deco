//! Slime hutches and incubators.
//!
//! * A hutch is a building with id `Slime Hutch` or the `Port.SlimeHutch`
//!   custom field. Its slimes are monsters living inside (up to 20); they
//!   are saved with the building and respawn when the hutch is entered.
//! * Water troughs are interior tiles with the `Port.Trough` property (or
//!   the original's four trough tiles at x 16, y 6..=9 when none are
//!   marked). Each night, each watered trough can grow a slime ball
//!   (approx. chance: 25% per slime, capped at 100%); troughs then dry.
//! * Slime balls are big craftables (`Port.SlimeBall` on the hutch building
//!   names the id, default `56`); breaking one drops slime (`766`, or the
//!   ball's `Port.SlimeBallDrop`) and sometimes petrified slime (`557`).
//! * Incubators (big craftable `156` or `Port.SlimeIncubator`) take a slime
//!   egg (objects with `Port.SlimeType <monster>` or well-known egg ids) and
//!   hatch it after 4000 minutes into the hutch they stand in.

use std::collections::BTreeSet;

use engine::Rng;

use crate::combat::Monster;
use crate::item::Item;
use crate::objects::{ObjKind, PlacedObject};
use crate::world::{World, WorldEvent};
use crate::TilePos;

pub const MAX_SLIMES: usize = 20;
pub const INCUBATION_MINUTES: i32 = 4000;
const SLIME: &str = "766";
const PETRIFIED_SLIME: &str = "557";
const PETRIFIED_CHANCE: f64 = 0.05;

/// Well-known slime eggs: (object id, monster name).
const EGGS: &[(&str, &str)] = &[("680", "Green Slime"), ("413", "Frost Jelly"), ("437", "Sludge"), ("439", "Sludge"), ("857", "Tiger Slime")];

impl World {
    fn is_hutch_kind(&self, kind: &str) -> bool {
        kind == "Slime Hutch" || self.buildings_data.get(kind).and_then(|d| d.custom_fields.as_ref()).is_some_and(|f| f.contains_key("Port.SlimeHutch"))
    }

    /// Index of the hutch building whose interior is `location`.
    pub fn hutch_of(&self, location: &str) -> Option<usize> {
        self.buildings.iter().position(|b| b.interior.as_deref() == Some(location) && self.is_hutch_kind(&b.kind))
    }

    /// Trough tiles of a hutch interior.
    pub fn troughs(&self, location: &str) -> Vec<TilePos> {
        let Some(loc) = self.locations.get(location) else { return vec![] };
        let mut v = Vec::new();
        for y in 0..loc.height() {
            for x in 0..loc.width() {
                let p = TilePos::new(x, y);
                if loc.tile_prop("Buildings", p, "Port.Trough").is_some() || loc.tile_prop("Back", p, "Port.Trough").is_some() {
                    v.push(p);
                }
            }
        }
        if v.is_empty() && loc.width() > 16 {
            v = (6..=9).map(|y| TilePos::new(16, y)).collect();
        }
        v
    }

    /// Watering can on a hutch tile: fill a trough. True if it was one.
    pub(crate) fn water_trough(&mut self, t: TilePos) -> bool {
        let here = self.current.clone();
        let Some(bi) = self.hutch_of(&here) else { return false };
        if !self.troughs(&here).contains(&t) {
            return false;
        }
        let b = &mut self.buildings[bi];
        if !b.watered_troughs.contains(&t) {
            b.watered_troughs.push(t);
        }
        true
    }

    pub fn slime_type_of_egg(&self, id: &str) -> Option<String> {
        let custom = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.SlimeType")).cloned();
        custom.or_else(|| EGGS.iter().find(|(k, _)| *k == id).map(|(_, m)| m.to_string()))
    }

    pub fn is_incubator(&self, big_id: &str) -> bool {
        big_id == "156" || self.items.big(big_id).and_then(|b| b.data.custom_fields.as_ref()).is_some_and(|f| f.contains_key("Port.SlimeIncubator"))
    }

    /// Put the held slime egg into the incubator at `t`. True if handled.
    pub(crate) fn load_incubator(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).cloned() else { return false };
        if !o.big || !self.is_incubator(&o.item_id) {
            return false;
        }
        if self.hutch_of(&self.current.clone()).is_none() {
            self.hud("Incubators only work inside a slime hutch.");
            return true;
        }
        if let ObjKind::Machine { output: Some(_), .. } = o.kind {
            if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                o.shake = 10;
            }
            return true;
        }
        let slot = self.farmer.current_slot;
        let Some(id) = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from) else {
            self.hud("Place a slime egg inside to incubate it.");
            return true;
        };
        if self.slime_type_of_egg(&id).is_none() {
            self.hud("Only slime eggs fit in here.");
            return true;
        }
        self.farmer.inventory.reduce(slot, 1);
        if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
            o.kind = ObjKind::Machine { output: Some(Item::object(&id, 1)), minutes_left: INCUBATION_MINUTES, ready: false };
            o.shake = 12;
        }
        self.sound("coin");
        true
    }

    /// Incubators that finished: hatch into their hutch.
    pub(crate) fn hatch_incubators(&mut self) {
        let mut hatched = Vec::new();
        for (name, loc) in &self.locations {
            for (p, o) in &loc.state.objects {
                if let ObjKind::Machine { output: Some(egg), ready: true, .. } = &o.kind {
                    if o.big && self.is_incubator(&o.item_id) {
                        hatched.push((name.clone(), *p, egg.object_id().unwrap_or_default().to_string()));
                    }
                }
            }
        }
        for (name, p, egg) in hatched {
            let Some(kind) = self.slime_type_of_egg(&egg) else { continue };
            let Some(bi) = self.hutch_of(&name) else { continue };
            if self.buildings[bi].slimes.len() >= MAX_SLIMES {
                continue;
            }
            if let Some(o) = self.locations.get_mut(&name).and_then(|l| l.state.objects.get_mut(&p)) {
                o.kind = ObjKind::Machine { output: None, minutes_left: 0, ready: false };
            }
            self.buildings[bi].slimes.push(kind.clone());
            if self.spawned_hutches.contains(&name) {
                self.spawn_hutch_slime(&name, &kind, Some(TilePos::new(p.x, p.y + 1)));
            }
            if name == self.current {
                self.sound("slimeHit");
                self.emit(WorldEvent::Hud("A slime hatched!".into()));
            }
        }
    }

    fn spawn_hutch_slime(&mut self, location: &str, kind: &str, near: Option<TilePos>) {
        let Some(info) = self.monster_info.get(kind).cloned().or_else(|| self.monster_info.values().find(|m| m.name.contains("Slime") || m.name.contains("Blob")).cloned()) else { return };
        let Some(loc) = self.locations.get(location) else { return };
        let free: Vec<TilePos> = (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).filter(|p| loc.is_clear(*p) && crate::collision::tile_passable(loc, *p)).collect();
        if free.is_empty() {
            return;
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, self.monsters.len() as i64, 131]));
        let tile = near.filter(|n| free.contains(n)).unwrap_or_else(|| free[rng.range(0, free.len() as i32) as usize]);
        self.next_monster_id += 1;
        self.monsters.push(Monster::new(self.next_monster_id, info, location, tile));
    }

    /// Entering a hutch brings its slimes out (once per session).
    pub(crate) fn sync_hutch_slimes(&mut self) {
        let here = self.current.clone();
        if self.spawned_hutches.contains(&here) {
            return;
        }
        let Some(bi) = self.hutch_of(&here) else { return };
        self.spawned_hutches.insert(here.clone());
        let kinds = self.buildings[bi].slimes.clone();
        for k in kinds {
            self.spawn_hutch_slime(&here, &k, None);
        }
    }

    /// Slimes killed in a hutch are gone from it.
    fn record_hutch_population(&mut self) {
        let spawned: Vec<String> = self.spawned_hutches.iter().cloned().collect();
        for name in spawned {
            let Some(bi) = self.hutch_of(&name) else { continue };
            self.buildings[bi].slimes = self.monsters.iter().filter(|m| m.location == name).map(|m| m.info.name.clone()).collect();
        }
    }

    /// Night: troughs grow slime balls, then dry up.
    pub(crate) fn hutches_overnight(&mut self) {
        self.record_hutch_population();
        let mut rng = self.day_rng(137);
        for bi in 0..self.buildings.len() {
            let b = self.buildings[bi].clone();
            if !self.is_hutch_kind(&b.kind) {
                continue;
            }
            let Some(interior) = b.interior.clone() else { continue };
            let ball = self.buildings_data.get(&b.kind).and_then(|d| d.custom_fields.as_ref()).and_then(|f| f.get("Port.SlimeBall")).cloned().unwrap_or_else(|| "56".into());
            let chance = (b.slimes.len() as f64 * 0.25).min(1.0);
            for _ in 0..b.watered_troughs.len() {
                if !rng.chance(chance) {
                    continue;
                }
                let Some(loc) = self.locations.get_mut(&interior) else { break };
                let Some(p) = loc.free_area(1, 1, |n| rng.range(0, n as i32) as usize).or_else(|| {
                    let free: Vec<TilePos> = (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).filter(|p| loc.is_clear(*p) && crate::collision::tile_passable(loc, *p)).collect();
                    (!free.is_empty()).then(|| free[rng.range(0, free.len() as i32) as usize])
                }) else { break };
                loc.state.objects.insert(p, PlacedObject::new(&ball, true, ObjKind::Placed));
            }
            self.buildings[bi].watered_troughs.clear();
        }
    }

    pub fn is_slime_ball(&self, o: &PlacedObject) -> bool {
        o.big && (o.item_id == "56" || self.items.big(&o.item_id).and_then(|b| b.data.custom_fields.as_ref()).is_some_and(|f| f.contains_key("Port.SlimeBallDrop")))
    }

    /// Break a slime ball at `t` (any tool or the action button).
    pub(crate) fn break_slime_ball(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).cloned() else { return false };
        if !self.is_slime_ball(&o) {
            return false;
        }
        self.location_mut().state.objects.remove(&t);
        let drop = self.items.big(&o.item_id).and_then(|b| b.data.custom_fields.as_ref()).and_then(|f| f.get("Port.SlimeBallDrop")).cloned().unwrap_or_else(|| SLIME.into());
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, t.x as i64, t.y as i64]));
        let n = rng.range(10, 21);
        if let Some(item) = self.items.create(&drop, n) {
            self.spawn_drop(item, t.world_center());
        }
        if rng.chance(PETRIFIED_CHANCE) {
            if let Some(item) = self.items.create(PETRIFIED_SLIME, 1) {
                self.spawn_drop(item, t.world_center());
            }
        }
        self.sound("slimedead");
        true
    }

    pub fn hutch_names(&self) -> BTreeSet<String> {
        self.buildings.iter().filter(|b| self.is_hutch_kind(&b.kind)).filter_map(|b| b.interior.clone()).collect()
    }
}
