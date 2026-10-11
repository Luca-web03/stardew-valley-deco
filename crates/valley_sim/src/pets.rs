//! The farm pet, trash cans (Data/GarbageCans) and achievements.
//! Pet friendship gains are approximations.

use std::collections::BTreeMap;

use engine::{Rng, Vec2};
use serde::{Deserialize, Serialize};

use crate::collision::box_blocked;
use crate::farm::FarmAnimal;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos, TILE_F};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pet {
    pub name: String,
    /// Texture asset of the pet sheet (32x32 frames, rows down/right/up/left).
    pub texture: String,
    pub location: String,
    pub pos: Vec2,
    pub facing: Dir,
    /// 0..=1000.
    pub friendship: i32,
    pub petted_today: bool,
    pub bowl_filled: bool,
    #[serde(skip)]
    pub moving: bool,
    #[serde(skip)]
    pub walk_ms: u32,
    #[serde(skip)]
    pub target: Option<Vec2>,
    #[serde(skip)]
    pub wait_ms: u32,
    #[serde(skip)]
    pub emote_ms: u32,
}

/// Engine-defined milestones (original names), checked each night.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Achievement {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

pub const ACHIEVEMENTS: &[Achievement] = &[
    Achievement { id: "earn_15k", name: "Pocket Change", description: "Earn 15,000g." },
    Achievement { id: "earn_100k", name: "Well Off", description: "Earn 100,000g." },
    Achievement { id: "ship_100", name: "Steady Supplier", description: "Ship 100 items." },
    Achievement { id: "fish_10", name: "Hooked", description: "Catch 10 fish." },
    Achievement { id: "cook_10", name: "Home Cook", description: "Cook 10 dishes." },
    Achievement { id: "craft_15", name: "Tinkerer", description: "Craft 15 items." },
    Achievement { id: "mine_40", name: "Deep Diver", description: "Reach level 40 of the mines." },
    Achievement { id: "slay_100", name: "Monster Hunter", description: "Defeat 100 monsters." },
    Achievement { id: "friend_5", name: "Good Neighbor", description: "Reach 5 hearts with someone." },
    Achievement { id: "married", name: "Partner for Life", description: "Get married." },
    Achievement { id: "arcade_ranger", name: "Sheriff of the Sands", description: "Beat Dust Ranger." },
    Achievement { id: "arcade_rails", name: "End of the Line", description: "Finish every track of Rail Runner." },
];

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct GarbageItem {
    item_id: String,
    condition: Option<String>,
    min_stack: i32,
    max_stack: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct GarbageCan {
    base_chance: Option<f64>,
    items: Vec<GarbageItem>,
}

/// `Data/GarbageCans` (public format, subset).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct GarbageData {
    default_chance_to_find_item: Option<f64>,
    before_all: Vec<GarbageItem>,
    after_all: Vec<GarbageItem>,
    garbage_cans: BTreeMap<String, GarbageCan>,
}

impl World {
    /// Give the farm its pet (called for new games).
    pub(crate) fn adopt_pet(&mut self, name: &str) {
        let texture = ["Animals/dog", "Animals/Dev_Dog"].iter().find(|t| self.pet_textures.contains(&t.to_string())).map(|s| s.to_string()).unwrap_or_else(|| "Animals/Dev_Dog".into());
        let start = self.locations.get("Farm").and_then(|f| f.map_prop_tile("FarmHouseEntry")).unwrap_or(TilePos::new(10, 10));
        self.pet = Some(Pet {
            name: name.to_string(),
            texture,
            location: "Farm".into(),
            pos: Vec2::new((start.x + 2) as f32 * TILE_F, (start.y + 1) as f32 * TILE_F - 16.0),
            facing: Dir::Down,
            friendship: 0,
            petted_today: false,
            bowl_filled: false,
            moving: false,
            walk_ms: 0,
            target: None,
            wait_ms: 0,
            emote_ms: 0,
        });
    }

    pub(crate) fn pet_the_pet(&mut self, t: TilePos) -> bool {
        let here = self.current.clone();
        let me = self.farmer.tile();
        let Some(p) = self.pet.as_mut() else { return false };
        let pt = TilePos::from_world(FarmAnimal::bbox_at(p.pos).center());
        if p.location != here || (pt != t && !(pt.manhattan(me) <= 1 && pt.manhattan(t) <= 1)) {
            return false;
        }
        if !p.petted_today {
            p.petted_today = true;
            p.friendship = (p.friendship + 12).min(1000);
        }
        p.emote_ms = 1500;
        p.wait_ms = 2000;
        p.target = None;
        let name = p.name.clone();
        self.sound("dog_bark");
        self.hud(format!("{name} is happy to see you!"));
        true
    }

    /// Watering can on the bowl tile fills it.
    pub(crate) fn fill_pet_bowl(&mut self, t: TilePos) -> bool {
        if self.current != "Farm" {
            return false;
        }
        let Some(bowl) = self.location().map_prop_tile("PetBowlTile") else { return false };
        if t != bowl {
            return false;
        }
        if let Some(p) = self.pet.as_mut() {
            p.bowl_filled = true;
        }
        true
    }

