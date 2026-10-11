//! Farm map choices. Each type uses its own farm map and, when present,
//! its `Data/Locations` entry `Farm_<suffix>` (fish, forage, artifact
//! spots). `Data/AdditionalFarms` entries (`Id`, `MapName`,
//! `TooltipStringPath`) add more choices. Type perks (approximations of
//! the original's code):
//!
//! * Hill-top: stones and ore nodes appear each morning in the mining area
//!   (`Port.MiningArea "x y w h"` map property, else the farm's top-left
//!   quarter).
//! * Wilderness: monsters roam the farm at night.
//! * Meadowlands: starts with a coop and two of its animals.

use engine::Rng;
use serde::Deserialize;
use valley_content::data::LocationData;
use valley_content::ContentManager;

use crate::combat::Monster;
use crate::location::GameLocation;
use crate::mines::RockKind;
use crate::objects::{ObjKind, PlacedObject};
use crate::world::World;
use crate::TilePos;

/// (id, display name, map asset, Data/Locations key, description).
pub const FARM_TYPES: &[(&str, &str, &str, &str, &str)] = &[
    ("Standard", "Standard", "Maps/Farm", "Farm_Standard", "Plenty of open land for crops."),
    ("Riverland", "Riverland", "Maps/Farm_Fishing", "Farm_Fishing", "Islands and rivers; good fishing, less farmland."),
    ("Forest", "Forest", "Maps/Farm_Foraging", "Farm_Foraging", "Wild forage and big stumps to clear."),
    ("Hilltop", "Hill-top", "Maps/Farm_Mining", "Farm_Mining", "A quarry where stones and ore appear."),
    ("Wilderness", "Wilderness", "Maps/Farm_Combat", "Farm_Combat", "Monsters come out at night."),
    ("FourCorners", "Four Corners", "Maps/Farm_FourCorners", "Farm_FourCorners", "Four regions in one."),
    ("Beach", "Beach", "Maps/Farm_Island", "Farm_Island", "Sand and surf; rare forage washes up."),
    ("Meadowlands", "Meadowlands", "Maps/Farm_Ranching", "Farm_Ranching", "Starts with a coop and two hens."),
];

/// The farm type id for a numeric or map-style name ("1", "Fishing").
pub fn farm_type_alias(name: &str) -> Option<&'static str> {
    if let Ok(i) = name.parse::<usize>() {
        return FARM_TYPES.get(i).map(|t| t.0);
    }
    FARM_TYPES.iter().find(|t| t.3.trim_start_matches("Farm_").eq_ignore_ascii_case(name) || t.0.eq_ignore_ascii_case(name)).map(|t| t.0)
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct AdditionalFarm {
    #[serde(rename = "ID", alias = "Id")]
    pub id: String,
    #[serde(rename = "MapName")]
    pub map_name: String,
    #[serde(rename = "TooltipStringPath")]
    pub tooltip_string_path: Option<String>,
}

/// A farm choice for the new-game screen.
#[derive(Clone, Debug, PartialEq)]
pub struct FarmChoice {
    pub id: String,
    pub name: String,
    pub description: String,
    pub map: String,
}

/// Farm types whose maps exist in the content (Standard always).
pub fn available(cm: &mut ContentManager) -> Vec<FarmChoice> {
    let mut v: Vec<FarmChoice> = FARM_TYPES
        .iter()
        .filter(|(id, _, map, _, _)| *id == "Standard" || cm.exists(map) || cm.exists(&format!("{map}.tmx")) || cm.exists(&format!("{map}.tbin")))
        .map(|(id, name, map, _, desc)| FarmChoice { id: id.to_string(), name: name.to_string(), description: desc.to_string(), map: map.to_string() })
        .collect();
    let extra: Vec<AdditionalFarm> = cm.load_json("Data/AdditionalFarms").unwrap_or_default();
    for f in extra {
        if f.id.is_empty() || f.map_name.is_empty() {
            continue;
        }
        let (name, description) = f
            .tooltip_string_path
            .as_deref()
            .map(|p| cm.expand(&format!("[LocalizedText {p}]")))
            .filter(|s| !s.is_empty() && !s.starts_with('['))
            .map(|s| {
                let mut it = s.splitn(2, '_');
                (it.next().unwrap_or("").to_string(), it.next().unwrap_or("").to_string())
            })
            .unwrap_or((f.id.clone(), String::new()));
        v.push(FarmChoice { id: f.id.clone(), name, description, map: format!("Maps/{}", f.map_name) });
    }
    v
}

