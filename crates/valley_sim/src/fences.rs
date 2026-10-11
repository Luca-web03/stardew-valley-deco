//! Fences, gates, floors and paths (`Data/Fences`, `Data/FloorsAndPaths`;
//! public format subsets).
//!
//! A fence is a placed object that joins its fence neighbours, blocks the
//! way and wears down by a point of health a day outdoors; at zero it falls
//! over (passable) until removed. The tool types in `RemovalToolTypes`
//! take a standing fence back as its item. Gates (well-known `325`, or a
//! `Port.Gate` custom field) open and close with the action button.
//!
//! Floors and paths are terrain placed from their `ItemId`; they join
//! their neighbours, can carry objects, give a small speed bonus on the
//! farm (`FarmSpeedBuff`, default 0.1; approx.) and come back up with a
//! pickaxe or axe.

use std::collections::BTreeMap;

use serde::Deserialize;
use valley_content::data::Point;

use crate::item::{Item, ToolKind};
use crate::objects::{ObjKind, PlacedObject};
use crate::terrain::TerrainFeature;
use crate::world::World;
use crate::TilePos;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct FenceData {
    pub health: i32,
    pub repair_health_adjustment_minimum: f32,
    pub repair_health_adjustment_maximum: f32,
    pub texture: String,
    pub placement_sound: Option<String>,
    pub removal_sound: Option<String>,
    pub removal_tool_ids: Vec<String>,
    pub removal_tool_types: Vec<String>,
}

impl Default for FenceData {
    fn default() -> Self {
        Self {
            health: 28,
            repair_health_adjustment_minimum: 0.0,
            repair_health_adjustment_maximum: 0.0,
            texture: String::new(),
            placement_sound: None,
            removal_sound: None,
            removal_tool_ids: Vec::new(),
            removal_tool_types: vec!["Axe".into()],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct FloorPathData {
    pub id: Option<String>,
    pub item_id: String,
    pub texture: String,
    pub corner: Point,
    pub winter_texture: Option<String>,
    pub placement_sound: Option<String>,
    pub removal_sound: Option<String>,
    pub footstep_sound: Option<String>,
    pub farm_speed_buff: Option<f32>,
}

/// Well-known gate ids.
const GATES: &[&str] = &["325", "Dev_Gate"];
/// Speed bonus on farm paths without a `FarmSpeedBuff` (approx.).
pub const PATH_SPEED: f32 = 0.1;

pub fn load_fences(cm: &mut valley_content::ContentManager) -> BTreeMap<String, FenceData> {
    cm.load_dict("Data/Fences")
}

pub fn load_floors(cm: &mut valley_content::ContentManager) -> BTreeMap<String, FloorPathData> {
    cm.load_dict("Data/FloorsAndPaths")
}

/// Neighbour mask (N=1, E=2, S=4, W=8) of tiles for which `is` holds.
pub fn neighbour_mask(t: TilePos, is: impl Fn(TilePos) -> bool) -> i32 {
    let mut m = 0;
    for (bit, (dx, dy)) in [(1, (0, -1)), (2, (1, 0)), (4, (0, 1)), (8, (-1, 0))] {
        if is(TilePos::new(t.x + dx, t.y + dy)) {
            m |= bit;
        }
    }
    m
}

impl World {
    pub fn is_gate(&self, id: &str) -> bool {
        GATES.contains(&id) || self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).is_some_and(|f| f.contains_key("Port.Gate"))
    }

    /// Fence data for an object id (gates fall back to defaults).
    pub fn fence_data(&self, id: &str) -> Option<FenceData> {
        self.fences.get(id).cloned().or_else(|| self.is_gate(id).then(FenceData::default))
    }

    /// The floor/path entry an item places.
    pub fn floor_for_item(&self, item_id: &str) -> Option<String> {
        self.floors.iter().find(|(_, f)| crate::item::split_qualified(&f.item_id).1 == item_id).map(|(k, _)| k.clone())
    }

    pub(crate) fn fence_object(&self, id: &str) -> Option<PlacedObject> {
        let d = self.fence_data(id)?;
        let mut rng = engine::Rng::new(engine::Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 6161]));
        let lo = d.repair_health_adjustment_minimum;
        let hi = d.repair_health_adjustment_maximum.max(lo);
        let extra = lo + (hi - lo) * rng.next_f64() as f32;
        let health = (d.health as f32 + extra).round().max(1.0) as i32;
        Some(PlacedObject::new(id, false, ObjKind::Fence { health, open: false }))
    }

