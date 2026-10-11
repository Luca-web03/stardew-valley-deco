//! Bushes (public behaviour; amounts approx.) and map `Paths` layer spawns.
//!
//! Bushes come in small (1 tile wide), medium (2) and large (3) sizes, plus
//! tea bushes. Medium bushes carry berries in their seasons (spring 15-18,
//! fall 8-11); shaking one drops them. Tea bushes grow from a tea sapling
//! in 20 days and give tea leaves on days 22-28 of each season (not in
//! winter outdoors). Farm bushes can be cut with a good enough axe (small:
//! any, medium: copper, large: steel; approx.).
//!
//! The `Paths` map layer marks spawn points: tile indices of sheets whose
//! image name contains a key of `Port/PathsLayer` map to "Tree <type>" or
//! "Bush <size>". Without the asset, the sheet `paths` maps 9/10/11 to the
//! three common wild trees.

use std::collections::BTreeMap;

use engine::Rng;
use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::terrain::{TerrainFeature, Tree};
use crate::world::World;
use crate::TilePos;

pub const SMALL: u8 = 0;
pub const MEDIUM: u8 = 1;
pub const LARGE: u8 = 2;
pub const TEA: u8 = 3;
pub const TEA_GROW_DAYS: i32 = 20;

const SPRING_BERRY: &[&str] = &["296", "Dev_Berry"];
const FALL_BERRY: &[&str] = &["410", "Dev_Berry"];
const TEA_SAPLING: &[&str] = &["251", "Dev_TeaSapling"];
const TEA_LEAVES: &[&str] = &["815", "Dev_TeaLeaves"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bush {
    pub size: u8,
    /// Days played when planted (tea bushes).
    #[serde(default)]
    pub planted: i32,
    /// Has berries or leaves to shake off today.
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub flip: bool,
    #[serde(skip)]
    pub shake: i32,
}

impl Bush {
    pub fn new(size: u8) -> Self {
        Self { size, planted: 0, ready: false, flip: false, shake: 0 }
    }
    /// Width in tiles (the bush stands on its left tile).
    pub fn width(&self) -> i32 {
        match self.size {
            MEDIUM => 2,
            LARGE => 3,
            _ => 1,
        }
    }
}

/// Is `day` of `season` a berry day, and which berries?
pub fn berry_window(season: crate::time::Season, day: i32) -> Option<&'static [&'static str]> {
    use crate::time::Season::*;
    match season {
        Spring if (15..=18).contains(&day) => Some(SPRING_BERRY),
        Fall if (8..=11).contains(&day) => Some(FALL_BERRY),
        _ => None,
    }
}

/// `Port/PathsLayer`: sheet-name key -> tile index -> spawn.
pub type PathsTable = BTreeMap<String, BTreeMap<String, String>>;

pub fn default_paths_table() -> PathsTable {
    let mut t = BTreeMap::new();
    t.insert("paths".to_string(), [("9", "Tree 1"), ("10", "Tree 2"), ("11", "Tree 3")].into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect());
    t
}

impl World {
    fn first_known(&self, ids: &[&str]) -> Option<String> {
        ids.iter().find(|id| self.items.objects.contains_key(**id)).map(|s| s.to_string())
    }

    pub fn is_tea_sapling(&self, id: &str) -> bool {
        TEA_SAPLING.contains(&id) && self.items.objects.contains_key(id)
    }

    /// The bush covering tile `t` (its anchor tile and the bush).
    pub fn bush_at(&self, t: TilePos) -> Option<(TilePos, &Bush)> {
        let loc = self.location();
        (0..3).find_map(|dx| {
            let a = TilePos::new(t.x - dx, t.y);
            match loc.terrain(a) {
                Some(TerrainFeature::Bush(b)) if dx < b.width() => Some((a, b)),
                _ => None,
            }
        })
    }