impl World {
    fn farm_type_entry(&self) -> Option<&'static (&'static str, &'static str, &'static str, &'static str, &'static str)> {
        FARM_TYPES.iter().find(|t| t.0 == self.farmer.farm_type)
    }

    pub fn farm_type_is(&self, id: &str) -> bool {
        self.farmer.farm_type == id || (self.farmer.farm_type.is_empty() && id == "Standard")
    }

    /// `Data/Locations` entry for a location, honouring the farm type.
    pub fn location_data_for(&self, name: &str) -> Option<&LocationData> {
        if name == "Farm" {
            if let Some(e) = self.farm_type_entry() {
                if let Some(d) = self.location_data.get(e.3) {
                    return Some(d);
                }
            }
        }
        self.location_data.get(name)
    }

    /// Swap in the chosen farm type's map (new games and loading).
    pub fn apply_farm_type(&mut self, cm: &mut ContentManager) {
        if self.farm_type_is("Standard") {
            return;
        }
        let map_asset = match self.farm_type_entry() {
            Some(e) => e.2.to_string(),
            None => {
                let extra: Vec<AdditionalFarm> = cm.load_json("Data/AdditionalFarms").unwrap_or_default();
                match extra.into_iter().find(|f| f.id == self.farmer.farm_type) {
                    Some(f) => format!("Maps/{}", f.map_name),
                    None => return,
                }
            }
        };
        let Some(map) = cm.load_map(&map_asset) else { return };
        let display = self.locations.get("Farm").map(|l| l.display_name.clone()).unwrap_or_else(|| "Farm".into());
        let mut loc = GameLocation::new("Farm", map);
        loc.display_name = display;
        self.locations.insert("Farm".into(), loc);
        self.build_location_edges_pub();
    }

    /// Meadowlands: a free coop with two of its animals.
    pub(crate) fn farm_type_start(&mut self) {
        if !self.farm_type_is("Meadowlands") {
            return;
        }
        let coop = self.buildings_data.iter().find(|(_, d)| d.valid_occupant_types.iter().any(|t| t == "Coop") && d.indoor_map.is_some()).map(|(k, d)| (k.clone(), (d.size.x.max(1), d.size.y.max(1))));
        let Some((kind, size)) = coop else { return };
        let Some(farm) = self.locations.get("Farm") else { return };
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, 173]));
        let Some(spot) = farm.free_area(size.0, size.1, |n| rng.range(0, n as i32) as usize) else { return };
        if self.grant_building(&kind, spot).is_err() {
            return;
        }
        if let Some(a) = self.animals_data.iter().find(|(_, a)| a.house == "Coop").map(|(k, _)| k.clone()) {
            for _ in 0..2 {
                let _ = self.add_animal(&a, None, false);
            }
        }
    }

    /// Hill-top: stones and ore nodes in the mining area each morning.
    pub(crate) fn farm_type_morning(&mut self) {
        if !self.farm_type_is("Hilltop") {
            return;
        }
        let mi = self.mine_items.clone();
        let mut rng = self.day_rng(179);
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        let (x0, y0, w, h) = match farm.map.prop("Port.MiningArea") {
            Some(s) => {
                let v: Vec<i32> = s.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if v.len() >= 4 { (v[0], v[1], v[2], v[3]) } else { (0, 0, farm.width() / 2, farm.height() / 2) }
            }
            None => (0, 0, farm.width() / 2, farm.height() / 2),
        };
        let spots: Vec<TilePos> = (y0..y0 + h).flat_map(|y| (x0..x0 + w).map(move |x| TilePos::new(x, y))).filter(|p| farm.is_clear(*p) && crate::collision::tile_passable(farm, *p)).collect();
        if spots.is_empty() {
            return;
        }
        for _ in 0..rng.range(4, 9) {
            let p = spots[rng.range(0, spots.len() as i32) as usize];
            if farm.state.objects.contains_key(&p) {
                continue;
            }
            let roll = rng.next_f64();
            let (kind, sprite) = if roll < 0.08 {
                (RockKind::Iron, mi.iron_node.clone())
            } else if roll < 0.2 {
                (RockKind::Copper, mi.copper_node.clone())
            } else {
                (RockKind::Stone, None)
            };
            let Some(sprite) = sprite.or_else(|| rng.pick(&mi.rock_sprites).cloned()) else { continue };
            let health = match kind {
                RockKind::Stone => 1,
                RockKind::Copper => 3,
                _ => 4,
            };
            farm.state.objects.insert(p, PlacedObject::new(&sprite, false, ObjKind::MineRock { rock: kind, health }));
        }
    }

    /// Wilderness: every ten minutes after dusk a monster may appear on
    /// the farm (up to six at a time).
    pub(crate) fn farm_type_night(&mut self) {
        if !self.farm_type_is("Wilderness") || self.clock.time < 1900 {
            return;
        }
        if self.monsters.iter().filter(|m| m.location == "Farm").count() >= 6 {
            return;
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, self.clock.time as i64, 181]));
        if !rng.chance(0.3) {
            return;
        }
        let names: Vec<String> = ["Green Slime", "Bat", "Fly", "Shadow Brute", "Dev_Blob", "Dev_Moth"].iter().filter(|n| self.monster_info.contains_key(**n)).map(|s| s.to_string()).collect();
        let Some(name) = rng.pick(&names).cloned() else { return };
        let me = self.farmer.tile();
        let Some(farm) = self.locations.get("Farm") else { return };
        let spots: Vec<TilePos> = (0..farm.height()).flat_map(|y| (0..farm.width()).map(move |x| TilePos::new(x, y))).filter(|p| p.manhattan(me) > 8 && farm.is_clear(*p) && crate::collision::tile_passable(farm, *p)).collect();
        let Some(tile) = rng.pick(&spots).copied() else { return };
        let info = self.monster_info[&name].clone();
        self.next_monster_id += 1;
        self.monsters.push(Monster::new(self.next_monster_id, info, "Farm", tile));
    }
}
