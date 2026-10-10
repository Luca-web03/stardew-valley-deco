//! The mines: entrance, generated or content-provided levels, rocks and
//! ores, ladders, monster spawning. Densities, ladder odds and spawn tables
//! are approximations of the original's behaviour.

use engine::Rng;
use valley_content::{Layer, Map, Tile, TileSheet};

use crate::collision::tile_passable;
use crate::combat::Monster;
use crate::item::Item;
use crate::location::GameLocation;
use crate::objects::{ObjKind, PlacedObject};
use crate::world::World;
use crate::{Dir, TilePos};

pub const MINE_PREFIX: &str = "UndergroundMine";
pub const MAX_LEVEL: i32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RockKind {
    Stone,
    Copper,
    Iron,
    Gold,
}

/// Item ids used by the mines, resolved against loaded data so the same
/// code works with real content (well-known ids) or the placeholder pack.
#[derive(Clone, Debug, Default)]
pub struct MineItems {
    pub stone: String,
    pub coal: Option<String>,
    pub copper_ore: Option<String>,
    pub iron_ore: Option<String>,
    pub gold_ore: Option<String>,
    pub geode: Option<String>,
    pub rock_sprites: Vec<String>,
    pub copper_node: Option<String>,
    pub iron_node: Option<String>,
    pub gold_node: Option<String>,
}

impl MineItems {
    pub fn resolve(db: &crate::item::ItemDb) -> Self {
        let pick = |c: &[&str]| c.iter().find(|id| db.objects.contains_key(**id)).map(|s| s.to_string());
        let rock_sprites: Vec<String> = ["668", "670", "32", "34", "36", "38", "40", "42", "Dev_MineRock"]
            .iter()
            .filter(|id| db.objects.contains_key(**id))
            .map(|s| s.to_string())
            .collect();
        Self {
            stone: pick(&["390"]).unwrap_or_else(|| "390".into()),
            coal: pick(&["382", "Dev_Coal"]),
            copper_ore: pick(&["378", "Dev_CopperOre"]),
            iron_ore: pick(&["380", "Dev_IronOre"]),
            gold_ore: pick(&["384", "Dev_GoldOre"]),
            geode: pick(&["535"]),
            rock_sprites,
            copper_node: pick(&["751", "Dev_CopperNode"]),
            iron_node: pick(&["290", "Dev_IronNode"]),
            gold_node: pick(&["764", "Dev_GoldNode"]),
        }
    }
}

pub fn level_location_name(level: i32) -> String {
    format!("{MINE_PREFIX}{level}")
}

pub fn level_of(location: &str) -> Option<i32> {
    location.strip_prefix(MINE_PREFIX).and_then(|n| n.parse().ok())
}

/// Monster names that can appear at a depth (most specific available).
fn spawn_table(level: i32) -> &'static [&'static str] {
    match level {
        0..=9 => &["Green Slime", "Bug", "Dev_Blob"],
        10..=29 => &["Green Slime", "Duggy", "Fly", "Dev_Blob", "Dev_Moth"],
        30..=39 => &["Bat", "Stone Golem", "Dev_Moth", "Dev_Blob"],
        40..=79 => &["Frost Jelly", "Frost Bat", "Dust Spirit", "Dev_Blob", "Dev_Moth"],
        _ => &["Sludge", "Lava Bat", "Shadow Brute", "Metal Head", "Dev_Blob", "Dev_Moth"],
    }
}

