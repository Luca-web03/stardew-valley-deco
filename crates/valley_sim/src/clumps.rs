//! Resource clumps: large stumps, hollow logs and boulders that need an
//! upgraded tool, meteorites, mine boulders, and giant crops
//! (`Data/GiantCrops`). A clump covers several tiles (top-left `tile`).
//!
//! Sprites of the built-in kinds are 2x2 cells of the object sheet
//! (`Maps/springobjects`) at the publicly documented sheet indices. Health,
//! required upgrade levels, drops and XP are approximations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use valley_content::data::GiantCropData;
use valley_content::ContentManager;

use crate::farmer::Skill;
use crate::item::{Item, ToolKind};
use crate::location::GameLocation;
use crate::terrain::TerrainFeature;
use crate::world::{World, WorldEvent};
use crate::TilePos;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClumpKind {
    Stump,
    Log,
    Boulder,
    Meteorite,
    /// Mine boulder variant 0..=3.
    MineBoulder(u8),
    /// A giant crop (`Data/GiantCrops` key).
    GiantCrop(String),
}

impl ClumpKind {
    /// Object sheet index of the 2x2 sprite's top-left cell.
    pub fn sheet_index(&self) -> Option<i32> {
        Some(match self {
            ClumpKind::Stump => 600,
            ClumpKind::Log => 602,
            ClumpKind::Meteorite => 622,
            ClumpKind::Boulder => 672,
            ClumpKind::MineBoulder(v) => 752 + 2 * (*v as i32 % 4),
            ClumpKind::GiantCrop(_) => return None,
        })
    }

    pub fn tool(&self) -> ToolKind {
        match self {
            ClumpKind::Stump | ClumpKind::Log | ClumpKind::GiantCrop(_) => ToolKind::Axe,
            _ => ToolKind::Pickaxe,
        }
    }

    /// Lowest tool upgrade level that can damage it.
    pub fn min_level(&self) -> i32 {
        match self {
            ClumpKind::Stump => 1,
            ClumpKind::Log | ClumpKind::Boulder => 2,
            ClumpKind::Meteorite => 3,
            ClumpKind::MineBoulder(_) | ClumpKind::GiantCrop(_) => 0,
        }
    }

