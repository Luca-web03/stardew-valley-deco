//! Save games: JSON snapshots of everything that isn't reloaded from content.

use std::collections::BTreeMap;

use engine::Rng;
use serde::{Deserialize, Serialize};
use valley_content::ContentManager;

use crate::farmer::Farmer;
use crate::item::Item;
use crate::location::LocationState;
use crate::time::{Clock, Weather};
use crate::world::{NewGame, World};

pub const SAVE_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SaveData {
    pub version: u32,
    pub seed: u64,
    pub rng: [u32; 4],
    pub clock: Clock,
    pub weather: Weather,
    pub weather_tomorrow: Weather,
    pub daily_luck: f64,
    pub farmer: Farmer,
    pub locations: BTreeMap<String, LocationState>,
    pub shipping_bin: Vec<Item>,
    #[serde(default)]
    pub buildings: Vec<crate::farm::Building>,
    #[serde(default)]
    pub animals: Vec<crate::farm::FarmAnimal>,
}

impl SaveData {
    pub fn capture(w: &World) -> Self {
        Self {
            version: SAVE_VERSION,
            seed: w.seed,
            rng: w.rng.state(),
            clock: w.clock.clone(),
            weather: w.weather,
            weather_tomorrow: w.weather_tomorrow,
            daily_luck: w.daily_luck,
            farmer: w.farmer.clone(),
            locations: w.locations.iter().map(|(k, v)| (k.clone(), v.state.clone())).collect(),
            shipping_bin: w.shipping_bin.clone(),
            buildings: w.buildings.clone(),
            animals: w.animals.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("save serializes")
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    /// Rebuild a world from content plus this snapshot.
    pub fn restore(self, cm: &mut ContentManager) -> World {
        let mut w = World::new(cm, NewGame { seed: self.seed, ..Default::default() });
        w.rng = Rng::from_state(self.rng);
        w.clock = self.clock;
        w.weather = self.weather;
        w.weather_tomorrow = self.weather_tomorrow;
        w.daily_luck = self.daily_luck;
        w.farmer = self.farmer;
        // Recreate building interiors before restoring location states.
        w.buildings = self.buildings;
        w.next_building_id = w.buildings.iter().map(|b| b.id).max().unwrap_or(0);
        for b in w.buildings.clone() {
            if b.days_left == 0 {
                w.finish_building(b.id);
            }
        }
        w.rebuild_location_edges();
        w.animals = self.animals;
        w.next_animal_id = w.animals.iter().map(|a| a.id).max().unwrap_or(0);
        for (name, state) in self.locations {
            if let Some(loc) = w.locations.get_mut(&name) {
                loc.state = state;
            }
        }
        w.shipping_bin = self.shipping_bin;
        w.current = "FarmHouse".into();
        w.start_day();
        w.drain_events();
        w
    }
}
