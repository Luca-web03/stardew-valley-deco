//! Things that happen to the farm overnight:
//!
//! * Crows: with 16+ crops on the farm, each block of 16 crops gives one
//!   30% chance (up to 4) of a crow eating a random crop, unless a
//!   scarecrow covers it. Scarecrows are big craftables with the
//!   `crow_scare` context tag (radius from `crow_scare_radius_<n>`, default
//!   9 tiles), or with "arecrow" in their name. Eaten crops have crows on
//!   them in the morning that fly off when the farmer comes close.
//! * Night events (approx. 1% chances each): a fairy fully grows a patch of
//!   crops, a witch leaves a void egg in a coop, a meteorite lands, a stone
//!   owl or a strange capsule appears; the first summer 3rd brings an
//!   earthquake (sets the `Port_Earthquake` flag, usable with
//!   `Port.ClearedBy`).
//! * Weather totems: objects with `Port.SetWeather <weather>` (or the rain
//!   totem, id 681) set tomorrow's weather.

use engine::{Rng, Vec2};
use serde::{Deserialize, Serialize};

use crate::objects::{ObjKind, PlacedObject};
use crate::terrain::TerrainFeature;
use crate::time::{Season, Weather};
use crate::world::{World, WorldEvent};
use crate::{TilePos, TILE_F};

pub const CROPS_PER_CROW: usize = 16;
pub const CROW_CHANCE: f64 = 0.3;
pub const MAX_CROWS: usize = 4;
pub const SCARECROW_RADIUS: f32 = 9.0;
pub const DELUXE_SCARECROW_RADIUS: f32 = 17.0;
/// Crops within this distance of the fairy's pick grow fully.
pub const FAIRY_RADIUS: f32 = 3.0;
/// Rain totem object id.
const RAIN_TOTEM: &str = "681";
/// Night event drops (big craftable ids).
const STONE_OWL: &str = "95";
const STRANGE_CAPSULE: &str = "96";
const VOID_EGG: &str = "305";

/// A crow on the farm in the morning (not saved).
#[derive(Clone, Debug, PartialEq)]
pub struct Crow {
    pub location: String,
    pub pos: Vec2,
    pub flying: bool,
    pub ms: u32,
    pub flip: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NightEvent {
    Fairy,
    Witch,
    Meteorite,
    Owl,
    Capsule,
    Earthquake,
}

impl NightEvent {
    pub fn text(self) -> &'static str {
        match self {
            NightEvent::Fairy => "A fairy fluttered over the farm in the night, and some crops grew overnight.",
            NightEvent::Witch => "A witch flew over the farm in the night...",
            NightEvent::Meteorite => "Something fell from the sky and crashed onto the farm in the night!",
            NightEvent::Owl => "Something was moving around the farm in the night...",
            NightEvent::Capsule => "A strange light flashed over the farm in the night...",
            NightEvent::Earthquake => "The whole valley shook in the night. Was that an earthquake?",
        }
    }
}

impl World {
    /// Scarecrows on a location: (tile, radius).
    pub fn scarecrows(&self, location: &str) -> Vec<(TilePos, f32)> {
        let Some(loc) = self.locations.get(location) else { return vec![] };
        let mut out = Vec::new();
        for (p, o) in &loc.state.objects {
            if !o.big {
                continue;
            }
            let Some(info) = self.items.big(&o.item_id) else { continue };
            let tags = info.data.context_tags.clone().unwrap_or_default();
            let radius = tags.iter().find_map(|t| t.strip_prefix("crow_scare_radius_").and_then(|n| n.parse::<f32>().ok()));
            let tagged = tags.iter().any(|t| t == "crow_scare");
            let named = info.data.name.to_ascii_lowercase().contains("arecrow");
            if !tagged && !named && radius.is_none() {
                continue;
            }
            let r = radius.unwrap_or(if info.data.name.contains("Deluxe") { DELUXE_SCARECROW_RADIUS } else { SCARECROW_RADIUS });
            out.push((*p, r));
        }
        out
    }

    pub fn scarecrow_covers(&self, location: &str, p: TilePos) -> bool {
        self.scarecrows(location).iter().any(|(s, r)| {
            let (dx, dy) = ((s.x - p.x) as f32, (s.y - p.y) as f32);
            (dx * dx + dy * dy).sqrt() < *r
        })
    }