    /// Shake a bush: berries or tea leaves fall out when it has some.
    pub(crate) fn shake_bush(&mut self, t: TilePos) -> bool {
        let Some((at, bush)) = self.bush_at(t).map(|(a, b)| (a, b.clone())) else { return false };
        if let Some(TerrainFeature::Bush(b)) = self.location_mut().terrain_mut(at) {
            b.shake = 12;
        }
        self.sound("leafrustle");
        if !bush.ready {
            return true;
        }
        let drops: Vec<Item> = if bush.size == TEA {
            self.first_known(TEA_LEAVES).map(|id| vec![Item::object(&id, 1)]).unwrap_or_default()
        } else {
            let Some(ids) = berry_window(self.clock.season, self.clock.day) else { return true };
            let Some(id) = self.first_known(ids) else { return true };
            let level = self.farmer.skills.level(crate::farmer::Skill::Foraging);
            let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, at.x as i64, at.y as i64, 727]));
            let n = rng.range(1, 3) + level / 4;
            let q = if self.farmer.has_profession("botanist") { crate::item::Quality::Iridium } else { crate::item::Quality::Normal };
            vec![Item::object_q(&id, n, q)]
        };
        if let Some(TerrainFeature::Bush(b)) = self.location_mut().terrain_mut(at) {
            b.ready = false;
        }
        let center = at.world_center();
        for d in drops {
            self.spawn_drop(d, center);
        }
        if bush.size != TEA {
            self.farmer.skills.gain(crate::farmer::Skill::Foraging, 1);
        }
        true
    }

    /// Plant a tea sapling.
    pub(crate) fn plant_tea(&mut self, t: TilePos) -> bool {
        let loc = self.location();
        let ok = (loc.is_farm || !loc.outdoors) && loc.is_clear(t) && crate::collision::tile_passable(loc, t) && self.farmer.tile() != t;
        if !ok {
            self.hud("Can't plant that here.");
            return true;
        }
        let mut b = Bush::new(TEA);
        b.planted = self.clock.days_played() as i32;
        self.location_mut().state.terrain.insert(t, TerrainFeature::Bush(b));
        self.farmer.inventory.reduce(self.farmer.current_slot, 1);
        self.sound("dirtyHit");
        true
    }

    /// An axe cuts a farm bush down if it's strong enough.
    pub(crate) fn cut_bush(&mut self, t: TilePos, axe_level: i32) -> bool {
        let Some((at, bush)) = self.bush_at(t).map(|(a, b)| (a, b.clone())) else { return false };
        let needed = match bush.size {
            SMALL | TEA => 0,
            MEDIUM => 1,
            _ => 2,
        };
        if !self.location().is_farm || axe_level < needed {
            if let Some(TerrainFeature::Bush(b)) = self.location_mut().terrain_mut(at) {
                b.shake = 12;
            }
            self.sound("leafrustle");
            return true;
        }
        self.location_mut().state.terrain.remove(&at);
        self.sound("cut");
        if bush.size == TEA {
            if let Some(id) = self.first_known(TEA_SAPLING) {
                self.spawn_drop(Item::object(&id, 1), at.world_center());
            }
        } else {
            self.spawn_drop(Item::object("388", bush.width() * 2), at.world_center());
        }
        true
    }

    /// Overnight: berries ripen in season; grown tea bushes leaf late in
    /// the season.
    pub(crate) fn bushes_overnight(&mut self) {
        let (season, day, today) = (self.clock.season, self.clock.day, self.clock.days_played() as i32);
        let berries = berry_window(season, day).is_some();
        for loc in self.locations.values_mut() {
            let outdoors = loc.outdoors;
            for t in loc.state.terrain.values_mut() {
                let TerrainFeature::Bush(b) = t else { continue };
                b.ready = match b.size {
                    MEDIUM => berries,
                    TEA => today - b.planted >= TEA_GROW_DAYS && day >= 22 && (!outdoors || season != crate::time::Season::Winter),
                    _ => false,
                };
            }
        }
    }

    /// Place trees and bushes marked on locations' `Paths` layers.
    pub(crate) fn spawn_paths_layers(&mut self, table: &PathsTable) {
        for loc in self.locations.values_mut() {
            let Some(layer) = loc.map.layer("Paths") else { continue };
            let mut spawns = Vec::new();
            for y in 0..layer.height {
                for x in 0..layer.width {
                    let Some(tile) = layer.get(x, y) else { continue };
                    let Some(sheet) = loc.map.sheets.get(tile.sheet as usize) else { continue };
                    let image = sheet.image.to_ascii_lowercase();
                    let entry = table.iter().filter(|(k, _)| image.contains(&k.to_ascii_lowercase())).max_by_key(|(k, _)| k.len()).and_then(|(_, m)| m.get(&tile.index.to_string()));
                    if let Some(e) = entry {
                        spawns.push((TilePos::new(x, y), e.clone()));
                    }
                }
            }
            for (p, spec) in spawns {
                if loc.state.terrain.contains_key(&p) || loc.state.objects.contains_key(&p) {
                    continue;
                }
                let toks: Vec<&str> = spec.split_whitespace().collect();
                let feature = match toks.as_slice() {
                    ["Tree", kind, ..] => TerrainFeature::Tree(Tree::new(kind, crate::terrain::TREE_MATURE)),
                    ["Bush", size, ..] => TerrainFeature::Bush(Bush::new(size.parse().unwrap_or(MEDIUM))),
                    _ => continue,
                };
                loc.state.terrain.insert(p, feature);
            }
        }
    }
}
