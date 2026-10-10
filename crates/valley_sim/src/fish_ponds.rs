//! Fish ponds: a building that holds one kind of fish (`Data/FishPondData`,
//! public format subset). Fish multiply every `SpawnTime` days up to the
//! pond's cap, ponds produce items by population, and population gates ask
//! for an item before the pond can grow further. Defaults are approximations.

use std::collections::BTreeMap;

use engine::Rng;
use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::world::{World, WorldEvent};
use crate::TilePos;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PondProduce {
    pub item_id: String,
    pub required_population: i32,
    pub chance: f64,
    pub min_quantity: i32,
    pub max_quantity: i32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct FishPondData {
    pub id: String,
    pub required_tags: Vec<String>,
    pub spawn_time: i32,
    pub produced_items: Vec<PondProduce>,
    /// Population -> requested items ("id [min] [max]").
    pub population_gates: Option<BTreeMap<String, Vec<String>>>,
    pub max_population: i32,
}

impl Default for FishPondData {
    fn default() -> Self {
        Self { id: String::new(), required_tags: vec![], spawn_time: -1, produced_items: vec![], population_gates: None, max_population: -1 }
    }
}

/// Defaults when an entry leaves them unset (approx.).
pub const DEFAULT_SPAWN_DAYS: i32 = 3;
pub const DEFAULT_MAX_POPULATION: i32 = 10;
pub const STARTING_CAP: i32 = 3;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PondState {
    pub fish: Option<String>,
    pub population: i32,
    /// Current population cap (raised by fulfilling gates).
    pub cap: i32,
    pub days_since_spawn: i32,
    pub output: Option<Item>,
    /// Item the pond wants before it can grow: (id, count).
    pub request: Option<(String, i32)>,
}

impl World {
    pub fn is_pond(&self, kind: &str) -> bool {
        kind == "Fish Pond" || self.buildings_data.get(kind).is_some_and(|d| d.custom_fields.as_ref().is_some_and(|f| f.contains_key("Port.FishPond")))
    }

    /// Best matching pond entry for a fish (all required tags must match).
    pub fn pond_data_for(&self, fish: &str) -> Option<FishPondData> {
        let item = Item::object(fish, 1);
        self.fish_pond_data.iter().find(|d| self.items.matches_tags(&item, &d.required_tags)).cloned()
    }

    fn is_fish(&self, item: &Item) -> bool {
        item.object_id().is_some() && self.items.context_tags(item).iter().any(|t| t == "category_fish")
    }

    /// Action on a pond: collect output, satisfy a request, or add a fish.
    pub(crate) fn interact_pond(&mut self, t: TilePos) -> bool {
        let Some(bi) = self.buildings.iter().position(|b| b.contains(t) && self.current == "Farm") else { return false };
        if !self.is_pond(&self.buildings[bi].kind) || self.buildings[bi].days_left > 0 {
            return false;
        }
        let mut pond = self.buildings[bi].pond.clone().unwrap_or_default();
        if pond.cap == 0 {
            pond.cap = STARTING_CAP;
        }
        let held = self.farmer.current_item().cloned();
        // 1. Collect the day's produce.
        if let Some(out) = pond.output.take() {
            if self.farmer.inventory.can_fit(&out) {
                self.give_item(out);
                self.sound("coin");
            } else {
                pond.output = Some(out);
                self.hud("Inventory full");
            }
        // 2. Hand over a requested item.
        } else if let (Some((want, n)), Some(h)) = (pond.request.clone(), held.as_ref()) {
            if h.object_id() == Some(want.as_str()) && h.stack() >= n {
                let slot = self.farmer.current_slot;
                self.farmer.inventory.reduce(slot, n);
                pond.request = None;
                pond.cap += if pond.cap < 5 { 2 } else { 3 };
                let max = self.pond_max(&pond);
                pond.cap = pond.cap.min(max);
                self.sound("newArtifact");
                self.hud("The pond can hold more fish now.");
            } else {
                self.describe_pond(&pond);
            }
        // 3. Add a fish.
        } else if let Some(h) = held.as_ref().filter(|h| self.is_fish(h)) {
            let id = h.object_id().unwrap().to_string();
            match &pond.fish {
                Some(f) if *f != id => {
                    self.emit(WorldEvent::Message("This pond already holds a different fish.".into()));
                }
                _ if pond.population >= pond.cap => {
                    self.emit(WorldEvent::Message("The pond is full.".into()));
                }
                _ => {
                    let slot = self.farmer.current_slot;
                    self.farmer.inventory.reduce(slot, 1);
                    pond.fish = Some(id);
                    pond.population += 1;
                    self.sound("dropItemInWater");
                }
            }
        } else {
            self.describe_pond(&pond);
        }
        self.buildings[bi].pond = Some(pond);
        true
    }

    fn describe_pond(&mut self, pond: &PondState) {
        let msg = match (&pond.fish, &pond.request) {
            (None, _) => "An empty pond. Add a fish to start a population.".to_string(),
            (Some(f), req) => {
                let name = self.items.object(f).map(|o| o.display_name.clone()).unwrap_or(f.clone());
                let mut m = format!("{name}: {}/{} fish.", pond.population, pond.cap);
                if let Some((id, n)) = req {
                    let want = self.items.object(id).map(|o| o.display_name.clone()).unwrap_or(id.clone());
                    m.push_str(&format!(" The fish would like {n} {want}."));
                }
                m
            }
        };
        self.emit(WorldEvent::Message(msg));
    }

    fn pond_max(&self, pond: &PondState) -> i32 {
        pond.fish.as_deref().and_then(|f| self.pond_data_for(f)).map(|d| d.max_population).filter(|m| *m > 0).unwrap_or(DEFAULT_MAX_POPULATION)
    }

    /// Night: grow, produce and post requests.
    pub(crate) fn ponds_overnight(&mut self) {
        let today = self.clock.days_played();
        for bi in 0..self.buildings.len() {
            if !self.is_pond(&self.buildings[bi].kind) {
                continue;
            }
            let Some(mut pond) = self.buildings[bi].pond.clone() else { continue };
            let Some(fish) = pond.fish.clone() else { continue };
            let data = self.pond_data_for(&fish).unwrap_or_default();
            let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, today as i64, self.buildings[bi].id as i64, 71]));
            let max = self.pond_max(&pond);
            // Growth.
            let spawn = if data.spawn_time > 0 { data.spawn_time } else { DEFAULT_SPAWN_DAYS };
            pond.days_since_spawn += 1;
            if pond.days_since_spawn >= spawn && pond.population < pond.cap.min(max) {
                pond.population += 1;
                pond.days_since_spawn = 0;
            }
            // A population gate at the cap asks for an item.
            if pond.request.is_none() && pond.population >= pond.cap && pond.cap < max {
                if let Some(gates) = &data.population_gates {
                    let gate = gates.iter().filter_map(|(k, v)| k.parse::<i32>().ok().map(|p| (p, v))).filter(|(p, _)| *p > pond.cap).min_by_key(|(p, _)| *p);
                    if let Some((_, reqs)) = gate {
                        if let Some(r) = rng.pick(reqs) {
                            let t: Vec<&str> = r.split_whitespace().collect();
                            let lo: i32 = t.get(1).and_then(|v| v.parse().ok()).unwrap_or(1);
                            let hi: i32 = t.get(2).and_then(|v| v.parse().ok()).unwrap_or(lo);
                            if let Some(id) = t.first() {
                                pond.request = Some((crate::item::split_qualified(id).1.to_string(), rng.range(lo, hi.max(lo) + 1)));
                            }
                        }
                    }
                } else {
                    pond.cap = max;
                }
            }
            // Produce (first entry that passes, in data order).
            if pond.output.is_none() {
                for p in data.produced_items.iter().filter(|p| pond.population >= p.required_population) {
                    if rng.chance(if p.chance > 0.0 { p.chance } else { 1.0 }) {
                        let n = rng.range(p.min_quantity.max(1), p.max_quantity.max(p.min_quantity).max(1) + 1);
                        if let Some(item) = self.items.create(&p.item_id, n) {
                            pond.output = Some(item);
                        }
                        break;
                    }
                }
            }
            self.buildings[bi].pond = Some(pond);
        }
    }
}