    /// Night: crows raid unprotected crops on the farm.
    pub(crate) fn crows_overnight(&mut self) {
        self.crows.clear();
        let Some(farm) = self.locations.get("Farm") else { return };
        let crops: Vec<TilePos> = farm
            .state
            .terrain
            .iter()
            .filter_map(|(p, t)| match t {
                TerrainFeature::HoeDirt(d) if d.crop.as_ref().is_some_and(|c| !c.dead) => Some(*p),
                _ => None,
            })
            .collect();
        if crops.len() < CROPS_PER_CROW {
            return;
        }
        let mut rng = self.day_rng(91);
        let tries = (crops.len() / CROPS_PER_CROW).min(MAX_CROWS);
        let mut eaten = Vec::new();
        for _ in 0..tries {
            if !rng.chance(CROW_CHANCE) {
                continue;
            }
            let p = crops[rng.range(0, crops.len() as i32) as usize];
            if eaten.contains(&p) || self.scarecrow_covers("Farm", p) {
                continue;
            }
            eaten.push(p);
        }
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        for p in eaten {
            if let Some(TerrainFeature::HoeDirt(d)) = farm.state.terrain.get_mut(&p) {
                d.crop = None;
            }
            self.crows.push(Crow { location: "Farm".into(), pos: p.world_center(), flying: false, ms: rng.range(0, 1000) as u32, flip: rng.chance(0.5) });
        }
    }

    /// Crows peck until the farmer comes close, then fly away.
    pub(crate) fn update_crows(&mut self, dt: u32) {
        if self.crows.is_empty() {
            return;
        }
        let me = self.farmer.pos;
        let mut scared = false;
        for c in self.crows.iter_mut() {
            c.ms += dt;
            if !c.flying && c.location == self.current && c.pos.distance(me) < TILE_F * 3.0 {
                c.flying = true;
                c.ms = 0;
                scared = true;
            }
            if c.flying {
                let dir = if c.flip { -1.0 } else { 1.0 };
                c.pos.x += dir * 0.3 * dt as f32;
                c.pos.y -= 0.2 * dt as f32;
            }
        }
        self.crows.retain(|c| !(c.flying && c.ms > 4000));
        if scared {
            self.sound("crow");
        }
    }

    /// Pick tonight's event (if any) and apply it. Runs after the date has
    /// advanced and crops have grown.
    pub(crate) fn night_event_overnight(&mut self) -> Option<NightEvent> {
        if !self.locations.contains_key("Farm") {
            return None;
        }
        let quake_flag = "Port_Earthquake";
        if self.clock.season == Season::Summer && self.clock.day == 3 && self.clock.year == 1 && !self.farmer.mail_received.iter().any(|m| m == quake_flag) {
            self.farmer.mail_received.push(quake_flag.into());
            self.apply_cleared_tiles();
            return Some(NightEvent::Earthquake);
        }
        let mut rng = self.day_rng(97);
        let r = rng.next_f64();
        let ev = if r < 0.01 {
            (self.clock.season != Season::Winter).then_some(NightEvent::Fairy)
        } else if r < 0.02 {
            Some(NightEvent::Witch)
        } else if r < 0.03 {
            Some(NightEvent::Meteorite)
        } else if r < 0.035 {
            (self.clock.year > 1).then_some(NightEvent::Capsule)
        } else if r < 0.04 {
            Some(NightEvent::Owl)
        } else {
            None
        }?;
        let happened = match ev {
            NightEvent::Fairy => self.fairy_visit(&mut rng),
            NightEvent::Witch => {
                self.witch_visit(&mut rng);
                true
            }
            NightEvent::Meteorite => self.meteorite_lands(&mut rng),
            NightEvent::Owl => self.drop_on_farm(STONE_OWL, &mut rng),
            NightEvent::Capsule => self.drop_on_farm(STRANGE_CAPSULE, &mut rng),
            NightEvent::Earthquake => true,
        };
        happened.then_some(ev)
    }

