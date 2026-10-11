//! A volcano dungeon (generic placeholder mechanics): a short run of
//! generated floors with lava pools that burn, rocks that can hold ember
//! shards, tougher monsters, and a forge on the last floor that spends
//! shards to strengthen weapons. Damage values are approximations.

use engine::Rng;
use valley_content::{PropValue, Tile};

use crate::collision::tile_passable;
use crate::combat::Monster;
use crate::item::Item;
use crate::location::GameLocation;
use crate::objects::{ObjKind, PlacedObject};
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos};

pub const VOLCANO_PREFIX: &str = "Port_Volcano";
pub const VOLCANO_FLOORS: i32 = 5;
/// Mine-sheet tile index used for lava.
pub const LAVA_TILE: i32 = 5;
/// Damage per lava contact (with the usual invincibility window).
pub const LAVA_DAMAGE: i32 = 8;
/// Shards per forge level and the damage each level adds.
pub const FORGE_COSTS: [i32; 3] = [10, 15, 20];
pub const FORGE_BONUS: i32 = 3;
/// Shard object ids (well-known, then placeholder).
const SHARDS: &[&str] = &["848", "Dev_EmberShard"];

pub fn volcano_floor(location: &str) -> Option<i32> {
    location.strip_prefix(VOLCANO_PREFIX).and_then(|n| n.parse().ok())
}