    pub(crate) fn update_pet(&mut self, dt: u32) {
        let Some(mut p) = self.pet.take() else { return };
        p.emote_ms = p.emote_ms.saturating_sub(dt);
        p.wait_ms = p.wait_ms.saturating_sub(dt);
        p.moving = false;
        if let Some(loc) = self.locations.get(&p.location) {
            let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 313]));
            if p.target.is_none() && p.wait_ms == 0 {
                let d = Vec2::new(rng.range(-4, 5) as f32, rng.range(-4, 5) as f32) * TILE_F;
                p.target = Some(p.pos + d);
                p.wait_ms = rng.range(2000, 6000) as u32;
            }
            if let Some(t) = p.target {
                let d = t - p.pos;
                if d.length() < 2.0 {
                    p.target = None;
                } else {
                    let v = d.normalized() * 2.0;
                    let np = p.pos + v;
                    if box_blocked(loc, FarmAnimal::bbox_at(np)) {
                        p.target = None;
                    } else {
                        p.pos = np;
                        p.moving = true;
                        p.walk_ms += dt;
                        p.facing = Dir::toward((v.x * 10.0) as i32, (v.y * 10.0) as i32);
                    }
                }
            }
        }
        self.pet = Some(p);
    }

    /// Morning: rain keeps the pet inside and fills the bowl; friendship from yesterday.
    pub(crate) fn pet_morning(&mut self) {
        let raining = self.weather.is_raining();
        let house_tile = {
            let wake = self.bed_tile;
            TilePos::new(wake.x - 2, wake.y + 2)
        };
        let farm_tile = self.locations.get("Farm").and_then(|f| f.map_prop_tile("FarmHouseEntry")).unwrap_or(TilePos::new(10, 10));
        let Some(p) = self.pet.as_mut() else { return };
        if p.bowl_filled {
            p.friendship = (p.friendship + 6).min(1000);
        }
        p.petted_today = false;
        p.bowl_filled = raining;
        p.target = None;
        if raining {
            p.location = "FarmHouse".into();
            p.pos = Vec2::new(house_tile.x as f32 * TILE_F, house_tile.y as f32 * TILE_F - 16.0);
        } else {
            p.location = "Farm".into();
            p.pos = Vec2::new((farm_tile.x + 2) as f32 * TILE_F, (farm_tile.y + 1) as f32 * TILE_F - 16.0);
        }
    }

    /// Search a trash can once per day.
    pub(crate) fn search_garbage(&mut self, can: &str) -> bool {
        let key = format!("{}:{can}", self.current);
        if self.garbage_searched.contains(&key) {
            self.emit(WorldEvent::Message("It's empty.".into()));
            return true;
        }
        self.garbage_searched.push(key);
        self.sound("trashcan");
        let Some(data) = self.garbage.clone() else { return true };
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, can.len() as i64, 66]));
        let entry = data.garbage_cans.get(can).cloned().unwrap_or_default();
        let chance = entry.base_chance.or(data.default_chance_to_find_item).unwrap_or(0.2);
        let pool: Vec<GarbageItem> = data.before_all.iter().cloned().chain(entry.items.iter().cloned()).chain(data.after_all.iter().cloned()).collect();
        let me = self.farmer.center();
        self.maybe_drop_note(&mut rng, me);
        if pool.is_empty() || !rng.chance(chance) {
            self.hud("Nothing but trash.");
            return true;
        }
        let pick = rng.pick(&pool).cloned().unwrap();
        let n = if pick.max_stack > pick.min_stack { rng.range(pick.min_stack.max(1), pick.max_stack + 1) } else { pick.min_stack.max(1) };
        if let Some(item) = self.items.create(&pick.item_id, n) {
            self.give_item(item);
        }
        true
    }

    /// Night: award achievements whose conditions are met.
    pub(crate) fn check_achievements(&mut self) {
        let f = &self.farmer;
        let shipped: i32 = f.shipped.values().sum();
        let fish: i32 = f.fish_caught.values().map(|v| v.0).sum();
        let cooked: i32 = f.cooking_recipes.values().sum();
        let crafted: i32 = f.crafting_recipes.values().sum();
        let best_hearts = f.friendships.values().map(|x| x.hearts()).max().unwrap_or(0);
        let won = |g: &str| f.counters.get(&format!("Arcade.{g}.Wins")).copied().unwrap_or(0) > 0;
        let checks: [(&str, bool); 12] = [
            ("earn_15k", f.total_earned >= 15_000),
            ("earn_100k", f.total_earned >= 100_000),
            ("ship_100", shipped >= 100),
            ("fish_10", fish >= 10),
            ("cook_10", cooked >= 10),
            ("craft_15", crafted >= 15),
            ("mine_40", f.deepest_mine_level >= 40),
            ("slay_100", self.monsters_killed >= 100),
            ("friend_5", best_hearts >= 5),
            ("married", f.spouse.is_some()),
            ("arcade_ranger", won("ranger")),
            ("arcade_rails", won("rails")),
        ];
        for (id, ok) in checks {
            if ok && !self.farmer.achievements.iter().any(|a| a == id) {
                self.farmer.achievements.push(id.to_string());
                let name = ACHIEVEMENTS.iter().find(|a| a.id == id).map(|a| a.name).unwrap_or(id);
                self.emit(WorldEvent::Hud(format!("Achievement: {name}")));
            }
        }
    }
}
