//! Placing and picking up furniture: rotations (the held piece turns with
//! the action button; side views swap the footprint), placement
//! restrictions (indoors / outdoors / both), items set on tables, lit
//! fireplaces, lamps glowing at night and fish tanks (fish swim in them,
//! drawn from `Data/AquariumFish`).

use crate::item::Item;
use crate::location::PlacedFurniture;
use crate::world::World;
use crate::TilePos;

impl World {
    /// Action button with furniture involved: turn the held piece, set an
    /// item on a table or take it back, light a fireplace.
    pub(crate) fn furniture_action(&mut self, t: TilePos) -> bool {
        if let Some(Item::Furniture { id }) = self.farmer.current_item().cloned() {
            let rotations = self.items.furniture.get(&id).map_or(1, |f| f.rotations);
            if rotations > 1 {
                self.farmer.furniture_rotation = (self.farmer.furniture_rotation + 1) % rotations;
                self.sound("dwop");
                return true;
            }
            return self.place_furniture(t);
        }
        let Some(idx) = self.location().state.furniture.iter().position(|f| f.covers(t)) else { return false };
        let f = self.location().state.furniture[idx].clone();
        let Some(info) = self.items.furniture.get(&f.id).cloned() else { return false };
        if info.is_table() {
            if let Some(item) = f.held.clone() {
                if !self.farmer.inventory.can_fit(&item) {
                    self.hud("Inventory full");
                    return true;
                }
                self.location_mut().state.furniture[idx].held = None;
                self.give_item(item);
                self.sound("coin");
                return true;
            }
            let slot = self.farmer.current_slot;
            let Some(held) = self.farmer.current_item().cloned() else { return false };
            if matches!(held, Item::Object { .. } | Item::Big { .. }) {
                let one = {
                    let mut i = held.clone();
                    i.set_stack(1);
                    i
                };
                self.location_mut().state.furniture[idx].held = Some(one);
                self.farmer.inventory.reduce(slot, 1);
                self.sound("woodyStep");
                return true;
            }
            return false;
        }
        if info.is_mannequin() {
            return self.dress_mannequin(idx);
        }
        if info.is_fish_tank() {
            let slot = self.farmer.current_slot;
            let held = self.farmer.current_item().cloned();
            let is_fish = held.as_ref().is_some_and(|i| matches!(i, Item::Object { .. }) && self.items.object(i.object_id().unwrap_or("")).is_some_and(|o| o.data.category == -4));
            if is_fish {
                if f.contents.len() >= info.tank_capacity() {
                    self.hud("The tank is full.");
                    return true;
                }
                let mut one = held.unwrap();
                one.set_stack(1);
                self.location_mut().state.furniture[idx].contents.push(one);
                self.farmer.inventory.reduce(slot, 1);
                self.sound("waterSlosh");
                return true;
            }
            if held.is_none() {
                if let Some(fish) = self.location_mut().state.furniture[idx].contents.pop() {
                    self.give_item(fish);
                    self.sound("fishSlap");
                    return true;
                }
            }
            return false;
        }
        if info.is_fireplace() {
            let on = !f.on;
            self.location_mut().state.furniture[idx].on = on;
            self.sound(if on { "fireball" } else { "woodyStep" });
            return true;
        }
        false
    }

    /// Place held furniture with its top-left footprint tile at `t`.
    pub(crate) fn place_furniture(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(Item::Furniture { id }) = self.farmer.inventory.get(slot).cloned() else { return false };
        let Some(info) = self.items.furniture.get(&id).cloned() else { return false };
        let outdoors = self.location().outdoors;
        if outdoors && !info.allowed_outdoors() {
            self.hud("Furniture can only be placed indoors.");
            return true;
        }
        if !outdoors && !info.allowed_indoors() {
            self.hud("This can only be placed outdoors.");
            return true;
        }
        let rotation = self.farmer.furniture_rotation % info.rotations.max(1);
        let footprint = info.footprint_at(rotation);
        let me = self.farmer.tile();
        let loc = self.location();
        for dy in 0..footprint.1 {
            for dx in 0..footprint.0 {
                let p = TilePos::new(t.x + dx, t.y + dy);
                let ok = if info.walkable() {
                    loc.in_bounds(p) && loc.has_tile("Back", p) && !loc.has_tile("Buildings", p) && loc.furniture_at(p).is_none()
                } else {
                    loc.is_clear(p) && crate::collision::tile_passable(loc, p) && p != me
                };
                if !ok {
                    self.hud("Can't place that here.");
                    return true;
                }
            }
        }
        self.location_mut().state.furniture.push(PlacedFurniture { id, tile: t, footprint, walkable: info.walkable(), rotation, held: None, on: false, contents: Vec::new() });
        self.farmer.inventory.reduce(slot, 1);
        self.farmer.furniture_rotation = 0;
        self.sound("woodyStep");
        true
    }