    fn fairy_visit(&mut self, rng: &mut Rng) -> bool {
        let crops_data = self.items.crops.clone();
        let Some(farm) = self.locations.get_mut("Farm") else { return false };
        let growing: Vec<TilePos> = farm
            .state
            .terrain
            .iter()
            .filter_map(|(p, t)| match t {
                TerrainFeature::HoeDirt(d) => d.crop.as_ref().filter(|c| !c.dead && crops_data.get(&c.seed_id).is_some_and(|data| !c.is_ready(data))).map(|_| *p),
                _ => None,
            })
            .collect();
        if growing.is_empty() {
            return false;
        }
        let center = growing[rng.range(0, growing.len() as i32) as usize];
        for p in growing {
            let (dx, dy) = ((p.x - center.x) as f32, (p.y - center.y) as f32);
            if (dx * dx + dy * dy).sqrt() > FAIRY_RADIUS {
                continue;
            }
            if let Some(TerrainFeature::HoeDirt(d)) = farm.state.terrain.get_mut(&p) {
                if let Some(c) = d.crop.as_mut() {
                    if let Some(data) = crops_data.get(&c.seed_id) {
                        c.grow_fully(data);
                    }
                }
            }
        }
        true
    }

    /// The witch leaves a void egg in a coop that has animals.
    fn witch_visit(&mut self, rng: &mut Rng) {
        if self.items.object(VOID_EGG).is_none() {
            return;
        }
        let coops: Vec<String> = self
            .buildings
            .iter()
            .filter(|b| b.days_left == 0 && b.kind.contains("Coop"))
            .filter_map(|b| b.interior.clone())
            .filter(|i| self.animals.iter().any(|a| &a.home == i))
            .collect();
        if coops.is_empty() {
            return;
        }
        let name = &coops[rng.range(0, coops.len() as i32) as usize];
        let Some(loc) = self.locations.get_mut(name) else { return };
        let free: Vec<TilePos> = (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).filter(|p| loc.is_clear(*p) && crate::collision::tile_passable(loc, *p)).collect();
        if free.is_empty() {
            return;
        }
        let p = free[rng.range(0, free.len() as i32) as usize];
        loc.state.objects.insert(p, PlacedObject::new(VOID_EGG, false, ObjKind::Forage));
    }

    fn meteorite_lands(&mut self, rng: &mut Rng) -> bool {
        let Some(farm) = self.locations.get_mut("Farm") else { return false };
        let Some(p) = farm.free_area(2, 2, |n| rng.range(0, n as i32) as usize) else { return false };
        farm.state.clumps.push(crate::clumps::ResourceClump::new(crate::clumps::ClumpKind::Meteorite, p));
        true
    }

    fn drop_on_farm(&mut self, big_id: &str, rng: &mut Rng) -> bool {
        if self.items.big(big_id).is_none() {
            return false;
        }
        let Some(farm) = self.locations.get_mut("Farm") else { return false };
        let Some(p) = farm.free_area(1, 1, |n| rng.range(0, n as i32) as usize) else { return false };
        farm.state.objects.insert(p, PlacedObject::new(big_id, true, ObjKind::Placed));
        true
    }

    /// Weather a held object sets for tomorrow, if it is a weather totem.
    pub fn weather_totem(&self, id: &str) -> Option<Weather> {
        let spec = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.SetWeather")).cloned();
        let spec = spec.or_else(|| (id == RAIN_TOTEM).then(|| "Rain".to_string()))?;
        match spec.trim().to_ascii_lowercase().as_str() {
            "rain" => Some(Weather::Rain),
            "storm" => Some(Weather::Storm),
            "sun" => Some(Weather::Sun),
            "snow" => Some(Weather::Snow),
            "wind" => Some(Weather::Wind),
            _ => None,
        }
    }

    /// Use a weather totem from `slot`. True if the item was one.
    pub(crate) fn use_weather_totem(&mut self, slot: usize, id: &str) -> bool {
        let Some(weather) = self.weather_totem(id) else { return false };
        let mut tomorrow = self.clock.clone();
        tomorrow.next_day();
        let festival = self.festivals.contains_key(&format!("{}{}", tomorrow.season.key(), tomorrow.day));
        if festival {
            self.emit(WorldEvent::Message("The sky stays still. Tomorrow's weather can't be changed.".into()));
            return true;
        }
        self.farmer.inventory.reduce(slot, 1);
        self.weather_tomorrow = weather;
        self.sound("thunder");
        self.emit(WorldEvent::Flash);
        self.hud("The clouds stir... the weather will change tomorrow.");
        true
    }
}
