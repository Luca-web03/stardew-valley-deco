//! Placeable objects: placement and pickup, chests, data-driven machines
//! (`Data/Machines`), sprinklers, fertilizer and daily forage spawning.

use engine::Rng;
use valley_content::data::{MachineOutputItem, QuantityModifier};

use crate::collision::tile_passable;
use crate::gsq::{self, QueryCtx};
use crate::inventory::Inventory;
use crate::item::{category, Item, ItemExtra, ItemDb};
use crate::objects::{ObjKind, PlacedObject, SprinklerShape};
use crate::terrain::TerrainFeature;
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Sprinkler coverage for an object id, by well-known ids or by this engine's
/// `sprinkler_cross` / `sprinkler_square1` / `sprinkler_square2` tags.
pub fn sprinkler_shape(db: &ItemDb, id: &str) -> Option<SprinklerShape> {
    match id {
        "599" => return Some(SprinklerShape::Cross),
        "621" => return Some(SprinklerShape::Square1),
        "645" => return Some(SprinklerShape::Square2),
        _ => {}
    }
    let tags = db.object(id)?.data.context_tags.clone().unwrap_or_default();
    for t in tags {
        match t.as_str() {
            "sprinkler_cross" => return Some(SprinklerShape::Cross),
            "sprinkler_square1" => return Some(SprinklerShape::Square1),
            "sprinkler_square2" => return Some(SprinklerShape::Square2),
            _ => {}
        }
    }
    None
}

/// Fertilizer effects: (quality level, speed bonus, water retention chance).
pub fn fertilizer_effect(db: &ItemDb, id: &str) -> Option<(i32, f32, f64)> {
    let known = match id {
        "368" => Some((1, 0.0, 0.0)),
        "369" => Some((2, 0.0, 0.0)),
        "919" => Some((3, 0.0, 0.0)),
        "465" => Some((0, 0.10, 0.0)),
        "466" => Some((0, 0.25, 0.0)),
        "918" => Some((0, 0.33, 0.0)),
        "370" => Some((0, 0.0, 0.33)),
        "371" => Some((0, 0.0, 0.66)),
        "920" => Some((0, 0.0, 1.0)),
        _ => None,
    };
    if known.is_some() {
        return known;
    }
    let o = db.object(id)?;
    if o.data.category != category::FERTILIZER {
        return None;
    }
    let cf = o.data.custom_fields.clone().unwrap_or_default();
    let q = cf.get("FertilizerQuality").and_then(|v| v.parse().ok()).unwrap_or(0);
    let s = cf.get("FertilizerSpeed").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let r = cf.get("FertilizerRetention").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    Some((q, s, r))
}

fn apply_modifiers(value: f32, mods: &[QuantityModifier]) -> f32 {
    let mut v = value;
    for m in mods {
        match m.modification.as_str() {
            "Add" => v += m.amount,
            "Subtract" => v -= m.amount,
            "Multiply" => v *= m.amount,
            "Divide" if m.amount != 0.0 => v /= m.amount,
            "Set" => v = m.amount,
            _ => {}
        }
    }
    v
}

/// Well-known flavored item ids and their price formulas (documented
/// ones; the newer kinds are approximations).
fn flavored(kind: &str) -> Option<(&'static str, fn(i32) -> i32)> {
    Some(match kind.to_ascii_lowercase().as_str() {
        "wine" => ("348", |p| p * 3),
        "juice" => ("350", |p| (p as f32 * 2.25) as i32),
        "jelly" => ("344", |p| p * 2 + 50),
        "pickle" => ("342", |p| p * 2 + 50),
        "honey" => ("340", |p| 100 + p * 2),
        "roe" => ("812", |p| 30 + p / 2),
        "agedroe" => ("447", |p| (30 + p / 2) * 2),
        "smokedfish" => ("SmokedFish", |p| p * 2),
        "driedfruit" => ("DriedFruit", |p| (p as f32 * 7.5) as i32 + 25),
        "driedmushroom" | "driedmushrooms" => ("DriedMushrooms", |p| (p as f32 * 7.5) as i32 + 25),
        "bait" | "specificbait" => ("SpecificBait", |_| 5),
        _ => return None,
    })
}

