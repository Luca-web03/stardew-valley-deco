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

/// Well-known flavored item ids and their documented price formulas.
fn flavored(kind: &str) -> Option<(&'static str, fn(i32) -> i32)> {
    Some(match kind.to_ascii_lowercase().as_str() {
        "wine" => ("348", |p| p * 3),
        "juice" => ("350", |p| (p as f32 * 2.25) as i32),
        "jelly" => ("344", |p| p * 2 + 50),
        "pickle" => ("342", |p| p * 2 + 50),
        "honey" => ("340", |p| 100 + p * 2),
        _ => return None,
    })
}

impl World {
    pub fn is_placeable(&self, item: &Item) -> bool {
        match item {
            Item::Big { .. } => true,
            Item::Object { id, .. } => {
                sprinkler_shape(&self.items, id).is_some() || self.items.object(id).is_some_and(|o| o.data.category == category::CRAFTING_PLACEABLE)
            }
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } | Item::Furniture { .. } | Item::Clothing { .. } => false,
        }
    }

    /// Place the held item on tile `t` (if clear and walkable).
    pub fn place_current(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(item) = self.farmer.inventory.get(slot).cloned() else { return false };
        if !self.is_placeable(&item) {
            return false;
        }
        let loc = self.location();
        let farmer_here = self.farmer.tile() == t;
        if !loc.is_clear(t) || !tile_passable(loc, t) || farmer_here {
            return false;
        }
        let obj = match &item {
            Item::Big { id, .. } => {
                let qid = format!("(BC){id}");
                let kind = if self.is_cask(id) {
                    ObjKind::Cask { item: None, aged: 0.0 }
                } else if self.items.machines.contains_key(&qid) {
                    ObjKind::Machine { output: None, minutes_left: 0, ready: false }
                } else if self.items.big(id).is_some_and(|b| b.data.name.contains("Chest") || b.data.context_tags.iter().flatten().any(|t| t == "chest_item" || t == "storage_item")) {
                    ObjKind::Chest(Inventory::new(36))
                } else {
                    ObjKind::Placed
                };
                PlacedObject::new(id, true, kind)
            }
            Item::Object { id, .. } => {
                let kind = match sprinkler_shape(&self.items, id) {
                    Some(s) => ObjKind::Sprinkler { radius: s },
                    None => ObjKind::Placed,
                };
                PlacedObject::new(id, false, kind)
            }
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } | Item::Furniture { .. } | Item::Clothing { .. } => return false,
        };
        self.location_mut().state.objects.insert(t, obj);
        self.farmer.inventory.reduce(slot, 1);
        self.sound("woodyStep");
        true
    }

    /// Pick a placed object back up (tools hitting it). Returns true if handled.
    pub fn pickup_object(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).cloned() else { return false };
        match &o.kind {
            ObjKind::Debris { .. } | ObjKind::Forage | ObjKind::MineRock { .. } | ObjKind::Ladder { .. } | ObjKind::Shaft | ObjKind::ArtifactSpot => return false,
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
                self.farmer.inventory.add(out.clone());
                if let Some(ObjKind::Machine { output, ready, minutes_left }) = self.location_mut().state.objects.get_mut(&t).map(|o| &mut o.kind) {
                    *output = None;
                    *ready = false;
                    *minutes_left = 0;
                }
                self.sound("coin");
                self.emit(WorldEvent::Pickup(out));
                true
            }
            ObjKind::Machine { output: Some(_), .. } => {
                if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                    o.shake = 10;
                }
                true
            }
            ObjKind::Machine { output: None, .. } => self.load_machine(t, &o.qualified_id()),
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
        let raw = def.item_id.clone().unwrap_or_default();
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
                Item::Object {
                    id: id.to_string(),
                    stack: 1,
                    quality: crate::item::Quality::Normal,
                    extra: Some(Box::new(ItemExtra { preserved_from: Some(src_id), display_name: Some(format!("{src_name} {base}")), price: Some(price_fn(src_price)) })),
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
    }

    /// Morning: sprinklers water nearby soil.
    pub fn run_sprinklers(&mut self) {
        for loc in self.locations.values_mut() {
            let sprinklers: Vec<(TilePos, SprinklerShape)> = loc
                .state
                .objects
                .iter()
                .filter_map(|(p, o)| match o.kind {
                    ObjKind::Sprinkler { radius } => Some((*p, radius)),
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
            let Some(data) = self.location_data.get(&name).cloned() else { continue };
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
