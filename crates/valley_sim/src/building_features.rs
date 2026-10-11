//! More of `Data/Buildings` (public format): upgrades
//! (`BuildingToUpgrade`: the base building is rebuilt in place, keeping
//! its interior contents and animals), `HayCapacity`, building chests with
//! overnight `ItemConversions` (e.g. a mill), `ActionTiles` (including
//! `BuildingChest <id>` to load or collect), `IndoorItems` placed when
//! built, `CollisionMap` footprints, `MagicalConstruction` and `Skins`
//! (alternative looks: chosen when building through entries
//! `<kind>|<skin>` for skins with `ShowAsSeparateConstructionEntry`, which
//! may override cost, materials and days, or changed later).

use std::collections::{BTreeMap, HashSet};

use engine::Rng;

use crate::inventory::Inventory;
use crate::item::Item;
use crate::objects::{ObjKind, PlacedObject};
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Hay a silo holds when its data gives no `HayCapacity`.
pub const DEFAULT_SILO_HAY: i32 = 240;

impl World {
    /// Hay storage across finished buildings.
    pub fn hay_capacity(&self) -> i32 {
        self.buildings
            .iter()
            .filter(|b| b.days_left == 0)
            .map(|b| match self.buildings_data.get(&b.kind).map(|d| d.hay_capacity).filter(|c| *c > 0) {
                Some(c) => c,
                None if b.kind.eq_ignore_ascii_case("Silo") => DEFAULT_SILO_HAY,
                None => 0,
            })
            .sum()
    }

    /// Buildings an upgrade can be applied to.
    pub fn upgradable(&self, kind: &str) -> Vec<u32> {
        let Some(base) = self.buildings_data.get(kind).and_then(|d| d.building_to_upgrade.clone()) else { return Vec::new() };
        self.buildings.iter().filter(|b| b.kind == base && b.days_left == 0 && b.upgrade_to.is_none()).map(|b| b.id).collect()
    }

    /// Start upgrading a building (`kind` names the upgraded building).
    pub fn upgrade_building(&mut self, kind: &str, id: Option<u32>) -> Result<u32, String> {
        let data = self.buildings_data.get(kind).cloned().ok_or("Unknown building")?;
        let candidates = self.upgradable(kind);
        let id = match id {
            Some(i) if candidates.contains(&i) => i,
            Some(_) => return Err("That building can't be upgraded to this.".into()),
            None => *candidates.first().ok_or_else(|| format!("You need a {} to upgrade.", data.building_to_upgrade.clone().unwrap_or_default()))?,
        };
        if self.farmer.money < data.build_cost {
            return Err("Not enough money.".into());
        }
        for m in &data.build_materials {
            let (_, mid) = crate::item::split_qualified(&m.item_id);
            if self.farmer.inventory.count(mid) < m.amount {
                return Err("Not enough materials.".into());
            }
        }
        self.farmer.money -= data.build_cost;
        for m in &data.build_materials {
            let (_, mid) = crate::item::split_qualified(&m.item_id);
            self.farmer.inventory.remove_object(mid, m.amount);
        }
        let days = if data.magical_construction { 0 } else { data.build_days.max(0) };
        if let Some(b) = self.buildings.iter_mut().find(|b| b.id == id) {
            b.upgrade_to = Some(kind.to_string());
            b.days_left = days;
        }
        if days == 0 {
            self.apply_upgrade(id);
        }
        self.sound("axe");
        Ok(id)
    }

    /// Finish an upgrade: the building becomes the new kind, its interior
    /// is rebuilt from the new map with its contents kept.
    pub(crate) fn apply_upgrade(&mut self, id: u32) {
        let Some(bi) = self.buildings.iter().position(|b| b.id == id) else { return };
        let Some(kind) = self.buildings[bi].upgrade_to.take() else { return };
        let size = self.buildings_data.get(&kind).map(|d| (d.size.x.max(1), d.size.y.max(1))).unwrap_or(self.buildings[bi].size);
        self.buildings[bi].kind = kind;
        self.buildings[bi].size = size;
        self.buildings[bi].days_left = 0;
        self.finish_building(id);
        self.rebuild_location_edges();
    }

    /// Items placed inside a newly built building (`IndoorItems`).
    pub(crate) fn place_indoor_items(&mut self, id: u32) {
        let Some(b) = self.buildings.iter().find(|b| b.id == id).cloned() else { return };
        let (Some(interior), Some(data)) = (b.interior.clone(), self.buildings_data.get(&b.kind).cloned()) else { return };
        for add in data.indoor_items {
            let t = TilePos::new(add.tile.x, add.tile.y);
            let Some(item) = self.items.create(&add.item_id, 1) else { continue };
            let Some(loc) = self.locations.get_mut(&interior) else { continue };
            if loc.state.objects.contains_key(&t) {
                continue;
            }
            let obj = match &item {
                Item::Big { id, .. } => {
                    let kind = if self.items.machines.contains_key(&format!("(BC){id}")) { ObjKind::Machine { output: None, minutes_left: 0, ready: false } } else { ObjKind::Placed };
                    PlacedObject::new(id, true, kind)
                }
                Item::Object { id, .. } => PlacedObject::new(id, false, ObjKind::Placed),
                _ => continue,
            };
            loc.state.objects.insert(t, obj);
        }
    }