/// Procedural cave for the placeholder pack (cellular automata).
pub fn generate_cave(level: i32, seed: u64) -> Map {
    let (w, h) = (36, 28);
    let mut rng = Rng::new(Rng::seed_from(&[seed as i64, level as i64, 9001]));
    let mut solid: Vec<bool> = (0..w * h).map(|i| {
        let (x, y) = (i % w, i / w);
        x < 2 || y < 3 || x >= w - 2 || y >= h - 2 || rng.chance(0.42)
    }).collect();
    for _ in 0..4 {
        let prev = solid.clone();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let mut n = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if (dx != 0 || dy != 0) && prev[((y + dy) * w + x + dx) as usize] {
                            n += 1;
                        }
                    }
                }
                let i = (y * w + x) as usize;
                solid[i] = if x < 2 || y < 3 || x >= w - 2 || y >= h - 2 { true } else { n >= 5 || (prev[i] && n >= 4) };
            }
        }
    }
    // Carve an entry room near the top middle and a corridor down the center.
    let entry = TilePos::new(w / 2, 4);
    for y in 3..8 {
        for x in w / 2 - 3..=w / 2 + 3 {
            solid[(y * w + x) as usize] = false;
        }
    }
    for y in 4..h - 3 {
        solid[(y * w + w / 2) as usize] = false;
        solid[(y * w + w / 2 + 1) as usize] = false;
    }
    let mut map = Map { id: format!("dev_cave_{level}"), width: w, height: h, tile_w: 16, tile_h: 16, ..Default::default() };
    let mut sheet = TileSheet { id: "mine".into(), image: "dev_mine".into(), sheet_w: 8, sheet_h: 1, tile_w: 16, tile_h: 16, ..Default::default() };
    sheet.set_tile_prop(0, "Type", valley_content::PropValue::Str("Stone".into()));
    sheet.set_tile_prop(1, "Type", valley_content::PropValue::Str("Stone".into()));
    map.sheets.push(sheet);
    let mut back = Layer::new("Back", w, h);
    let mut buildings = Layer::new("Buildings", w, h);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            back.set(x, y, Some(Tile::new(0, if (x * 7 + y * 13) % 5 == 0 { 1 } else { 0 })));
            if solid[i] {
                let below_open = y + 1 < h && !solid[((y + 1) * w + x) as usize];
                buildings.set(x, y, Some(Tile::new(0, if below_open { 3 } else { 2 })));
            }
        }
    }
    map.layers.push(back);
    map.layers.push(buildings);
    map.layers.push(Layer::new("Front", w, h));
    map.props.insert("EntryTile".into(), valley_content::PropValue::Str(format!("{} {}", entry.x, entry.y + 1)));
    map
}

/// Where the farmer arrives on a level map.
fn entry_tile(loc: &GameLocation) -> TilePos {
    if let Some(t) = loc.map_prop_tile("EntryTile") {
        return t;
    }
    // Content maps: arrive below the up-ladder tile if one can be found.
    if let Some(layer) = loc.map.layer("Buildings") {
        for y in 0..layer.height {
            for x in 0..layer.width {
                if let Some(t) = layer.get(x, y) {
                    let img = loc.map.sheets.get(t.sheet as usize).map(|s| s.image.to_ascii_lowercase()).unwrap_or_default();
                    if img.contains("mine") && t.index == 115 {
                        return TilePos::new(x, y + 1);
                    }
                }
            }
        }
    }
    let c = TilePos::new(loc.width() / 2, loc.height() / 2);
    let mut best = None;
    let mut best_d = i32::MAX;
    for y in 0..loc.height() {
        for x in 0..loc.width() {
            let p = TilePos::new(x, y);
            if tile_passable(loc, p) && p.manhattan(c) < best_d {
                best_d = p.manhattan(c);
                best = Some(p);
            }
        }
    }
    best.unwrap_or(c)
}

impl World {
    pub fn mine_level(&self) -> Option<i32> {
        level_of(&self.current)
    }

    /// Remove all generated mine levels (and their monsters).
    pub fn leave_mines(&mut self) {
        self.locations.retain(|k, _| level_of(k).is_none());
        self.monsters.retain(|m| level_of(&m.location).is_none());
        self.dropped.retain(|d| level_of(&d.location).is_none());
    }

