//! Harvest huts: a farm building (id `Junimo Hut` or the `Port.HarvestHut`
//! custom field) that gathers ripe, hand-harvested crops within a radius
//! each night and keeps them in its own storage. The radius and capacity
//! are approximations.

use crate::inventory::Inventory;
use crate::item::Item;
use crate::terrain::TerrainFeature;
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Tiles from the hut (in every direction) it harvests.
pub const HUT_RADIUS: i32 = 8;
pub const HUT_SLOTS: usize = 36;

/// Chest-menu "tile" addressing building `id`'s storage.
pub fn hut_tile(id: u32) -> TilePos {
    TilePos::new(id as i32, -2)
}

impl World {
    pub fn is_harvest_hut(&self, kind: &str) -> bool {
        kind == "Junimo Hut" || self.buildings_data.get(kind).is_some_and(|d| d.custom_fields.as_ref().is_some_and(|f| f.contains_key("Port.HarvestHut")))
    }

    /// Action on a hut: open its storage.
    pub(crate) fn interact_hut(&mut self, t: TilePos) -> bool {
        if self.current != "Farm" {
            return false;
        }
        let Some(b) = self.buildings.iter().find(|b| b.contains(t)) else { return false };
        if !self.is_harvest_hut(&b.kind) || b.days_left > 0 {
            return false;
        }
        let id = b.id;
        self.sound("openChest");
        self.emit(WorldEvent::OpenChest(hut_tile(id)));
        true
    }

    /// Night: each hut harvests ripe crops near it into its storage.
    pub(crate) fn huts_overnight(&mut self) {
        let level = self.farmer.skills.level(crate::farmer::Skill::Farming);
        let items = self.items.clone();
        let huts: Vec<(usize, TilePos, (i32, i32))> = self
            .buildings
            .iter()
            .enumerate()
            .filter(|(_, b)| self.is_harvest_hut(&b.kind) && b.days_left == 0)
            .map(|(i, b)| (i, b.tile, b.size))
            .collect();
        for (bi, tile, size) in huts {
            let mut rng = self.day_rng(600 + bi as i64);
            let mut store = self.buildings[bi].storage.take().unwrap_or_else(|| Inventory::new(HUT_SLOTS));
            if let Some(farm) = self.locations.get_mut("Farm") {
                let (cx, cy) = (tile.x + size.0 / 2, tile.y + size.1 / 2);
                for y in cy - HUT_RADIUS..=cy + HUT_RADIUS {
                    for x in cx - HUT_RADIUS..=cx + HUT_RADIUS {
                        let Some(TerrainFeature::HoeDirt(d)) = farm.state.terrain.get_mut(&TilePos::new(x, y)) else { continue };
                        let fert = d.fertilizer.as_ref().and_then(|f| crate::machines::fertilizer_effect(&items, f)).map(|(q, _, _)| q).unwrap_or(0);
                        let Some(crop) = d.crop.as_mut() else { continue };
                        let Some(data) = items.crops.get(&crop.seed_id) else { continue };
                        if crop.dead || !crop.is_ready(data) || data.harvest_method.eq_ignore_ascii_case("Scythe") {
                            continue;
                        }
                        // A full hut leaves crops in the field.
                        if !store.can_fit(&Item::object(&data.harvest_item_id, 1)) {
                            continue;
                        }
                        if let Some((got, remove)) = crop.harvest(data, &mut rng, level, fert) {
                            if remove {
                                d.crop = None;
                            }
                            for it in got {
                                let _ = store.add(it);
                            }
                        }
                    }
                }
            }
            self.buildings[bi].storage = Some(store);
        }
    }
}