    /// Tiles the farm's buildings block (collision maps where given).
    pub(crate) fn building_blocked_tiles(&self) -> HashSet<TilePos> {
        let mut out = HashSet::new();
        for b in &self.buildings {
            match self.buildings_data.get(&b.kind).and_then(|d| d.collision_map.clone()) {
                Some(map) => {
                    for (dy, row) in map.lines().map(str::trim).filter(|r| !r.is_empty()).enumerate() {
                        for (dx, c) in row.chars().enumerate() {
                            if c == 'X' || c == 'x' {
                                out.insert(TilePos::new(b.tile.x + dx as i32, b.tile.y + dy as i32));
                            }
                        }
                    }
                }
                None => out.extend(b.footprint()),
            }
        }
        out
    }

    /// An action tile of a building at `t` (farm tile), if any.
    pub(crate) fn building_action_tile(&self, t: TilePos) -> Option<(u32, String)> {
        let b = self.buildings.iter().find(|b| b.contains(t) && b.days_left == 0)?;
        let data = self.buildings_data.get(&b.kind)?;
        let rel = (t.x - b.tile.x, t.y - b.tile.y);
        data.action_tiles.iter().find(|a| (a.tile.x, a.tile.y) == rel).map(|a| (b.id, a.action.clone()))
    }

    /// Use a building chest: load the held item into a `Load` chest, or
    /// take everything from a `Collect` chest.
    pub(crate) fn use_building_chest(&mut self, id: u32, chest: &str) -> bool {
        let Some(b) = self.buildings.iter().find(|b| b.id == id).cloned() else { return false };
        let Some(data) = self.buildings_data.get(&b.kind).cloned() else { return false };
        let Some(spec) = data.chests.iter().find(|c| c.id == chest).cloned() else { return false };
        match spec.kind.as_str() {
            "Load" => {
                let slot = self.farmer.current_slot;
                let Some(held) = self.farmer.inventory.get(slot).cloned() else { return true };
                // Only what some conversion takes.
                let ok = data.item_conversions.iter().any(|c| c.source_chest == chest && self.items.matches_tags(&held, &c.required_tags));
                if !ok {
                    let msg = spec.invalid_item_message.map(|m| self.items.expand_text(&m)).unwrap_or_else(|| "That can't go in here.".into());
                    self.hud(msg);
                    return true;
                }
                let Some(bm) = self.buildings.iter_mut().find(|x| x.id == id) else { return false };
                let inv = bm.chests.entry(chest.to_string()).or_insert_with(|| Inventory::new(36));
                if !inv.can_fit(&held) {
                    self.hud("It's full.");
                    return true;
                }
                inv.add(held);
                self.farmer.inventory.take(slot);
                self.sound(spec.sound.as_deref().unwrap_or("Ship"));
                true
            }
            _ => {
                let items: Vec<Item> = self.buildings.iter_mut().find(|x| x.id == id).and_then(|bm| bm.chests.get_mut(chest)).map(|inv| inv.slots.iter_mut().filter_map(|s| s.take()).collect()).unwrap_or_default();
                if items.is_empty() {
                    self.hud("There's nothing to collect.");
                    return true;
                }
                for it in items {
                    if self.farmer.inventory.can_fit(&it) {
                        self.farmer.inventory.add(it);
                    } else if let Some(bm) = self.buildings.iter_mut().find(|x| x.id == id) {
                        bm.chests.entry(chest.to_string()).or_insert_with(|| Inventory::new(36)).add(it);
                    }
                }
                self.sound("coin");
                true
            }
        }
    }