impl World {
    pub fn is_placeable(&self, item: &Item) -> bool {
        match item {
            Item::Big { .. } => true,
            Item::Object { id, .. } => {
                sprinkler_shape(&self.items, id).is_some()
                    || self.items.object(id).is_some_and(|o| o.data.category == category::CRAFTING_PLACEABLE)
                    || self.fence_data(id).is_some()
                    || self.floor_for_item(id).is_some()
            }
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } | Item::Furniture { .. } | Item::Clothing { .. } | Item::Trinket { .. } | Item::Decor { .. } => false,
        }
    }

    /// Place the held item on tile `t` (if clear and walkable).
    pub fn place_current(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(item) = self.farmer.inventory.get(slot).cloned() else { return false };
        if !self.is_placeable(&item) {
            return false;
        }
        if let Some(placed) = item.object_id().and_then(|id| self.place_floor(t, &id.to_string())) {
            return placed;
        }
        let loc = self.location();
        let farmer_here = self.farmer.tile() == t;
        if !loc.is_clear_for_object(t) || !tile_passable(loc, t) || farmer_here {
            return false;
        }
        let obj = match &item {
            Item::Big { id, .. } => {
                let qid = format!("(BC){id}");
                let kind = if self.is_cask(id) {
                    ObjKind::Cask { item: None, aged: 0.0 }
                } else if self.items.machines.contains_key(&qid) {
                    ObjKind::Machine { output: None, minutes_left: 0, ready: false }
                } else if self.is_auto_grabber(id) || self.items.big(id).is_some_and(|b| b.data.name.contains("Chest") || b.data.context_tags.iter().flatten().any(|t| t == "chest_item" || t == "storage_item")) {
                    ObjKind::Chest(Inventory::new(36))
                } else {
                    ObjKind::Placed
                };
                PlacedObject::new(id, true, kind)
            }
            Item::Object { id, .. } => match self.fence_object(id) {
                Some(fence) => fence,
                None => {
                    let kind = match sprinkler_shape(&self.items, id) {
                        Some(s) => ObjKind::Sprinkler { radius: s, attachment: None, fertilizer: None },
                        None => ObjKind::Placed,
                    };
                    PlacedObject::new(id, false, kind)
                }
            },
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } | Item::Furniture { .. } | Item::Clothing { .. } | Item::Trinket { .. } | Item::Decor { .. } => return false,
        };
        let fence_cue = matches!(obj.kind, ObjKind::Fence { .. }).then(|| self.fence_data(&obj.item_id).and_then(|d| d.placement_sound)).flatten();
        self.location_mut().state.objects.insert(t, obj);
        self.farmer.inventory.reduce(slot, 1);
        self.sound(fence_cue.as_deref().unwrap_or("woodyStep"));
        let here = self.current.clone();
        self.machine_auto_trigger(&here, t, "MachinePutDown", None);
        true
    }

    /// Pick a placed object back up (tools hitting it). Returns true if handled.
    pub fn pickup_object(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).cloned() else { return false };
        match &o.kind {
            ObjKind::Debris { .. } | ObjKind::Forage | ObjKind::MineRock { .. } | ObjKind::Ladder { .. } | ObjKind::Shaft | ObjKind::ArtifactSpot | ObjKind::Fence { .. } => return false,
            ObjKind::CrabPot { catch: Some(_), .. } => return false,
            ObjKind::Tapper { .. } => {
                if let Some(TerrainFeature::Tree(tr)) = self.location_mut().terrain_mut(t) {
                    tr.tapped = false;
                }
            }
            ObjKind::Chest(inv) if inv.slots.iter().any(|s| s.is_some()) => {
                if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                    o.shake = 20;
                }
                self.sound("hammer");
                return true;
            }
            ObjKind::Machine { output: Some(_), .. } | ObjKind::Cask { item: Some(_), .. } => {
                if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                    o.shake = 20;
                }
                self.sound("hammer");
                return true;
            }
            _ => {}
        }
        self.location_mut().state.objects.remove(&t);
        self.sound("hammer");
        self.spawn_drop(o.as_item(), t.world_center());
        true
    }

    /// Interact with a placed object (open chest, load/collect machine).
    pub fn interact_object(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).cloned() else { return false };
        if matches!(o.kind, ObjKind::Cask { .. }) {
            return self.cask_action(t);
        }
        if o.big && self.is_sewing_machine(&o.item_id) {
            return self.open_tailoring();
        }
        if o.big {
            if let Some(game) = self.items.big(&o.item_id).and_then(|b| b.data.custom_fields.as_ref()).and_then(|f| f.get("Port.Arcade")).cloned() {
                return self.open_arcade(&game);
            }
        }
        match o.kind {
            ObjKind::Chest(_) => {
                self.emit(WorldEvent::OpenChest(t));
                self.sound("openChest");
                true
            }
            ObjKind::Machine { output: Some(out), ready: true, .. } => {
                if !self.farmer.inventory.can_fit(&out) {
                    self.hud("Inventory full");
                    return true;
                }
                let here = self.current.clone();
                let out = self.flavor_honey(&here, t, out);
                self.farmer.inventory.add(out.clone());
                if let Some(ObjKind::Machine { output, ready, minutes_left }) = self.location_mut().state.objects.get_mut(&t).map(|o| &mut o.kind) {
                    *output = None;
                    *ready = false;
                    *minutes_left = 0;
                }
                self.sound("coin");
                self.emit(WorldEvent::Pickup(out.clone()));
                self.machine_auto_trigger(&here, t, "OutputCollected", Some(&out));
                true
            }
            ObjKind::Machine { output: Some(_), .. } => {
                if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                    o.shake = 10;
                }
                true
            }
            ObjKind::Machine { output: None, .. } => self.load_machine(t, &o.qualified_id()),
            ObjKind::Fence { .. } => self.toggle_gate(t),
            _ => false,
        }
    }

    fn load_machine(&mut self, t: TilePos, machine_qid: &str) -> bool {
        let Some(data) = self.items.machines.get(machine_qid).cloned() else { return false };
        let slot = self.farmer.current_slot;
        let Some(input) = self.farmer.inventory.get(slot).cloned() else { return false };
        // Additional fuel-like items.
        for extra in &data.additional_consumed_items {
            let (_, id) = crate::item::split_qualified(&extra.item_id);
            if self.farmer.inventory.count(id) < extra.required_count {
                let name = self.items.object(id).map(|o| o.display_name.clone()).unwrap_or_else(|| id.to_string());
                self.hud(format!("Requires {} {name}", extra.required_count));
                return true;
            }
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, t.x as i64, t.y as i64, self.clock.time as i64]));
        let input_qid = input.qualified_id();
        for rule in &data.output_rules {
            let trig = rule.triggers.iter().find(|tr| {
                (tr.trigger.is_empty() || tr.trigger.contains("ItemPlacedInMachine"))
                    && tr.required_item_id.as_ref().map_or(true, |r| r == &input_qid || crate::item::split_qualified(r).1 == input.object_id().unwrap_or(""))
                    && tr.required_tags.as_ref().map_or(true, |tags| self.items.matches_tags(&input, tags))
                    && input.stack() >= tr.required_count
            });
            let Some(trig) = trig.cloned() else { continue };
            let mail = self.farmer.mail_received.clone();
            let seen = self.farmer.events_seen.clone();
            let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng };
            let out_def: Option<MachineOutputItem> = rule.output_item.iter().find(|o| gsq::check(o.condition.as_deref(), &mut ctx)).cloned();
            let Some(out_def) = out_def else { continue };
            let Some(output) = self.make_machine_output(&out_def, &input) else { continue };
            // Consume input + extras.
            self.farmer.inventory.reduce(slot, trig.required_count);
            for extra in &data.additional_consumed_items {
                let (_, id) = crate::item::split_qualified(&extra.item_id);
                self.farmer.inventory.remove_object(id, extra.required_count);
            }
            let minutes = if rule.days_until_ready > 0 {
                // Ready at the start of the Nth day.
                crate::time::Clock::minutes_between(self.clock.time, 2600) + 240 + (rule.days_until_ready - 1) * 1440
            } else {
                rule.minutes_until_ready.max(10)
            };
            if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                o.kind = ObjKind::Machine { output: Some(output), minutes_left: minutes, ready: false };
                o.shake = 12;
            }
            self.sound("Ship");
            return true;
        }
        if input.tool().is_none() {
            let msg = data.invalid_item_message.clone().filter(|m| !m.is_empty()).unwrap_or_else(|| "That item can't be used here.".into());
            self.hud(msg);
        }
        true
    }

    fn make_machine_output(&self, def: &MachineOutputItem, input: &Item) -> Option<Item> {
        if let Some(method) = def.output_method.as_deref() {
            return self.machine_output_method(method, input);
        }
        let raw = match def.random_item_id.as_ref().filter(|v| !v.is_empty()) {
            Some(ids) => {
                let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, self.clock.time as i64, 211]));
                rng.pick(ids).cloned().unwrap_or_default()
            }
            None => def.item_id.clone().unwrap_or_default(),
        };
        let input_name = self.items.display_name(input);
        let input_price = self.items.object(input.object_id().unwrap_or("")).map(|o| o.data.price).unwrap_or(0);
        let toks: Vec<&str> = raw.split_whitespace().collect();
        let mut item = match toks.as_slice() {
            ["DROP_IN_ID"] => input.clone(),
            ["FLAVORED_ITEM", kind, src, ..] => {
                let (id, price_fn) = flavored(kind)?;
                let src_id = if *src == "DROP_IN_ID" { input.object_id().unwrap_or("").to_string() } else { crate::item::split_qualified(src).1.to_string() };
                let base = self.items.object(id).map(|o| o.display_name.clone()).unwrap_or_else(|| kind.to_string());
                let src_name = self.items.object(&src_id).map(|o| o.display_name.clone()).unwrap_or_default();
                let src_price = self.items.object(&src_id).map(|o| o.data.price).unwrap_or(0);
                // "-1" (or an unknown source) makes the plain item, e.g. wild honey.
                if src_name.is_empty() {
                    let mut plain = self.items.create(id, 1).unwrap_or_else(|| Item::object(id, 1));
                    if let Item::Object { extra, .. } = &mut plain {
                        let wild = kind.eq_ignore_ascii_case("honey").then(|| format!("Wild {base}"));
                        *extra = Some(Box::new(ItemExtra { preserved_from: None, display_name: wild, price: Some(price_fn(0).max(self.items.object(id).map(|o| o.data.price).unwrap_or(0))), ..Default::default() }));
                    }
                    return Some(plain);
                }
                Item::Object {
                    id: id.to_string(),
                    stack: 1,
                    quality: crate::item::Quality::Normal,
                    extra: Some(Box::new(ItemExtra { preserved_from: Some(src_id), display_name: Some(format!("{src_name} {base}")), price: Some(price_fn(src_price)), ..Default::default() })),
                }
            }
            [id, ..] => self.items.create(id, 1)?,
            [] => return None,
        };
        let min = def.min_stack.max(1);
        let max = def.max_stack.max(min);
        item.set_stack(if max > min { min + (self.clock.time as i32 % (max - min + 1)) } else { min });
        if let Item::Object { quality, extra, .. } = &mut item {
            if def.copy_quality {
                *quality = input.quality();
            }
            if let Some(q) = def.quality {
                *quality = crate::item::Quality::from_level(q);
            }
            let mut ex = extra.take().map(|b| *b).unwrap_or_default();
            if def.copy_price || def.price_modifiers.is_some() {
                let base = if def.copy_price { input_price } else { ex.price.unwrap_or(0) };
                let mods = def.price_modifiers.clone().unwrap_or_default();
                ex.price = Some(apply_modifiers(base as f32, &mods).round() as i32);
            }
            if def.preserve_id.is_some() {
                ex.preserved_from = input.object_id().map(String::from);
            }
            if let Some(n) = def.object_display_name.as_ref().or(def.object_internal_name.as_ref()) {
                let n = self.items.expand_text(n).replace("{0}", &input_name).replace("%PRESERVED_DISPLAY_NAME", &input_name).replace("%DISPLAY_NAME", &input_name);
                ex.display_name = Some(n);
            }
            if ex != ItemExtra::default() {
                *extra = Some(Box::new(ex));
            }
        }
        Some(item)
    }

    /// Honey takes the flavor of the closest fully grown flower crop within
    /// five tiles of the hive when it is collected; else it is wild honey.
    pub fn flavor_honey(&self, loc_name: &str, hive: TilePos, item: Item) -> Item {
        let honey_id = flavored("honey").map(|f| f.0).unwrap_or("340");
        let plain = match &item {
            Item::Object { id, extra, .. } => id == honey_id && extra.as_ref().map_or(true, |e| e.preserved_from.is_none()),
            _ => false,
        };
        let Some(loc) = self.locations.get(loc_name).filter(|_| plain) else { return item };
        let mut best: Option<(i32, String)> = None;
        for (p, tf) in &loc.state.terrain {
            let d = (p.x - hive.x).abs() + (p.y - hive.y).abs();
            if d > 5 || best.as_ref().is_some_and(|b| b.0 <= d) {
                continue;
            }
            let crate::terrain::TerrainFeature::HoeDirt(dirt) = tf else { continue };
            let Some(crop) = dirt.crop.as_ref().filter(|c| !c.dead) else { continue };
            let Some(data) = self.items.crops.get(&crop.seed_id) else { continue };
            if !crop.is_ready(data) {
                continue;
            }
            let flower = crate::item::split_qualified(&data.harvest_item_id).1.to_string();
            if self.items.object(&flower).is_some_and(|o| o.data.category == -80) {
                best = Some((d, flower));
            }
        }
        let Some((_, flower)) = best else { return item };
        let name = self.items.object(&flower).map(|o| o.display_name.clone()).unwrap_or_default();
        let price = self.items.object(&flower).map(|o| o.data.price).unwrap_or(0);
        let base = self.items.object(honey_id).map(|o| o.display_name.clone()).unwrap_or_else(|| "Honey".into());
        let mut item = item;
        if let Item::Object { extra, .. } = &mut item {
            *extra = Some(Box::new(ItemExtra { preserved_from: Some(flower), display_name: Some(format!("{name} {base}")), price: Some(100 + price * 2), ..Default::default() }));
        }
        item
    }

    /// Code-defined outputs (`OutputMethod`) this engine knows: the seed
    /// maker. Others produce nothing.
    fn machine_output_method(&self, method: &str, input: &Item) -> Option<Item> {
        let name = method.rsplit(':').next().unwrap_or(method).trim();
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, self.clock.time as i64, 223]));
        match name {
            "OutputSeedMaker" => {
                let id = input.object_id()?;
                // Rare extras (approx. chances).
                if rng.chance(0.005) && self.items.objects.contains_key("499") {
                    return Some(Item::object("499", 1));
                }
                if rng.chance(0.02) && self.items.objects.contains_key("770") {
                    return Some(Item::object("770", rng.range(1, 5)));
                }
                let seed = self.items.crops.iter().find(|(_, c)| crate::item::split_qualified(&c.harvest_item_id).1 == id).map(|(k, _)| k.clone())?;
                Some(Item::object(&seed, rng.range(1, 4)))
            }
            _ => None,
        }
    }

    /// Fire a machine's input-less rules (`DayUpdate`, `MachinePutDown`,
    /// `OutputCollected`) at `t` in `loc_name`. `last` is the output just
    /// collected (the input for `OutputCollected` rules).
    pub(crate) fn machine_auto_trigger(&mut self, loc_name: &str, t: TilePos, trigger: &str, last: Option<&Item>) -> bool {
        let Some(o) = self.locations.get(loc_name).and_then(|l| l.object(t)).cloned() else { return false };
        if !matches!(o.kind, ObjKind::Machine { output: None, .. }) {
            return false;
        }
        let Some(data) = self.items.machines.get(&o.qualified_id()).cloned() else { return false };
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, t.x as i64, t.y as i64, 227]));
        let mail = self.farmer.mail_received.clone();
        let seen = self.farmer.events_seen.clone();
        for rule in &data.output_rules {
            let fires = rule.triggers.iter().any(|tr| {
                let named = tr.trigger.split(',').any(|x| x.trim() == trigger);
                let item_ok = match (trigger, last) {
                    ("OutputCollected", Some(l)) => {
                        tr.required_item_id.as_ref().map_or(true, |r| *r == l.qualified_id() || crate::item::split_qualified(r).1 == l.object_id().unwrap_or(""))
                            && tr.required_tags.as_ref().map_or(true, |tags| self.items.matches_tags(l, tags))
                    }
                    ("OutputCollected", None) => false,
                    _ => tr.required_item_id.is_none() && tr.required_tags.is_none(),
                };
                let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: loc_name, rng: &mut rng };
                named && item_ok && gsq::check(tr.condition.as_deref(), &mut ctx)
            });
            if !fires {
                continue;
            }
            let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: loc_name, rng: &mut rng };
            let Some(def) = rule.output_item.iter().find(|d| gsq::check(d.condition.as_deref(), &mut ctx)).cloned() else { continue };
            let input = last.cloned().unwrap_or_else(|| Item::big(&o.item_id, 1));
            let Some(output) = self.make_machine_output(&def, &input) else { continue };
            let minutes = if rule.days_until_ready > 0 {
                crate::time::Clock::minutes_between(self.clock.time, 2600) + 240 + (rule.days_until_ready - 1) * 1440
            } else {
                rule.minutes_until_ready.max(10)
            };
            if let Some(obj) = self.locations.get_mut(loc_name).and_then(|l| l.state.objects.get_mut(&t)) {
                obj.kind = ObjKind::Machine { output: Some(output), minutes_left: minutes, ready: false };
            }
            return true;
        }
        false
    }

    /// Morning: idle machines with `DayUpdate` rules start working.
    pub(crate) fn machines_day_update(&mut self) {
        let idle: Vec<(String, TilePos)> = self
            .locations
            .iter()
            .flat_map(|(name, l)| l.state.objects.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Machine { output: None, .. })).map(move |(p, _)| (name.clone(), *p)))
            .collect();
        for (name, p) in idle {
            self.machine_auto_trigger(&name, p, "DayUpdate", None);
        }
    }

    /// Advance all machines by game minutes.
    pub fn advance_machines(&mut self, minutes: i32) {
        let mut ready_here = false;
        for (name, loc) in self.locations.iter_mut() {
            for o in loc.state.objects.values_mut() {
                if let ObjKind::Machine { output: Some(_), minutes_left, ready } = &mut o.kind {
                    if !*ready {
                        *minutes_left -= minutes;
                        if *minutes_left <= 0 {
                            *ready = true;
                            if *name == self.current {
                                ready_here = true;
                            }
                        }
                    }
                }
            }
        }
        if ready_here {
            self.sound("dwop");
        }
        self.hatch_incubators();
    }

    /// Morning: sprinklers water nearby soil.
    pub fn run_sprinklers(&mut self) {
        self.enrich_soil();
        let nozzles: Vec<String> = self.items.objects.keys().filter(|k| self.is_nozzle(k)).cloned().collect();
        for loc in self.locations.values_mut() {
            let sprinklers: Vec<(TilePos, SprinklerShape)> = loc
                .state
                .objects
                .iter()
                .filter_map(|(p, o)| match &o.kind {
                    ObjKind::Sprinkler { radius, attachment, .. } => Some((*p, crate::automation::sprinkler_area(*radius, attachment.as_ref().is_some_and(|a| nozzles.contains(a))))),
                    _ => None,
                })
                .collect();
            for (p, shape) in sprinklers {
                for t in shape.tiles(p) {
                    if let Some(TerrainFeature::HoeDirt(d)) = loc.state.terrain.get_mut(&t) {
                        d.watered = true;
                    }
                }
            }
        }
    }

    /// Apply the held fertilizer to tilled soil at `t`.
    pub fn apply_fertilizer(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(id) = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from) else { return false };
        let Some((_, speed, _)) = fertilizer_effect(&self.items, &id) else { return false };
        let crops = self.items.crops.clone();
        let applied = {
            let Some(TerrainFeature::HoeDirt(d)) = self.location_mut().terrain_mut(t) else { return false };
            if d.fertilizer.is_some() {
                None
            } else {
                match &mut d.crop {
                    Some(c) if c.phase > 0 || c.day_in_phase > 0 => None,
                    Some(c) => {
                        if speed > 0.0 {
                            if let Some(data) = crops.get(&c.seed_id) {
                                c.apply_speed_bonus(data, speed);
                            }
                        }
                        d.fertilizer = Some(id.clone());
                        Some(())
                    }
                    None => {
                        d.fertilizer = Some(id.clone());
                        Some(())
                    }
                }
            }
        };
        if applied.is_none() {
            self.hud("Can't use fertilizer here.");
            return true;
        }
        self.farmer.inventory.reduce(slot, 1);
        self.sound("dirtyHit");
        true
    }

    /// Quality level contributed by the fertilizer in a soil tile.
    pub fn fertilizer_quality(&self, fert: Option<&String>) -> i32 {
        fert.and_then(|f| fertilizer_effect(&self.items, f)).map(|(q, _, _)| q).unwrap_or(0)
    }

    /// Daily forage spawning per `Data/Locations` forage lists.
    pub fn spawn_forage(&mut self) {
        let season = self.clock.season;
        let names: Vec<String> = self.locations.keys().cloned().collect();
        for name in names {
            let Some(data) = self.location_data_for(&name).cloned() else { continue };
            if data.forage.is_empty() {
                continue;
            }
            let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, name.len() as i64, 11]));
            let candidates: Vec<_> = data
                .forage
                .iter()
                .filter(|f| f.season.as_deref().map_or(true, |s| crate::time::Season::parse(s) == Some(season)))
                .cloned()
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let (lo, hi) = if data.max_daily_forage_spawn > 0 { (data.min_daily_forage_spawn, data.max_daily_forage_spawn) } else { (1, 4) };
            let mut count = rng.range(lo, hi + 1);
            let Some(loc) = self.locations.get_mut(&name) else { continue };
            let existing = loc.state.objects.values().filter(|o| matches!(o.kind, ObjKind::Forage)).count() as i32;
            count = count.min(data.max_spawned_forage_at_once - existing);
            let (w, h) = (loc.width(), loc.height());
            let mut placed = 0;
            for _ in 0..count.max(0) * 20 {
                if placed >= count {
                    break;
                }
                let p = TilePos::new(rng.range(0, w), rng.range(0, h));
                let spawnable = loc.tile_prop("Back", p, "Spawnable").is_some_and(|v| crate::location::truthy(&v))
                    || (loc.is_diggable(p) && loc.ground_type(p).as_deref() == Some("Grass"));
                if !spawnable || !loc.is_clear(p) || !tile_passable(loc, p) {
                    continue;
                }
                let Some(f) = rng.pick(&candidates).cloned() else { break };
                if !rng.chance(f.chance.unwrap_or(1.0)) {
                    continue;
                }
                let Some(id) = f.item_id.as_deref() else { continue };
                let (_, id) = crate::item::split_qualified(id);
                if !self.items.objects.contains_key(id) {
                    continue;
                }
                loc.state.objects.insert(p, PlacedObject::forage(id));
                placed += 1;
            }
        }
    }
}
