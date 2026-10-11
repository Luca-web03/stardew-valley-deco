//! Placing and picking up furniture: rotations (the held piece turns with
//! the action button; side views swap the footprint), placement
//! restrictions (indoors / outdoors / both), items set on tables, lit
//! fireplaces and lamps glowing at night.

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
        self.location_mut().state.furniture.push(PlacedFurniture { id, tile: t, footprint, walkable: info.walkable(), rotation, held: None, on: false });
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
}