    /// Overnight: buildings convert items between their chests.
    pub(crate) fn building_conversions_overnight(&mut self) {
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 515]));
        for bi in 0..self.buildings.len() {
            let Some(data) = self.buildings_data.get(&self.buildings[bi].kind).cloned() else { continue };
            if self.buildings[bi].days_left > 0 {
                continue;
            }
            for conv in &data.item_conversions {
                let mut done = 0;
                loop {
                    if conv.max_daily_conversions >= 0 && done >= conv.max_daily_conversions {
                        break;
                    }
                    let b = &self.buildings[bi];
                    let Some(src) = b.chests.get(&conv.source_chest) else { break };
                    let have: i32 = src.slots.iter().flatten().filter(|i| self.items.matches_tags(i, &conv.required_tags)).map(|i| i.stack()).sum();
                    if have < conv.required_count.max(1) {
                        break;
                    }
                    // Take the input.
                    let mut left = conv.required_count.max(1);
                    let items = self.items.clone();
                    let src = self.buildings[bi].chests.get_mut(&conv.source_chest).unwrap();
                    for s in src.slots.iter_mut() {
                        if left <= 0 {
                            break;
                        }
                        let Some(it) = s.as_mut() else { continue };
                        if !items.matches_tags(it, &conv.required_tags) {
                            continue;
                        }
                        let take = it.stack().min(left);
                        left -= take;
                        if it.stack() == take {
                            *s = None;
                        } else {
                            it.set_stack(it.stack() - take);
                        }
                    }
                    // Make the output.
                    for p in &conv.produced_items {
                        if !self.check_query(p.condition.as_deref(), None, None, &mut rng.clone()) {
                            continue;
                        }
                        let id = match &p.random_item_id {
                            Some(ids) if !ids.is_empty() => rng.pick(ids).cloned(),
                            _ => p.item_id.clone(),
                        };
                        let n = if p.max_stack > p.min_stack { rng.range(p.min_stack.max(1), p.max_stack + 1) } else { p.min_stack.max(1) };
                        if let Some(item) = id.and_then(|id| self.items.create(&id, n)) {
                            self.buildings[bi].chests.entry(conv.destination_chest.clone()).or_insert_with(|| Inventory::new(36)).add(item);
                        }
                    }
                    done += 1;
                }
            }
        }
    }

    /// Building chest contents (for menus).
    pub fn building_chest(&self, id: u32, chest: &str) -> Option<&Inventory> {
        self.buildings.iter().find(|b| b.id == id)?.chests.get(chest)
    }

    /// Building chests keyed by building id (save helper).
    pub fn building_chests(&self) -> BTreeMap<u32, Vec<String>> {
        self.buildings.iter().map(|b| (b.id, b.chests.keys().cloned().collect())).collect()
    }

    /// Run a building action tile at `t`.
    pub(crate) fn run_building_action(&mut self, t: TilePos) -> bool {
        let Some((id, action)) = self.building_action_tile(t) else { return false };
        let toks: Vec<&str> = action.split_whitespace().collect();
        if toks.first() == Some(&"BuildingChest") {
            if let Some(c) = toks.get(1) {
                return self.use_building_chest(id, c);
            }
        }
        if toks.first() == Some(&"Message") {
            self.emit(WorldEvent::Message(action.trim_start_matches("Message").trim().trim_matches('"').to_string()));
            return true;
        }
        self.run_action_pub(&action)
    }

    /// Effective data of a construction entry (`kind` or `kind|skin`).
    pub fn construction_entry(&self, entry: &str) -> Option<valley_content::data::BuildingData> {
        let (kind, skin) = match entry.split_once('|') {
            Some((k, s)) => (k, Some(s)),
            None => (entry, None),
        };
        let mut d = self.buildings_data.get(kind)?.clone();
        if let Some(skin) = skin {
            let sk = d.skins.iter().find(|s| s.id == skin)?.clone();
            d.texture = sk.texture.clone();
            if let Some(n) = sk.name {
                d.name = self.items.expand_text(&n);
            }
            if let Some(t) = sk.description {
                d.description = self.items.expand_text(&t);
            }
            if let Some(c) = sk.build_cost {
                d.build_cost = c;
            }
            if let Some(m) = sk.build_materials {
                d.build_materials = m;
            }
            if let Some(days) = sk.build_days {
                d.build_days = days;
            }
        }
        Some(d)
    }

    /// Skins available for a building kind: (id, name).
    pub fn building_skins(&self, kind: &str) -> Vec<(String, String)> {
        let Some(d) = self.buildings_data.get(kind) else { return Vec::new() };
        d.skins
            .iter()
            .filter(|s| self.gsq_ok(s.condition.as_deref()))
            .map(|s| (s.id.clone(), s.name.as_deref().map(|n| self.items.expand_text(n)).unwrap_or_else(|| s.id.clone())))
            .collect()
    }

    /// Extra carpenter entries for skins listed on their own.
    pub(crate) fn skin_entries(&self, kind: &str) -> Vec<String> {
        let Some(d) = self.buildings_data.get(kind) else { return Vec::new() };
        d.skins.iter().filter(|s| s.show_as_separate_construction_entry && self.gsq_ok(s.condition.as_deref())).map(|s| format!("{kind}|{}", s.id)).collect()
    }

    /// Change a building's look (`None` = the default).
    pub fn set_building_skin(&mut self, id: u32, skin: Option<&str>) -> Result<(), String> {
        let kind = self.buildings.iter().find(|b| b.id == id).map(|b| b.kind.clone()).ok_or("No such building")?;
        if let Some(s) = skin {
            if !self.building_skins(&kind).iter().any(|(k, _)| k == s) {
                return Err("That look isn't available.".into());
            }
        }
        if let Some(b) = self.buildings.iter_mut().find(|b| b.id == id) {
            b.skin = skin.map(str::to_string);
        }
        Ok(())
    }

    /// The texture a building is drawn with.
    pub fn building_texture(&self, b: &crate::farm::Building) -> Option<String> {
        let d = self.buildings_data.get(&b.kind)?;
        let skin = b.skin.as_ref().and_then(|s| d.skins.iter().find(|k| &k.id == s));
        Some(skin.map(|s| s.texture.clone()).unwrap_or_else(|| d.texture.clone()))
    }
}