    pub fn base_health(&self) -> i32 {
        match self {
            ClumpKind::Stump | ClumpKind::Boulder => 8,
            ClumpKind::Log | ClumpKind::Meteorite => 12,
            ClumpKind::MineBoulder(_) => 4,
            ClumpKind::GiantCrop(_) => 3,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ClumpKind::Stump => "stump",
            ClumpKind::Log => "log",
            ClumpKind::Boulder | ClumpKind::MineBoulder(_) => "boulder",
            ClumpKind::Meteorite => "meteorite",
            ClumpKind::GiantCrop(_) => "giant crop",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceClump {
    pub kind: ClumpKind,
    pub tile: TilePos,
    pub size: (i32, i32),
    pub health: i32,
    #[serde(skip)]
    pub shake: i32,
}

impl ResourceClump {
    pub fn new(kind: ClumpKind, tile: TilePos) -> Self {
        let health = kind.base_health();
        Self { kind, tile, size: (2, 2), health, shake: 0 }
    }

    pub fn covers(&self, p: TilePos) -> bool {
        p.x >= self.tile.x && p.y >= self.tile.y && p.x < self.tile.x + self.size.0 && p.y < self.tile.y + self.size.1
    }

    pub fn tiles(&self) -> Vec<TilePos> {
        (0..self.size.1).flat_map(|dy| (0..self.size.0).map(move |dx| TilePos::new(self.tile.x + dx, self.tile.y + dy))).collect()
    }
}

pub fn load_giant_crops(cm: &mut ContentManager) -> BTreeMap<String, GiantCropData> {
    cm.load_dict("Data/GiantCrops")
}

impl GameLocation {
    pub fn clump_at(&self, p: TilePos) -> Option<usize> {
        self.state.clumps.iter().position(|c| c.covers(p))
    }

    /// Top-left of a free `w`x`h` area (clear, diggable tiles) picked by
    /// `pick` among all candidates.
    pub fn free_area(&self, w: i32, h: i32, mut pick: impl FnMut(usize) -> usize) -> Option<TilePos> {
        let mut spots = Vec::new();
        for y in 0..self.height() - h {
            for x in 0..self.width() - w {
                let ok = (0..h).all(|dy| (0..w).all(|dx| {
                    let p = TilePos::new(x + dx, y + dy);
                    self.is_clear(p) && self.is_diggable(p)
                }));
                if ok {
                    spots.push(TilePos::new(x, y));
                }
            }
        }
        if spots.is_empty() {
            None
        } else {
            Some(spots[pick(spots.len())])
        }
    }
}

impl World {
    /// Hit the clump at `t` with a tool; true if there was one.
    pub(crate) fn hit_clump(&mut self, t: TilePos, kind: ToolKind, level: i32) -> bool {
        let Some(i) = self.location().clump_at(t) else { return false };
        let clump = self.location().state.clumps[i].clone();
        if clump.kind.tool() != kind {
            return false;
        }
        if level < clump.kind.min_level() {
            self.location_mut().state.clumps[i].shake = 12;
            self.sound(if kind == ToolKind::Axe { "axe" } else { "clank" });
            let tool = if kind == ToolKind::Axe { "axe" } else { "pickaxe" };
            self.emit(WorldEvent::Message(format!("This {} is too tough for your {tool}. An upgraded {tool} could break it.", clump.kind.name())));
            return true;
        }
        let damage = level + 1;
        {
            let c = &mut self.location_mut().state.clumps[i];
            c.health -= damage;
            c.shake = 12;
        }
        if self.location().state.clumps[i].health > 0 {
            self.sound(if kind == ToolKind::Axe { "axchop" } else { "hammer" });
            return true;
        }
        let c = self.location_mut().state.clumps.remove(i);
        self.sound(if kind == ToolKind::Axe { "stumpCrack" } else { "boulderBreak" });
        let center = engine::Vec2::new((c.tile.x as f32 + c.size.0 as f32 / 2.0) * crate::TILE_F, (c.tile.y as f32 + c.size.1 as f32 / 2.0) * crate::TILE_F);
        let (drops, skill, xp) = self.clump_drops(&c.kind);
        for item in drops {
            let mut left = item.stack();
            while left > 0 {
                let n = left.min(5);
                left -= n;
                let mut piece = item.clone();
                piece.set_stack(n);
                self.spawn_drop(piece, center);
            }
        }
        if let Some(l) = self.farmer.skills.gain(skill, xp) {
            self.emit(WorldEvent::LevelUp(skill, l));
        }
        true
    }

    /// Items, skill and XP for breaking a clump.
    fn clump_drops(&mut self, kind: &ClumpKind) -> (Vec<Item>, Skill, i32) {
        let mut rng = self.rng.clone();
        let out = match kind {
            ClumpKind::Stump => (vec![Item::object("709", 2)], Skill::Foraging, 25),
            ClumpKind::Log => (vec![Item::object("709", 8)], Skill::Foraging, 25),
            ClumpKind::Boulder => (vec![Item::object("390", 15)], Skill::Mining, 15),
            ClumpKind::MineBoulder(_) => (vec![Item::object("390", rng.range(8, 12))], Skill::Mining, 10),
            ClumpKind::Meteorite => {
                let mut v = vec![Item::object("386", 6), Item::object("390", 6)];
                if self.items.object("537").is_some() {
                    v.push(Item::object("537", 2));
                }
                (v, Skill::Mining, 25)
            }
            ClumpKind::GiantCrop(key) => {
                let mut v = Vec::new();
                if let Some(d) = self.giant_crops.get(key).cloned() {
                    for h in &d.harvest_items {
                        if !rng.chance(h.chance) || !self.gsq_ok(h.condition.as_deref()) {
                            continue;
                        }
                        let max = if h.max_stack < h.min_stack { h.min_stack } else { h.max_stack };
                        let n = rng.range(h.min_stack, max + 1).max(1);
                        if let Some(it) = self.items.create(&h.item_id, n) {
                            v.push(it);
                        }
                    }
                }
                (v, Skill::Farming, 0)
            }
        };
        self.rng = rng;
        out
    }

    /// Night: fully grown crops filling a giant crop's footprint may merge.
    pub(crate) fn giant_crops_overnight(&mut self) {
        if self.giant_crops.is_empty() {
            return;
        }
        let mut rng = self.day_rng(83);
        let giants: Vec<(String, GiantCropData)> = self.giant_crops.iter().filter(|(_, d)| self.gsq_ok(d.condition.as_deref())).map(|(k, d)| (k.clone(), d.clone())).collect();
        let crops = self.items.crops.clone();
        for loc in self.locations.values_mut() {
            if !loc.outdoors {
                continue;
            }
            // Fully grown crop at each tile -> its harvest item id.
            let grown: BTreeMap<TilePos, String> = loc
                .state
                .terrain
                .iter()
                .filter_map(|(p, t)| match t {
                    TerrainFeature::HoeDirt(d) => {
                        let c = d.crop.as_ref().filter(|c| !c.dead)?;
                        let data = crops.get(&c.seed_id)?;
                        c.is_ready(data).then(|| (*p, crate::item::split_qualified(&data.harvest_item_id).1.to_string()))
                    }
                    _ => None,
                })
                .collect();
            let mut used: Vec<TilePos> = Vec::new();
            for (p, item) in &grown {
                for (key, d) in &giants {
                    if crate::item::split_qualified(&d.from_item_id).1 != item {
                        continue;
                    }
                    let (w, h) = (d.tile_size.x.max(1), d.tile_size.y.max(1));
                    let area: Vec<TilePos> = (0..h).flat_map(|dy| (0..w).map(move |dx| TilePos::new(p.x + dx, p.y + dy))).collect();
                    if !area.iter().all(|q| grown.get(q) == Some(item) && !used.contains(q)) {
                        continue;
                    }
                    if !rng.chance(d.chance) {
                        continue;
                    }
                    for q in &area {
                        if let Some(TerrainFeature::HoeDirt(dirt)) = loc.state.terrain.get_mut(q) {
                            dirt.crop = None;
                        }
                    }
                    used.extend(area);
                    let mut c = ResourceClump::new(ClumpKind::GiantCrop(key.clone()), *p);
                    c.size = (w, h);
                    c.health = d.health.max(1);
                    loc.state.clumps.push(c);
                    break;
                }
            }
        }
    }

    /// New farm: a few stumps, logs and boulders (approx. counts).
    pub(crate) fn place_farm_clumps(&mut self) {
        let mut rng = engine::Rng::new(engine::Rng::seed_from(&[self.seed as i64, 79]));
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        if farm.map.prop("Port.NoClumps").is_some() {
            return;
        }
        let area = farm.width() * farm.height();
        let n = (area / 600).clamp(2, 14);
        let protected: Vec<TilePos> = ["FarmHouseEntry", "ShippingBinTile"].iter().filter_map(|k| farm.map_prop_tile(k)).collect();
        for i in 0..n {
            let kind = match i % 4 {
                0 | 2 => ClumpKind::Stump,
                1 => ClumpKind::Boulder,
                _ => ClumpKind::Log,
            };
            let Some(p) = farm.free_area(2, 2, |len| rng.range(0, len as i32) as usize) else { break };
            if protected.iter().any(|q| q.manhattan(p) <= 5) {
                continue;
            }
            farm.state.clumps.push(ResourceClump::new(kind, p));
        }
    }

    pub(crate) fn animate_clumps(&mut self) {
        for loc in self.locations.values_mut() {
            for c in loc.state.clumps.iter_mut() {
                c.shake = (c.shake - 1).max(0);
            }
        }
    }
}