    /// Pick up furniture at `t` (empty hands). A table hands back what's on
    /// it first.
    pub(crate) fn pickup_furniture(&mut self, t: TilePos) -> bool {
        if self.farmer.current_item().is_some() {
            return false;
        }
        let Some(idx) = self.location().state.furniture.iter().position(|f| f.covers(t)) else { return false };
        let f = self.location().state.furniture[idx].clone();
        if !f.contents.is_empty() {
            if self.items.furniture.get(&f.id).is_some_and(|i| i.is_mannequin()) {
                // Undress it first.
                let clothes = std::mem::take(&mut self.location_mut().state.furniture[idx].contents);
                for c in clothes {
                    self.give_item(c);
                }
                self.sound("dwop");
                return true;
            }
            self.hud("Take the fish out first.");
            return true;
        }
        if let Some(on_top) = f.held.clone() {
            self.location_mut().state.furniture[idx].held = None;
            self.give_item(on_top);
            self.sound("coin");
            return true;
        }
        let item = Item::Furniture { id: f.id };
        if !self.farmer.inventory.can_fit(&item) {
            self.hud("Inventory full");
            return true;
        }
        self.location_mut().state.furniture.remove(idx);
        self.farmer.inventory.add(item);
        self.sound("coin");
        true
    }

    /// Tiles lit by furniture here: lamps at night, lit fireplaces.
    pub fn furniture_lights(&self, location: &str) -> Vec<TilePos> {
        let Some(loc) = self.locations.get(location) else { return Vec::new() };
        let night = self.clock.time >= crate::time::dusk(self.clock.season);
        loc.state
            .furniture
            .iter()
            .filter(|f| self.items.furniture.get(&f.id).is_some_and(|i| (i.is_lamp() && night) || (i.is_fireplace() && f.on)))
            .map(|f| TilePos::new(f.tile.x + f.footprint.0 / 2, f.tile.y))
            .collect()
    }

    /// Mannequins: clothing held goes on (swapping what was in that slot);
    /// empty hands swap outfits with the farmer.
    fn dress_mannequin(&mut self, idx: usize) -> bool {
        let slot_of = |i: &Item| -> Option<u8> {
            match i {
                Item::Clothing { slot, .. } => Some(match slot {
                    crate::item::ClothingSlot::Hat => 0,
                    crate::item::ClothingSlot::Shirt => 1,
                    crate::item::ClothingSlot::Pants => 2,
                }),
                Item::Boots { .. } => Some(3),
                _ => None,
            }
        };
        let held = self.farmer.current_item().cloned();
        match held {
            Some(item) => {
                let Some(k) = slot_of(&item) else { return false };
                let hand = self.farmer.current_slot;
                self.farmer.inventory.take(hand);
                let worn = &mut self.location_mut().state.furniture[idx].contents;
                let old = worn.iter().position(|w| slot_of(w) == Some(k)).map(|i| worn.remove(i));
                worn.push(item);
                if let Some(o) = old {
                    self.give_item(o);
                }
                self.sound("dwop");
                true
            }
            None => {
                let worn = std::mem::take(&mut self.location_mut().state.furniture[idx].contents);
                let f = &mut self.farmer;
                let mut mine: Vec<Item> = [f.hat.take(), f.shirt_item.take(), f.pants_item.take(), f.boots.take()].into_iter().flatten().collect();
                for w in worn {
                    match slot_of(&w) {
                        Some(0) => self.farmer.hat = Some(w),
                        Some(1) => self.farmer.shirt_item = Some(w),
                        Some(2) => self.farmer.pants_item = Some(w),
                        Some(3) => self.farmer.boots = Some(w),
                        _ => mine.push(w),
                    }
                }
                self.location_mut().state.furniture[idx].contents = mine;
                self.sound("dwop");
                true
            }
        }
    }

    /// Debug: put furniture or a big craftable straight onto a tile of the
    /// current location (no checks, nothing taken from the inventory).
    pub fn debug_place(&mut self, qualified: &str, t: TilePos, rotation: i32) -> Result<(), String> {
        match self.items.create(qualified, 1) {
            Some(Item::Furniture { id }) => {
                let info = self.items.furniture.get(&id).cloned().ok_or("unknown furniture")?;
                let rotation = rotation.rem_euclid(info.rotations.max(1));
                let footprint = info.footprint_at(rotation);
                self.location_mut().state.furniture.push(PlacedFurniture { id, tile: t, footprint, walkable: info.walkable(), rotation, held: None, on: false, contents: Vec::new() });
                Ok(())
            }
            Some(Item::Big { id, .. }) => {
                let kind = if self.items.machines.contains_key(&format!("(BC){id}")) {
                    crate::objects::ObjKind::Machine { output: None, minutes_left: 0, ready: false }
                } else {
                    crate::objects::ObjKind::Placed
                };
                self.location_mut().state.objects.insert(t, crate::objects::PlacedObject::new(&id, true, kind));
                Ok(())
            }
            _ => Err(format!("can't place {qualified}")),
        }
    }

    /// Debug: sit on the seat at `t`.
    pub fn debug_sit(&mut self, t: TilePos) -> bool {
        self.try_sit(t)
    }
}

