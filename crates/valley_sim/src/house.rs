//! Farmhouse upgrades: Maps/FarmHouse1.. replace the farmhouse map.

use crate::world::{World, WorldEvent};

impl World {
    /// Cost of the next upgrade: (money, wood-like material id, amount).
    /// Values follow the publicly documented upgrade costs; the material
    /// falls back to wood when hardwood isn't available.
    pub fn house_upgrade_cost(&self) -> Option<(i32, String, i32)> {
        let next = self.farmer.house_level + 1;
        if !self.house_maps.contains_key(&next) {
            return None;
        }
        let hardwood = if self.items.objects.contains_key("709") { "709" } else { "388" };
        Some(match next {
            1 => (10_000, "388".to_string(), 450),
            2 => (65_000, hardwood.to_string(), 100),
            _ => (100_000, "388".to_string(), 0),
        })
    }

    pub fn start_house_upgrade(&mut self) -> Result<(), String> {
        if self.farmer.house_upgrade_days > 0 {
            return Err("Your house is already being worked on.".into());
        }
        let (money, mat, n) = self.house_upgrade_cost().ok_or("Your house can't be upgraded further.")?;
        if self.farmer.money < money || self.farmer.inventory.count(&mat) < n {
            let name = self.items.object(&mat).map(|o| o.display_name.clone()).unwrap_or(mat.clone());
            return Err(format!("Needs {money}g and {n} {name}."));
        }
        self.farmer.money -= money;
        self.farmer.inventory.remove_object(&mat, n);
        self.farmer.house_upgrade_days = 3;
        self.sound("axe");
        Ok(())
    }

    /// Overnight: finish an upgrade and swap the map in.
    pub(crate) fn house_overnight(&mut self) {
        if self.farmer.house_upgrade_days <= 0 {
            return;
        }
        self.farmer.house_upgrade_days -= 1;
        if self.farmer.house_upgrade_days > 0 {
            return;
        }
        self.farmer.house_level += 1;
        self.apply_house_map();
        self.emit(WorldEvent::Hud("Your farmhouse upgrade is finished!".into()));
    }

    /// Install the map for the current house level (keeps placed things).
    pub(crate) fn apply_house_map(&mut self) {
        let lvl = self.farmer.house_level;
        let Some(map) = self.house_maps.get(&lvl).cloned() else { return };
        if let Some(h) = self.locations.get_mut("FarmHouse") {
            h.warps = map.prop("Warp").map(|w| crate::location::parse_warps(&w)).unwrap_or_default();
            h.map = map;
        }
        self.bed_tile = self.find_bed_tile_pub();
        self.build_location_edges_pub();
    }
}
