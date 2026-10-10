//! Farm buildings (Data/Buildings) and animals (Data/FarmAnimals).
//! Movement, mood and produce timing are approximations.

use std::collections::BTreeMap;

use engine::{IRect, Rng, Vec2};
use serde::{Deserialize, Serialize};
use valley_content::data::{BuildingData, FarmAnimalData};

use crate::collision::tile_passable;
use crate::item::{Item, Quality, ToolKind};
use crate::location::{GameLocation, Warp};
use crate::objects::{ObjKind, PlacedObject};
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos, TILE_F};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub id: u32,
    pub kind: String,
    /// Top-left footprint tile on the farm.
    pub tile: TilePos,
    pub size: (i32, i32),
    /// Interior location name, once built.
    pub interior: Option<String>,
    /// Days of construction left (0 = finished).
    pub days_left: i32,
    pub animal_door_open: bool,
}

impl Building {
    pub fn footprint(&self) -> impl Iterator<Item = TilePos> + '_ {
        (0..self.size.1).flat_map(move |dy| (0..self.size.0).map(move |dx| TilePos::new(self.tile.x + dx, self.tile.y + dy)))
    }
    pub fn contains(&self, t: TilePos) -> bool {
        t.x >= self.tile.x && t.y >= self.tile.y && t.x < self.tile.x + self.size.0 && t.y < self.tile.y + self.size.1
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FarmAnimal {
    pub id: u32,
    pub name: String,
    pub kind: String,
    /// Interior location this animal lives in.
    pub home: String,
    pub location: String,
    pub pos: Vec2,
    pub facing: Dir,
    pub age_days: i32,
    /// 0..=1000.
    pub friendship: i32,
    pub fed_today: bool,
    pub petted_today: bool,
    pub went_outside: bool,
    pub days_since_produce: i32,
    /// Produce waiting for a tool (milk pail / shears).
    pub produce: Option<Item>,
    #[serde(skip)]
    pub moving: bool,
    #[serde(skip)]
    pub walk_ms: u32,
    #[serde(skip)]
    pub wander_ms: u32,
    #[serde(skip)]
    pub target: Option<Vec2>,
    #[serde(skip)]
    pub emote_ms: u32,
}

impl FarmAnimal {
    pub fn bbox_at(p: Vec2) -> IRect {
        IRect::new(p.x as i32 + 12, p.y as i32 + 28, 40, 32)
    }
    pub fn bbox(&self) -> IRect {
        Self::bbox_at(self.pos)
    }
    pub fn tile(&self) -> TilePos {
        TilePos::from_world(self.bbox().center())
    }
    pub fn is_baby(&self, data: &FarmAnimalData) -> bool {
        self.age_days < data.days_to_mature
    }
}

/// Tile property marking a feeding trough in animal houses.
pub const TROUGH_PROP: &str = "Trough";

impl World {
    pub fn building_data(&self, kind: &str) -> Option<&BuildingData> {
        self.buildings_data.get(kind)
    }

    /// Tiles covered by finished or in-progress buildings in a location.
    pub fn building_at(&self, location: &str, t: TilePos) -> Option<&Building> {
        if location != "Farm" {
            return None;
        }
        self.buildings.iter().find(|b| b.contains(t))
    }

    /// Validate and pay for a new building at `tile`. Returns an error message on failure.
    pub fn place_building(&mut self, kind: &str, tile: TilePos) -> Result<(), String> {
        let data = self.buildings_data.get(kind).cloned().ok_or("Unknown building")?;
        let size = (data.size.x.max(1), data.size.y.max(1));
        let farm = self.locations.get("Farm").ok_or("No farm")?;
        for dy in 0..size.1 {
            for dx in 0..size.0 {
                let p = TilePos::new(tile.x + dx, tile.y + dy);
                if !farm.is_clear(p) || !tile_passable(farm, p) || self.building_at("Farm", p).is_some() {
                    return Err("Can't build there.".into());
                }
                if self.farmer.tile() == p && self.current == "Farm" {
                    return Err("Something is in the way.".into());
                }
            }
        }
        if self.farmer.money < data.build_cost {
            return Err("Not enough money.".into());
        }
        for m in &data.build_materials {
            let (_, id) = crate::item::split_qualified(&m.item_id);
            if self.farmer.inventory.count(id) < m.amount {
                return Err("Not enough materials.".into());
            }
        }
        self.farmer.money -= data.build_cost;
        for m in &data.build_materials {
            let (_, id) = crate::item::split_qualified(&m.item_id);
            self.farmer.inventory.remove_object(id, m.amount);
        }
        self.next_building_id += 1;
        let b = Building { id: self.next_building_id, kind: kind.to_string(), tile, size, interior: None, days_left: data.build_days.max(0), animal_door_open: true };
        let done = b.days_left == 0;
        self.buildings.push(b);
        self.rebuild_location_edges();
        if done {
            let id = self.next_building_id;
            self.finish_building(id);
        }
        self.sound("axe");
        Ok(())
    }

    /// Create the interior and hook up its exit warp.
    pub(crate) fn finish_building(&mut self, id: u32) {
        let Some(bi) = self.buildings.iter().position(|b| b.id == id) else { return };
        let b = self.buildings[bi].clone();
        let Some(data) = self.buildings_data.get(&b.kind).cloned() else { return };
        let Some(map_name) = data.indoor_map.clone() else { return };
        let Some(map) = self.building_maps.get(&map_name).cloned() else { return };
        let name = format!("{}{}", b.kind.replace(' ', ""), b.id);
        let mut loc = GameLocation::new(&name, map);
        loc.outdoors = false;
        loc.display_name = data.name.clone();
        let door = self.door_tile(&b, &data);
        let exit = TilePos::new(door.x, door.y + 1);
        let mut warps: Vec<Warp> = loc.warps.iter().cloned().map(|mut w| {
            if w.target == "Farm" || w.target.is_empty() {
                w.target = "Farm".into();
                w.to = exit;
            }
            w
        }).collect();
        if warps.is_empty() {
            // Interiors without warps: exit from the bottom-middle tile.
            warps.push(Warp { from: TilePos::new(loc.width() / 2, loc.height() - 1), target: "Farm".into(), to: exit });
        }
        loc.warps = warps;
        self.locations.insert(name.clone(), loc);
        self.buildings[bi].interior = Some(name);
        self.buildings[bi].days_left = 0;
        self.rebuild_location_edges();
    }

    pub fn door_tile(&self, b: &Building, data: &BuildingData) -> TilePos {
        if data.human_door.x >= 0 {
            TilePos::new(b.tile.x + data.human_door.x, b.tile.y + data.human_door.y)
        } else {
            TilePos::new(b.tile.x + b.size.0 / 2, b.tile.y + b.size.1 - 1)
        }
    }

    /// Action on a building tile: enter through the door.
    pub(crate) fn interact_building(&mut self, t: TilePos) -> bool {
        let Some(b) = self.building_at(&self.current, t).cloned() else { return false };
        if b.days_left > 0 {
            self.emit(WorldEvent::Message(format!("Under construction: {} day(s) left.", b.days_left)));
            return true;
        }
        let Some(data) = self.buildings_data.get(&b.kind).cloned() else { return false };
        let door = self.door_tile(&b, &data);
        if t != door {
            // Animal door toggles when clicked.
            if let Some(ad) = &data.animal_door {
                let at = TilePos::new(b.tile.x + ad.x, b.tile.y + ad.y);
                if t == at {
                    if let Some(bm) = self.buildings.iter_mut().find(|x| x.id == b.id) {
                        bm.animal_door_open = !bm.animal_door_open;
                    }
                    self.sound("doorCreak");
                    return true;
                }
            }
            return false;
        }
        let Some(interior) = b.interior.clone() else { return false };
        let entry = self.locations.get(&interior).map(|l| {
            l.warps.first().map(|w| TilePos::new(w.from.x, w.from.y - 1)).unwrap_or(TilePos::new(l.width() / 2, l.height() - 2))
        });
        if let Some(e) = entry {
            self.warp_farmer(&interior, e, Some(Dir::Up));
        }
        true
    }

    /// Buy an animal; it moves into the first suitable building with space.
    pub fn buy_animal(&mut self, kind: &str, name: Option<String>) -> Result<(), String> {
        let data = self.animals_data.get(kind).cloned().ok_or("Unknown animal")?;
        if data.purchase_price < 0 {
            return Err("Not for sale.".into());
        }
        if self.farmer.money < data.purchase_price {
            return Err("Not enough money.".into());
        }
        let home = self
            .buildings
            .iter()
            .filter(|b| b.interior.is_some())
            .find(|b| {
                let bd = self.buildings_data.get(&b.kind);
                let fits = bd.is_some_and(|d| d.valid_occupant_types.iter().any(|t| t == &data.house));
                let occupants = self.animals.iter().filter(|a| Some(&a.home) == b.interior.as_ref()).count() as i32;
                fits && occupants < bd.map(|d| d.max_occupants).unwrap_or(0)
            })
            .and_then(|b| b.interior.clone())
            .ok_or(format!("You need a {} with free space.", data.house))?;
        self.farmer.money -= data.purchase_price;
        self.next_animal_id += 1;
        let loc = &self.locations[&home];
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.next_animal_id as i64, 17]));
        let mut spot = TilePos::new(loc.width() / 2, loc.height() / 2);
        for _ in 0..100 {
            let p = TilePos::new(rng.range(1, loc.width() - 1), rng.range(1, loc.height() - 1));
            if tile_passable(loc, p) {
                spot = p;
                break;
            }
        }
        let display = if data.display_name.is_empty() { kind.to_string() } else { data.display_name.clone() };
        let count = self.animals.iter().filter(|a| a.kind == kind).count() + 1;
        self.animals.push(FarmAnimal {
            id: self.next_animal_id,
            name: name.unwrap_or_else(|| format!("{display} {count}")),
            kind: kind.to_string(),
            home: home.clone(),
            location: home,
            pos: Vec2::new(spot.x as f32 * TILE_F, spot.y as f32 * TILE_F - 16.0),
            facing: Dir::Down,
            age_days: 0,
            friendship: 0,
            fed_today: false,
            petted_today: false,
            went_outside: false,
            days_since_produce: 0,
            produce: None,
            moving: false,
            walk_ms: 0,
            wander_ms: 0,
            target: None,
            emote_ms: 0,
        });
        self.sound("purchase");
        Ok(())
    }

    fn animal_at(&self, t: TilePos) -> Option<usize> {
        let here = self.current.clone();
        let me = self.farmer.tile();
        self.animals.iter().position(|a| a.location == here && (a.tile() == t || (a.tile().manhattan(me) <= 1 && a.tile().manhattan(t) <= 1)))
    }

    /// Pet an animal (action button).
    pub(crate) fn pet_animal(&mut self, t: TilePos) -> bool {
        let Some(i) = self.animal_at(t) else { return false };
        let a = &mut self.animals[i];
        if !a.petted_today {
            a.petted_today = true;
            a.friendship = (a.friendship + 15).min(1000);
        }
        a.emote_ms = 1200;
        let sound = self.animals_data.get(&self.animals[i].kind).and_then(|d| d.sound.clone()).unwrap_or_else(|| "cluck".into());
        self.sound(&sound);
        let name = self.animals[i].name.clone();
        let hearts = self.animals[i].friendship / 200;
        self.hud(format!("{name} {}", "<3".repeat(hearts.max(1) as usize)));
        true
    }

    /// Milk pail / shears.
    pub(crate) fn use_animal_tool(&mut self, kind: ToolKind, t: TilePos) {
        let Some(i) = self.animal_at(t) else {
            self.sound("woodyStep");
            return;
        };
        let tool_name = match kind {
            ToolKind::MilkPail => "MilkPail",
            _ => "Shears",
        };
        let ok = self.animals_data.get(&self.animals[i].kind).and_then(|d| d.harvest_tool.clone()).is_some_and(|h| h.replace(' ', "").eq_ignore_ascii_case(tool_name));
        if !ok {
            return;
        }
        let Some(item) = self.animals[i].produce.take() else {
            self.sound("cancel");
            return;
        };
        if !self.farmer.inventory.can_fit(&item) {
            self.animals[i].produce = Some(item);
            self.hud("Inventory full");
            return;
        }
        self.animals[i].friendship = (self.animals[i].friendship + 5).min(1000);
        self.sound(if kind == ToolKind::MilkPail { "Milking" } else { "scissors" });
        self.give_item(item.clone());
        self.quest_obtained(&item);
    }

    /// Put hay into a trough tile.
    pub(crate) fn place_hay(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(id) = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from) else { return false };
        if Some(&id) != self.hay_id.as_ref() {
            return false;
        }
        let loc = self.location();
        if loc.tile_prop("Back", t, TROUGH_PROP).is_none() && loc.tile_prop("Buildings", t, TROUGH_PROP).is_none() {
            return false;
        }
        if loc.object(t).is_some() {
            return true;
        }
        self.location_mut().state.objects.insert(t, PlacedObject::new(&id, false, ObjKind::Placed));
        self.farmer.inventory.reduce(slot, 1);
        self.sound("shwip");
        true
    }

    /// Per-tick animal wandering and going in/out.
    pub(crate) fn update_animals(&mut self, dt: u32) {
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 211]));
        let locs = &self.locations;
        for a in self.animals.iter_mut() {
            a.emote_ms = a.emote_ms.saturating_sub(dt);
            let Some(loc) = locs.get(&a.location) else { continue };
            a.wander_ms = a.wander_ms.saturating_sub(dt);
            if a.target.is_none() && a.wander_ms == 0 {
                if rng.chance(0.4) {
                    let d = Vec2::new(rng.range(-3, 4) as f32, rng.range(-3, 4) as f32) * TILE_F;
                    a.target = Some(a.pos + d);
                }
                a.wander_ms = rng.range(1500, 5000) as u32;
            }
            a.moving = false;
            if let Some(t) = a.target {
                let d = t - a.pos;
                if d.length() < 2.0 {
                    a.target = None;
                } else {
                    let v = d.normalized() * 1.2;
                    let np = a.pos + v;
                    if crate::collision::box_blocked(loc, FarmAnimal::bbox_at(np)) {
                        a.target = None;
                    } else {
                        a.pos = np;
                        a.moving = true;
                        a.walk_ms += dt;
                        a.facing = Dir::toward((v.x * 10.0) as i32, (v.y * 10.0) as i32);
                    }
                }
            }
        }
    }

    /// Morning: animals go outside through open doors on nice days.
    pub(crate) fn animals_morning(&mut self) {
        let nice = !self.weather.is_raining() && self.weather != crate::time::Weather::Snow && self.clock.season != crate::time::Season::Winter;
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 333]));
        let buildings = self.buildings.clone();
        for a in self.animals.iter_mut() {
            a.location = a.home.clone();
            a.went_outside = false;
            if !nice || rng.chance(0.3) {
                continue;
            }
            let Some(b) = buildings.iter().find(|b| b.interior.as_ref() == Some(&a.home)) else { continue };
            if !b.animal_door_open {
                continue;
            }
            let Some(farm) = self.locations.get("Farm") else { continue };
            let base = TilePos::new(b.tile.x + b.size.0 / 2, b.tile.y + b.size.1 + 1);
            for _ in 0..30 {
                let p = TilePos::new(base.x + rng.range(-3, 4), base.y + rng.range(0, 4));
                if tile_passable(farm, p) && !buildings.iter().any(|x| x.contains(p)) {
                    a.location = "Farm".into();
                    a.went_outside = true;
                    a.pos = Vec2::new(p.x as f32 * TILE_F, p.y as f32 * TILE_F - 16.0);
                    break;
                }
            }
        }
    }

    /// Overnight: eat, age, produce, friendship changes, construction.
    pub(crate) fn animals_overnight(&mut self) {
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 444]));
        // Construction progress.
        let mut finished = Vec::new();
        for b in self.buildings.iter_mut() {
            if b.days_left > 0 {
                b.days_left -= 1;
                if b.days_left == 0 {
                    finished.push(b.id);
                }
            }
        }
        for id in finished {
            self.finish_building(id);
            self.hud("Construction finished!");
        }
        let hay = self.hay_id.clone();
        let winter = self.clock.season == crate::time::Season::Winter;
        let mut drops: Vec<(String, Item)> = Vec::new();
        let animals_data = self.animals_data.clone();
        for i in 0..self.animals.len() {
            let a = self.animals[i].clone();
            let Some(data) = animals_data.get(&a.kind) else { continue };
            // Food: grazing outside or hay from a trough in its home.
            let mut fed = a.went_outside && !winter;
            if !fed {
                if let Some(home) = self.locations.get_mut(&a.home) {
                    let trough = home.state.objects.iter().find(|(_, o)| Some(&o.item_id) == hay.as_ref() && matches!(o.kind, ObjKind::Placed)).map(|(p, _)| *p);
                    if let Some(p) = trough {
                        home.state.objects.remove(&p);
                        fed = true;
                    }
                }
            }
            let mut f = a.friendship;
            if !a.petted_today {
                f -= 10 - f / 200;
            }
            if !fed {
                f -= 20;
            }
            let f = f.clamp(0, 1000);
            let mut age = a.age_days + 1;
            let mut since = a.days_since_produce;
            let mut produce = a.produce.clone();
            if fed && age > data.days_to_mature {
                since += 1;
                if since >= data.days_to_produce.max(1) {
                    since = 0;
                    let list = if f >= 800 && !data.deluxe_produce_item_ids.is_empty() && rng.chance(0.3) { &data.deluxe_produce_item_ids } else { &data.produce_item_ids };
                    let opts: Vec<_> = list.iter().filter(|p| f >= p.minimum_friendship).collect();
                    if let Some(p) = rng.pick(&opts) {
                        let (_, id) = crate::item::split_qualified(&p.item_id);
                        let q = if rng.chance(f as f64 / 1500.0) { if rng.chance(f as f64 / 2000.0) { Quality::Gold } else { Quality::Silver } } else { Quality::Normal };
                        let item = Item::object_q(id, 1, q);
                        if data.harvest_type.eq_ignore_ascii_case("DropOvernight") {
                            drops.push((a.home.clone(), item));
                        } else {
                            produce = Some(item);
                        }
                    }
                }
            }
            if age > 100000 {
                age = 100000;
            }
            let an = &mut self.animals[i];
            an.friendship = f.clamp(0, 1000);
            an.age_days = age;
            an.days_since_produce = since;
            an.produce = produce;
            an.fed_today = fed;
            an.petted_today = false;
        }
        for (home, item) in drops {
            let Some(loc) = self.locations.get_mut(&home) else { continue };
            let (w, h) = (loc.width(), loc.height());
            for _ in 0..60 {
                let p = TilePos::new(rng.range(1, w - 1), rng.range(1, h - 1));
                if loc.is_clear(p) && tile_passable(loc, p) && loc.tile_prop("Back", p, TROUGH_PROP).is_none() {
                    let mut o = PlacedObject::forage(item.object_id().unwrap_or(""));
                    o.kind = ObjKind::Forage;
                    loc.state.objects.insert(p, o);
                    break;
                }
            }
        }
    }
}

/// Building interiors keyed by map asset name ("Coop" -> Maps/Coop).
pub type BuildingMaps = BTreeMap<String, valley_content::Map>;