    /// Place a floor or path from the held item. True if it was one.
    pub(crate) fn place_floor(&mut self, t: TilePos, item_id: &str) -> Option<bool> {
        let key = self.floor_for_item(item_id)?;
        let loc = self.location();
        let free = loc.in_bounds(t)
            && loc.has_tile("Back", t)
            && !loc.has_tile("Buildings", t)
            && !loc.is_water(t)
            && !loc.state.terrain.contains_key(&t)
            && crate::collision::tile_passable(loc, t);
        if !free {
            return Some(false);
        }
        self.location_mut().state.terrain.insert(t, TerrainFeature::Flooring(key.clone()));
        self.farmer.inventory.reduce(self.farmer.current_slot, 1);
        let cue = self.floors.get(&key).and_then(|f| f.placement_sound.clone()).unwrap_or_else(|| "thudStep".into());
        self.sound(&cue);
        Some(true)
    }

    /// A tool hits a fence: tools of a removal type take it back.
    pub(crate) fn hit_fence(&mut self, t: TilePos, kind: ToolKind) -> bool {
        let Some(o) = self.location().object(t).cloned() else { return false };
        let ObjKind::Fence { health, .. } = o.kind else { return false };
        let d = self.fence_data(&o.item_id).unwrap_or_default();
        let type_ok = d.removal_tool_types.iter().any(|ty| ty.eq_ignore_ascii_case(kind.data_key()) || ty.eq_ignore_ascii_case(&format!("{kind:?}")));
        let id_ok = d.removal_tool_ids.iter().any(|id| crate::item::split_qualified(id).1 == kind.data_key());
        if !(type_ok || id_ok) && health > 0 {
            return false;
        }
        self.location_mut().state.objects.remove(&t);
        let cue = d.removal_sound.unwrap_or_else(|| "axchop".into());
        self.sound(&cue);
        if health > 0 {
            self.spawn_drop(Item::object(&o.item_id, 1), t.world_center());
        }
        true
    }

    /// A pickaxe or axe lifts a floor or path.
    pub(crate) fn hit_floor(&mut self, t: TilePos) -> bool {
        let Some(TerrainFeature::Flooring(key)) = self.location().terrain(t).cloned() else { return false };
        if self.location().object(t).is_some() {
            return false;
        }
        self.location_mut().state.terrain.remove(&t);
        let data = self.floors.get(&key).cloned().unwrap_or_default();
        self.sound(data.removal_sound.as_deref().unwrap_or("hammer"));
        let id = crate::item::split_qualified(&data.item_id).1.to_string();
        if self.items.objects.contains_key(&id) {
            self.spawn_drop(Item::object(&id, 1), t.world_center());
        }
        true
    }

    /// Open or close a gate.
    pub(crate) fn toggle_gate(&mut self, t: TilePos) -> bool {
        let Some(id) = self.location().object(t).filter(|o| matches!(o.kind, ObjKind::Fence { .. })).map(|o| o.item_id.clone()) else { return false };
        if !self.is_gate(&id) {
            return false;
        }
        let farmer = self.farmer.tile();
        let Some(ObjKind::Fence { open, .. }) = self.location_mut().state.objects.get_mut(&t).map(|o| &mut o.kind) else { return false };
        // A gate can't close on the farmer.
        if *open && farmer == t {
            return true;
        }
        *open = !*open;
        let cue = if *open { "doorOpen" } else { "doorClose" };
        self.sound(cue);
        true
    }

    /// Overnight: outdoor fences wear down; at zero they fall over. Gates
    /// keep standing.
    pub(crate) fn fences_overnight(&mut self) {
        let gates: Vec<String> = self.locations.values().flat_map(|l| l.state.objects.values()).filter(|o| matches!(o.kind, ObjKind::Fence { .. }) && self.is_gate(&o.item_id)).map(|o| o.item_id.clone()).collect();
        for loc in self.locations.values_mut().filter(|l| l.outdoors) {
            for o in loc.state.objects.values_mut() {
                if let ObjKind::Fence { health, .. } = &mut o.kind {
                    if *health > 0 && !gates.contains(&o.item_id) {
                        *health -= 1;
                    }
                }
            }
        }
    }

    /// Speed bonus from a floor or path under the farmer on the farm.
    pub fn floor_speed_bonus(&self) -> f32 {
        if !self.current.starts_with("Farm") || self.current.starts_with("FarmHouse") || self.current.starts_with("FarmCave") {
            return 0.0;
        }
        match self.location().terrain(self.farmer.tile()) {
            Some(TerrainFeature::Flooring(key)) => {
                let b = self.floors.get(key).and_then(|f| f.farm_speed_buff).unwrap_or(-1.0);
                if b < 0.0 {
                    PATH_SPEED
                } else {
                    b
                }
            }
            _ => 0.0,
        }
    }

    /// Footstep cue of the floor under the farmer.
    pub fn floor_footstep(&self) -> Option<String> {
        match self.location().terrain(self.farmer.tile()) {
            Some(TerrainFeature::Flooring(key)) => self.floors.get(key).and_then(|f| f.footstep_sound.clone()),
            _ => None,
        }
    }
}