    /// First visit: hand over a starter weapon if the farmer has none.
    fn give_starter_weapon(&mut self) {
        if self.farmer.inventory.slots.iter().flatten().any(|i| i.weapon_id().is_some()) {
            return;
        }
        if self.farmer.mail_received.iter().any(|m| m == "port_starter_weapon") {
            return;
        }
        let id = ["0", "Dev_Sword"].iter().find(|id| self.items.weapons.contains_key(**id)).map(|s| s.to_string());
        if let Some(id) = id {
            self.farmer.mail_received.push("port_starter_weapon".into());
            let item = Item::Weapon { id };
            let name = self.items.display_name(&item);
            self.give_item(item);
            self.emit(crate::world::WorldEvent::Message(format!("You found a {name} by the entrance. The mines are dangerous; keep it handy.")));
        }
    }

    /// Build and enter a mine level.
    pub fn enter_mine_level(&mut self, level: i32) {
        self.give_starter_weapon();
        let level = level.clamp(1, MAX_LEVEL);
        let name = level_location_name(level);
        let map = if self.mine_maps.is_empty() {
            generate_cave(level, self.seed ^ self.clock.days_played() as u64)
        } else {
            self.mine_maps[((level - 1) as usize) % self.mine_maps.len()].clone()
        };
        let mut loc = GameLocation::new(&name, map);
        loc.outdoors = false;
        loc.display_name = format!("Mines {level}");
        let entry = entry_tile(&loc);
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, level as i64, 404]));

        // Up-ladder above the entry.
        let up = TilePos::new(entry.x, entry.y - 1);
        loc.state.objects.insert(up, PlacedObject::new("Port_LadderUp", false, ObjKind::Ladder { down: false }));

        // Rocks and ore nodes.
        let mi = self.mine_items.clone();
        let mut stones = 0;
        for y in 0..loc.height() {
            for x in 0..loc.width() {
                let p = TilePos::new(x, y);
                if p.manhattan(entry) <= 2 || !loc.is_clear(p) || !tile_passable(&loc, p) {
                    continue;
                }
                if !rng.chance(0.18) {
                    continue;
                }
                let ore_roll = rng.next_f64();
                let (kind, sprite) = match level {
                    l if l >= 80 && ore_roll < 0.05 => (RockKind::Gold, mi.gold_node.clone()),
                    l if l >= 40 && ore_roll < 0.06 => (RockKind::Iron, mi.iron_node.clone()),
                    _ if ore_roll < 0.07 => (RockKind::Copper, mi.copper_node.clone()),
                    _ => (RockKind::Stone, None),
                };
                let sprite = sprite.or_else(|| rng.pick(&mi.rock_sprites).cloned()).unwrap_or_else(|| "Dev_MineRock".into());
                let health = match kind {
                    RockKind::Stone => 1 + level / 40,
                    RockKind::Copper => 3,
                    RockKind::Iron => 4,
                    RockKind::Gold => 6,
                };
                let mut o = PlacedObject::new(&sprite, false, ObjKind::MineRock { rock: kind, health });
                o.flip = rng.chance(0.5);
                loc.state.objects.insert(p, o);
                stones += 1;
            }
        }
        self.mine_stones_left = stones;
        self.mine_ladder_spawned = false;

        // Monsters (none on every 5th floor, which acts as a rest stop).
        let spawnable: Vec<String> = spawn_table(level).iter().filter(|n| self.monster_info.contains_key(**n)).map(|s| s.to_string()).collect();
        if level % 5 != 0 && !spawnable.is_empty() {
            let count = 3 + level / 8 + rng.range(0, 3);
            let mut placed = 0;
            for _ in 0..count * 30 {
                if placed >= count {
                    break;
                }
                let p = TilePos::new(rng.range(0, loc.width()), rng.range(0, loc.height()));
                if p.manhattan(entry) < 6 || !tile_passable(&loc, p) {
                    continue;
                }
                let name = rng.pick(&spawnable).cloned().unwrap();
                let mut info = self.monster_info[&name].clone();
                // Deeper floors scale monsters up a little.
                info.health += level / 4;
                info.damage += level / 20;
                self.next_monster_id += 1;
                self.monsters.push(Monster::new(self.next_monster_id, info, &name_clone(&name, &loc.name), p));
                placed += 1;
            }
        }
        self.locations.insert(name.clone(), loc);
        if level > self.farmer.deepest_mine_level {
            self.farmer.deepest_mine_level = level;
        }
        // Drop previous levels.
        let keep = name.clone();
        self.locations.retain(|k, _| level_of(k).is_none() || *k == keep);
        self.monsters.retain(|m| level_of(&m.location).is_none() || m.location == keep);
        self.warp_farmer(&name, entry, Some(Dir::Down));
        self.sound("stairsdown");
    }

    /// Break a mine rock at `t` with pickaxe power `power`.
    pub(crate) fn hit_mine_rock(&mut self, t: TilePos, power: i32) -> bool {
        let Some(PlacedObject { kind: ObjKind::MineRock { rock, health }, .. }) = self.location().object(t).cloned() else { return false };
        let left = health - power;
        if left > 0 {
            if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                o.kind = ObjKind::MineRock { rock, health: left };
                o.shake = 10;
            }
            self.sound("hammer");
            return true;
        }
        self.location_mut().state.objects.remove(&t);
        self.sound("stoneCrack");
        let level = self.mine_level().unwrap_or(1);
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, t.x as i64, t.y as i64]));
        let mi = self.mine_items.clone();
        let c = t.world_center();
        match rock {
            RockKind::Stone => {
                self.spawn_drop(Item::object(&mi.stone, 1), c);
                if let Some(coal) = &mi.coal {
                    if rng.chance(0.05) {
                        self.spawn_drop(Item::object(coal, 1), c);
                    }
                }
                if let Some(g) = &mi.geode {
                    if rng.chance(0.02 + level as f64 / 4000.0) {
                        self.spawn_drop(Item::object(g, 1), c);
                    }
                }
            }
            RockKind::Copper | RockKind::Iron | RockKind::Gold => {
                let ore = match rock {
                    RockKind::Copper => mi.copper_ore.clone(),
                    RockKind::Iron => mi.iron_ore.clone(),
                    _ => mi.gold_ore.clone(),
                };
                if let Some(o) = ore {
                    self.spawn_drop(Item::object(&o, rng.range(1, 4)), c);
                }
            }
        }
        if let Some(l) = self.farmer.skills.gain(crate::farmer::Skill::Mining, if rock == RockKind::Stone { 1 } else { 5 }) {
            self.emit(crate::world::WorldEvent::LevelUp(crate::farmer::Skill::Mining, l));
        }
        // Ladder chance rises as rocks run out.
        self.mine_stones_left = (self.mine_stones_left - 1).max(0);
        if !self.mine_ladder_spawned && level < MAX_LEVEL {
            let chance = 0.02 + 1.0 / (self.mine_stones_left.max(1) as f64);
            if rng.chance(chance) {
                self.mine_ladder_spawned = true;
                self.location_mut().state.objects.insert(t, PlacedObject::new("Port_Ladder", false, ObjKind::Ladder { down: true }));
                self.sound("newRecipe");
            }
        }
        true
    }

    /// Use a ladder object at `t`.
    pub(crate) fn use_ladder(&mut self, t: TilePos) -> bool {
        let Some(PlacedObject { kind: ObjKind::Ladder { down }, .. }) = self.location().object(t).cloned() else { return false };
        let level = self.mine_level().unwrap_or(0);
        if down {
            self.enter_mine_level(level + 1);
        } else {
            self.leave_mines();
            let (loc, tile) = self.mine_entrance.clone();
            self.warp_farmer(&loc, tile, Some(Dir::Down));
            self.sound("stairsdown");
        }
        true
    }
}

fn name_clone(_monster: &str, location: &str) -> String {
    location.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_caves_are_deterministic_and_open() {
        let a = generate_cave(3, 42);
        let b = generate_cave(3, 42);
        assert_eq!(crate::location::GameLocation::new("x", a.clone()).warps.len(), 0);
        let loc = GameLocation::new("L", a);
        let entry = entry_tile(&loc);
        assert!(tile_passable(&loc, entry));
        let open = (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).filter(|p| tile_passable(&loc, *p)).count();
        assert!(open > 150, "{open}");
        assert_eq!(b.layers[1].tiles, loc.map.layers[1].tiles);
    }
}
