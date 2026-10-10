//! Placing and picking up furniture.

use crate::item::Item;
use crate::location::PlacedFurniture;
use crate::world::World;
use crate::TilePos;

impl World {
    /// Place held furniture with its top-left footprint tile at `t`.
    pub(crate) fn place_furniture(&mut self, t: TilePos) -> bool {
        let slot = self.farmer.current_slot;
        let Some(Item::Furniture { id }) = self.farmer.inventory.get(slot).cloned() else { return false };
        let Some(info) = self.items.furniture.get(&id).cloned() else { return false };
        if self.location().outdoors {
            self.hud("Furniture can only be placed indoors.");
            return true;
        }
        let me = self.farmer.tile();
        let loc = self.location();
        for dy in 0..info.footprint.1 {
            for dx in 0..info.footprint.0 {
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
        self.location_mut().state.furniture.push(PlacedFurniture { id, tile: t, footprint: info.footprint, walkable: info.walkable() });
        self.farmer.inventory.reduce(slot, 1);
        self.sound("woodyStep");
        true
    }

    /// Pick up furniture at `t` (empty hands).
    pub(crate) fn pickup_furniture(&mut self, t: TilePos) -> bool {
        if self.farmer.current_item().is_some() {
            return false;
        }
        let Some(idx) = self.location().state.furniture.iter().position(|f| f.covers(t)) else { return false };
        let f = self.location().state.furniture[idx].clone();
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
}
