//! Passive festivals (`Data/PassiveFestivals`, public format subset):
//! multi-day events that swap location maps (stalls, decorations, shops
//! placed as tile actions) without stopping the day.

use std::collections::BTreeMap;

use serde::Deserialize;
use valley_content::Map;

use crate::world::{World, WorldEvent};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PassiveFestivalData {
    pub display_name: String,
    pub condition: Option<String>,
    pub show_on_calendar: bool,
    pub season: String,
    pub start_day: i32,
    pub end_day: i32,
    pub start_time: i32,
    pub start_message: Option<String>,
    pub only_show_message_on_first_day: bool,
    /// Location name -> replacement map asset (under Maps/).
    pub map_replacements: BTreeMap<String, String>,
}

impl World {
    /// Festivals running today.
    pub fn passive_festival_today(&self, id: &str) -> bool {
        self.passive_festivals_today().iter().any(|(k, _)| k == id)
    }

    pub fn passive_festivals_today(&self) -> Vec<(String, PassiveFestivalData)> {
        let season = self.clock.season.key();
        self.passive_festivals
            .iter()
            .filter(|(_, f)| f.season.eq_ignore_ascii_case(season) && (f.start_day..=f.end_day).contains(&self.clock.day))
            .filter(|(_, f)| {
                let mut rng = self.day_rng(61);
                let mail = self.farmer.mail_received.clone();
                let seen = self.farmer.events_seen.clone();
                let mut ctx = crate::gsq::QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng, world: Some(self) };
                crate::gsq::check(f.condition.as_deref(), &mut ctx)
            })
            .map(|(k, f)| (k.clone(), f.clone()))
            .collect()
    }

    /// Morning: undo yesterday's replacements, apply today's, announce.
    pub(crate) fn passive_festivals_morning(&mut self) {
        // Restore maps replaced for festivals that ended.
        let originals = std::mem::take(&mut self.festival_original_maps);
        for (loc, map) in originals {
            self.swap_location_map(&loc, map);
        }
        for (_, f) in self.passive_festivals_today() {
            for (loc, asset) in &f.map_replacements {
                let Some(map) = self.passive_maps.get(asset).cloned() else { continue };
                if let Some(old) = self.swap_location_map(loc, map) {
                    self.festival_original_maps.entry(loc.clone()).or_insert(old);
                }
            }
            let first = self.clock.day == f.start_day;
            if let Some(msg) = &f.start_message {
                if first || !f.only_show_message_on_first_day {
                    let text = self.items.expand_text(msg);
                    self.emit(WorldEvent::Hud(text));
                }
            }
        }
    }

    /// Replace a location's map in place (keeping its state); returns the old map.
    fn swap_location_map(&mut self, loc: &str, map: Map) -> Option<Map> {
        let l = self.locations.get_mut(loc)?;
        l.warps = map.prop("Warp").map(|w| crate::location::parse_warps(&w)).unwrap_or_default();
        let old = std::mem::replace(&mut l.map, map);
        self.build_location_edges_pub();
        Some(old)
    }
}

/// Load the data and every replacement map it names.
pub fn load(cm: &mut valley_content::ContentManager) -> (BTreeMap<String, PassiveFestivalData>, BTreeMap<String, Map>) {
    let data: BTreeMap<String, PassiveFestivalData> = cm.load_json("Data/PassiveFestivals").unwrap_or_default();
    let mut maps = BTreeMap::new();
    for f in data.values() {
        for asset in f.map_replacements.values() {
            if !maps.contains_key(asset) {
                if let Some(m) = cm.load_map(&format!("Maps/{asset}")) {
                    maps.insert(asset.clone(), m);
                }
            }
        }
    }
    (data, maps)
}

/// A fish that earns a reward while a passive festival runs
/// (`Port/FestivalCatches`: [{ "Festival", "Fish", "Reward", "Location"? }]);
/// without the asset the trout derby's golden tag for each rainbow trout.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct FestivalCatch {
    pub festival: String,
    pub fish: String,
    pub reward: String,
    pub location: Option<String>,
}

pub fn default_festival_catches() -> Vec<FestivalCatch> {
    vec![FestivalCatch { festival: "TroutDerby".into(), fish: "(O)138".into(), reward: "(O)TroutDerbyTag".into(), location: None }]
}

impl World {
    /// A fish was caught: festival rewards for it.
    pub fn festival_catch_rewards(&mut self, fish_id: &str) {
        let today: Vec<String> = self.passive_festivals_today().into_iter().map(|(k, _)| k).collect();
        if today.is_empty() {
            return;
        }
        let rules: Vec<FestivalCatch> = self
            .festival_catches
            .iter()
            .filter(|c| today.contains(&c.festival) && crate::item::split_qualified(&c.fish).1 == fish_id)
            .filter(|c| c.location.as_deref().map_or(true, |l| l == self.current))
            .cloned()
            .collect();
        for c in rules {
            if let Some(item) = self.items.create(&c.reward, 1) {
                let name = self.items.display_name(&item);
                self.give_item(item);
                self.hud(format!("You got a {name}!"));
            }
        }
    }
}

