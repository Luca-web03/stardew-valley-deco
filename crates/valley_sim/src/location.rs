use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use valley_content::props::PropsExt;
use valley_content::Map;

use crate::objects::PlacedObject;
use crate::terrain::TerrainFeature;
use crate::TilePos;

/// A warp area: stepping onto `from` sends the farmer to `to` in `target`.
#[derive(Clone, Debug, PartialEq)]
pub struct Warp {
    pub from: TilePos,
    pub target: String,
    pub to: TilePos,
}

/// Parse a map's `Warp` property ("x y Target tx ty ...").
pub fn parse_warps(s: &str) -> Vec<Warp> {
    let toks: Vec<&str> = s.split_whitespace().collect();
    toks.chunks(5)
        .filter_map(|c| {
            if c.len() < 5 {
                return None;
            }
            Some(Warp {
                from: TilePos::new(c[0].parse().ok()?, c[1].parse().ok()?),
                target: c[2].to_string(),
                to: TilePos::new(c[3].parse().ok()?, c[4].parse().ok()?),
            })
        })
        .collect()
}

/// Mutable per-location state that is saved (map comes from content).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct LocationState {
    #[serde(with = "tile_map")]
    pub terrain: BTreeMap<TilePos, TerrainFeature>,
    #[serde(with = "tile_map")]
    pub objects: BTreeMap<TilePos, PlacedObject>,
}

/// JSON object keys must be strings, so tile-keyed maps are stored as
/// `[[{x,y}, value], ...]` lists.
pub mod tile_map {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::TilePos;

    pub fn serialize<T: Serialize, S: Serializer>(m: &BTreeMap<TilePos, T>, s: S) -> Result<S::Ok, S::Error> {
        let v: Vec<(&TilePos, &T)> = m.iter().collect();
        v.serialize(s)
    }

    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<TilePos, T>, D::Error> {
        let v: Vec<(TilePos, T)> = Vec::deserialize(d)?;
        Ok(v.into_iter().collect())
    }
}

pub struct GameLocation {
    pub name: String,
    pub map: Map,
    pub state: LocationState,
    pub warps: Vec<Warp>,
    pub outdoors: bool,
    pub is_farm: bool,
    pub display_name: String,
    /// Tiles blocked at runtime (building footprints).
    pub blocked: std::collections::HashSet<TilePos>,
}

impl GameLocation {
    pub fn new(name: &str, map: Map) -> Self {
        let warps = map.prop("Warp").map(|w| parse_warps(&w)).unwrap_or_default();
        let outdoors = match map.props.str("Outdoors") {
            Some(v) => valley_content::PropValue::Str(v).truthy(),
            None => map.layer("Back").map(|_| map.sheets.iter().any(|s| is_seasonal(&s.image))).unwrap_or(false),
        };
        Self {
            name: name.to_string(),
            is_farm: name == "Farm" || name.starts_with("Farm_"),
            map,
            state: LocationState::default(),
            warps,
            outdoors,
            display_name: name.to_string(),
            blocked: Default::default(),
        }
    }

    pub fn width(&self) -> i32 {
        self.map.width
    }
    pub fn height(&self) -> i32 {
        self.map.height
    }

    pub fn in_bounds(&self, p: TilePos) -> bool {
        p.x >= 0 && p.y >= 0 && p.x < self.map.width && p.y < self.map.height
    }

    pub fn tile_prop(&self, layer: &str, p: TilePos, name: &str) -> Option<String> {
        self.map.tile_prop(layer, p.x, p.y, name)
    }

    pub fn has_tile(&self, layer: &str, p: TilePos) -> bool {
        self.map.tile(layer, p.x, p.y).is_some()
    }

    pub fn is_water(&self, p: TilePos) -> bool {
        self.tile_prop("Back", p, "Water").is_some_and(|v| truthy(&v))
    }

    pub fn is_diggable(&self, p: TilePos) -> bool {
        self.tile_prop("Back", p, "Diggable").is_some_and(|v| truthy(&v))
            && !self.has_tile("Buildings", p)
    }

    /// Action string on the Buildings layer (e.g. "Warp 5 8 FarmHouse").
    pub fn action_at(&self, p: TilePos) -> Option<String> {
        self.tile_prop("Buildings", p, "Action")
    }

    pub fn touch_action_at(&self, p: TilePos) -> Option<String> {
        self.tile_prop("Back", p, "TouchAction")
    }

    /// Footstep/tool sound type of the ground ("Dirt", "Grass", "Stone", "Wood", ...).
    pub fn ground_type(&self, p: TilePos) -> Option<String> {
        self.tile_prop("Back", p, "Type")
    }

    pub fn terrain(&self, p: TilePos) -> Option<&TerrainFeature> {
        self.state.terrain.get(&p)
    }

    pub fn terrain_mut(&mut self, p: TilePos) -> Option<&mut TerrainFeature> {
        self.state.terrain.get_mut(&p)
    }

    pub fn object(&self, p: TilePos) -> Option<&PlacedObject> {
        self.state.objects.get(&p)
    }

    /// Free for placing something (no object, terrain feature, building tile, water).
    pub fn is_clear(&self, p: TilePos) -> bool {
        !self.blocked.contains(&p)
            && self.in_bounds(p)
            && self.has_tile("Back", p)
            && !self.has_tile("Buildings", p)
            && !self.is_water(p)
            && !self.state.objects.contains_key(&p)
            && !self.state.terrain.contains_key(&p)
    }

    pub fn map_prop_tile(&self, name: &str) -> Option<TilePos> {
        let s = self.map.prop(name)?;
        let mut it = s.split_whitespace();
        Some(TilePos::new(it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    }
}

pub fn truthy(v: &str) -> bool {
    valley_content::PropValue::Str(v.to_string()).truthy()
}

/// Tilesheet images that follow the seasonal naming convention.
pub fn is_seasonal(image: &str) -> bool {
    let lower = image.to_ascii_lowercase();
    ["spring_", "summer_", "fall_", "winter_"].iter().any(|p| lower.starts_with(p))
}

/// Swap the season prefix of a tilesheet image name.
pub fn seasonal_image(image: &str, season: &str) -> String {
    let lower = image.to_ascii_lowercase();
    for p in ["spring_", "summer_", "fall_", "winter_"] {
        if lower.starts_with(p) {
            return format!("{season}_{}", &image[p.len()..]);
        }
    }
    image.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warps_and_seasons() {
        let w = parse_warps("48 13 Town 1 15 -1 16 Farm 46 14 bad");
        assert_eq!(w.len(), 2);
        assert_eq!(w[1].from, TilePos::new(-1, 16));
        assert_eq!(seasonal_image("spring_outdoorsTileSheet", "fall"), "fall_outdoorsTileSheet");
        assert_eq!(seasonal_image("townInterior", "fall"), "townInterior");
    }
}