impl World {
    pub fn shard_item(&self) -> Option<&'static str> {
        SHARDS.iter().copied().find(|id| self.items.objects.contains_key(*id))
    }

    /// Is the farmer standing on lava?
    pub fn on_lava(&self) -> bool {
        let t = self.farmer.tile();
        self.location().tile_prop("Back", t, "Type").is_some_and(|v| v.eq_ignore_ascii_case("Lava"))
    }

    /// Per tick: lava burns.
    pub(crate) fn lava_check(&mut self) {
        if self.on_lava() && self.farmer.invincible_ms == 0 {
            let c = self.farmer.center();
            self.damage_farmer(LAVA_DAMAGE, c);
        }
    }

    /// `Port.Volcano`: enter floor 1 from here.
    pub(crate) fn enter_volcano_from_here(&mut self) -> bool {
        let me = self.farmer.tile();
        self.volcano_entrance = (self.current.clone(), me);
        self.enter_volcano(1);
        true
    }

    /// Build and enter volcano floor `n`.
    pub fn enter_volcano(&mut self, n: i32) {
        let n = n.clamp(1, VOLCANO_FLOORS);
        let name = format!("{VOLCANO_PREFIX}{n}");
        let mut map = crate::mines::generate_cave(900 + n, self.seed ^ self.clock.days_played() as u64);
        if let Some(sheet) = map.sheets.get_mut(0) {
            sheet.set_tile_prop(LAVA_TILE, "Type", PropValue::Str("Lava".into()));
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, n as i64, 4242]));
        let mut loc = GameLocation::new(&name, map);
        loc.outdoors = false;
        loc.display_name = format!("Volcano {n}");
        let entry = loc.map_prop_tile("EntryTile").unwrap_or(TilePos::new(loc.width() / 2, 5));
        // Lava pools: blobs on open floor away from the entry.
        let open: Vec<TilePos> = (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).filter(|p| tile_passable(&loc, *p)).collect();
        for _ in 0..(3 + n) {
            let Some(c) = rng.pick(&open).copied() else { break };
            if c.manhattan(entry) < 6 {
                continue;
            }
            for dy in -1..=1 {
                for dx in -2..=2 {
                    let p = TilePos::new(c.x + dx, c.y + dy);
                    if tile_passable(&loc, p) && rng.chance(0.75) {
                        if let Some(layer) = loc.map.layer_mut("Back") {
                            layer.set(p.x, p.y, Some(Tile::new(0, LAVA_TILE)));
                        }
                    }
                }
            }
        }
        // Up ladder at the entry; a way down (or the forge) far from it.
        let up = TilePos::new(entry.x, entry.y - 1);
        loc.state.objects.insert(up, PlacedObject::new("Port_LadderUp", false, ObjKind::Ladder { down: false }));
        let far = open.iter().copied().filter(|p| tile_passable(&loc, *p)).max_by_key(|p| p.manhattan(entry)).unwrap_or(entry);
        if n < VOLCANO_FLOORS {
            loc.state.objects.insert(far, PlacedObject::new("Port_Ladder", false, ObjKind::Ladder { down: true }));
        } else if let Some(layer) = loc.map.layer_mut("Buildings") {
            let mut t = Tile::new(0, 3);
            t.props.insert("Action".into(), PropValue::Str("Port.Forge".into()));
            layer.set(far.x, far.y, Some(t));
        }
        // Rocks (some hold shards) and monsters.
        let rock_sprite = self.mine_items.rock_sprites.first().cloned().unwrap_or_else(|| "Dev_MineRock".into());
        for p in open.iter().copied() {
            if p.manhattan(entry) <= 2 || p == far || !tile_passable(&loc, p) || !loc.is_clear(p) || !rng.chance(0.12) {
                continue;
            }
            loc.state.objects.insert(p, PlacedObject::new(&rock_sprite, false, ObjKind::MineRock { rock: crate::mines::RockKind::Stone, health: 3 }));
        }
        let names: Vec<String> = ["Lava Bat", "Magma Sprite", "Dev_Moth", "Dev_Blob"].iter().filter(|m| self.monster_info.contains_key(**m)).map(|s| s.to_string()).collect();
        for _ in 0..(4 + n * 2) {
            let Some(p) = rng.pick(&open).copied() else { break };
            if p.manhattan(entry) < 6 || names.is_empty() {
                continue;
            }
            let mut info = self.monster_info[rng.pick(&names).unwrap()].clone();
            info.health = info.health * 3 + 20 * n;
            info.damage += 6 + n;
            self.next_monster_id += 1;
            self.monsters.push(Monster::new(self.next_monster_id, info, &name, p));
        }
        self.locations.retain(|k, _| volcano_floor(k).is_none());
        self.monsters.retain(|m| volcano_floor(&m.location).is_none() || m.location == name);
        self.locations.insert(name.clone(), loc);
        // The way down is fixed; no random ladders from rocks.
        self.mine_ladder_spawned = true;
        self.warp_farmer(&name, entry, Some(Dir::Down));
        self.sound("stairsdown");
    }

    /// Ladders inside the volcano. Returns false outside it.
    pub(crate) fn volcano_ladder(&mut self, down: bool) -> bool {
        let Some(n) = volcano_floor(&self.current) else { return false };
        if down {
            self.enter_volcano(n + 1);
        } else {
            self.leave_volcano();
        }
        true
    }

    pub fn leave_volcano(&mut self) {
        self.locations.retain(|k, _| volcano_floor(k).is_none());
        self.monsters.retain(|m| volcano_floor(&m.location).is_none());
        self.dropped.retain(|d| volcano_floor(&d.location).is_none());
        let (loc, tile) = self.volcano_entrance.clone();
        self.warp_farmer(&loc, tile, Some(Dir::Down));
    }

    /// Breaking a volcano rock may turn up a shard.
    pub(crate) fn volcano_rock_bonus(&mut self, at: engine::Vec2) {
        if volcano_floor(&self.current).is_none() {
            return;
        }
        let Some(shard) = self.shard_item() else { return };
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 4343]));
        if rng.chance(0.35) {
            self.spawn_drop(Item::object(shard, rng.range(1, 3)), at);
        }
    }

    /// Forge level of a weapon (0..=3).
    pub fn forge_level(&self, weapon_id: &str) -> i32 {
        self.farmer.weapon_forge.get(weapon_id).copied().unwrap_or(0)
    }

    /// The forge: spend shards to add damage to the held weapon.
    pub fn forge_held_weapon(&mut self) -> Result<i32, String> {
        let Some(id) = self.farmer.current_item().and_then(|i| i.weapon_id()).map(String::from) else { return Err("Hold the weapon you want to forge.".into()) };
        let level = self.forge_level(&id);
        let Some(&cost) = FORGE_COSTS.get(level as usize) else { return Err("This weapon can't be forged any further.".into()) };
        let shard = self.shard_item().ok_or("There's nothing to forge with.")?;
        if self.farmer.inventory.count(shard) < cost {
            return Err(format!("You need {cost} shards."));
        }
        self.farmer.inventory.remove_object(shard, cost);
        self.farmer.weapon_forge.insert(id, level + 1);
        self.sound("crit");
        Ok(level + 1)
    }

    pub(crate) fn forge_action(&mut self) -> bool {
        match self.forge_held_weapon() {
            Ok(l) => self.emit(WorldEvent::Hud(format!("Forged! Weapon damage +{} (level {l}).", FORGE_BONUS * l))),
            Err(e) => self.emit(WorldEvent::Message(e)),
        }
        true
    }
}
