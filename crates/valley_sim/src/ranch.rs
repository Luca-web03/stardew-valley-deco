//! Grass, hay and silos; storms and lightning rods.

use engine::Rng;

use crate::item::Item;
use crate::objects::{ObjKind, PlacedObject};
use crate::terrain::{Grass, TerrainFeature};
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Hay capacity per silo (documented).
pub const SILO_CAPACITY: i32 = 240;
const RODS: &[&str] = &["9", "Dev_LightningRod"];
const BATTERY: &[&str] = &["787", "Dev_Battery"];

impl World {
    pub fn silo_capacity(&self) -> i32 {
        self.buildings.iter().filter(|b| b.days_left == 0 && b.kind.eq_ignore_ascii_case("Silo")).count() as i32 * SILO_CAPACITY
    }

    /// Cutting grass with the scythe may store hay.
    pub(crate) fn cut_grass(&mut self, t: TilePos) -> bool {
        self.cut_grass_with(t, true)
    }

    /// Cut grass; `hay` gives the scythe's chance of hay for the silos.
    /// A haymaker weapon drops hay half the time either way.
    pub(crate) fn cut_grass_with(&mut self, t: TilePos, hay: bool) -> bool {
        let Some(TerrainFeature::Grass(_)) = self.location().terrain(t) else { return false };
        self.location_mut().state.terrain.remove(&t);
        self.sound("cut");
        let cap = self.silo_capacity();
        if hay && cap > self.farmer.hay_storage && self.rng.chance(0.5) {
            self.farmer.hay_storage += 1;
            self.hud(format!("Hay +1 ({}/{})", self.farmer.hay_storage, cap));
        }
        if self.held_weapon_has("Haymaker") && self.rng.chance(0.5) {
            if let Some(h) = self.hay_id.clone() {
                self.spawn_drop(Item::object(&h, 1), t.world_center());
            }
        }
        true
    }

    /// Take one hay from storage (hay hopper action).
    pub(crate) fn hay_hopper(&mut self) -> bool {
        let Some(hay) = self.hay_id.clone() else { return false };
        if self.farmer.hay_storage <= 0 {
            self.emit(WorldEvent::Message("The silos are empty.".into()));
            return true;
        }
        let item = Item::object(&hay, 1);
        if !self.farmer.inventory.can_fit(&item) {
            self.hud("Inventory full");
            return true;
        }
        self.farmer.hay_storage -= 1;
        self.farmer.inventory.add(item.clone());
        self.sound("shwip");
        true
    }

    /// Initial grass patches on a new farm.
    pub(crate) fn seed_grass(&mut self, rng: &mut Rng) {
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        let (w, h) = (farm.width(), farm.height());
        for _ in 0..(w * h / 120) {
            let c = TilePos::new(rng.range(0, w), rng.range(0, h));
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let p = TilePos::new(c.x + dx, c.y + dy);
                    if farm.is_clear(p) && farm.is_diggable(p) && farm.ground_type(p).as_deref() == Some("Grass") && rng.chance(0.6) {
                        farm.state.terrain.insert(p, TerrainFeature::Grass(Grass { weeds: rng.range(1, 5) }));
                    }
                }
            }
        }
    }

    /// Overnight grass spread on the farm (not in winter; approx. odds).
    pub(crate) fn grass_overnight(&mut self) {
        if self.clock.season == crate::time::Season::Winter {
            return;
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 1777]));
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        let tufts: Vec<TilePos> = farm.state.terrain.iter().filter(|(_, t)| matches!(t, TerrainFeature::Grass(_))).map(|(p, _)| *p).collect();
        for p in tufts {
            if !rng.chance(0.1) {
                continue;
            }
            let n = p.offset(crate::Dir::from_index(rng.range(0, 4)));
            if farm.is_clear(n) && farm.is_diggable(n) && farm.ground_type(n).as_deref() == Some("Grass") {
                farm.state.terrain.insert(n, TerrainFeature::Grass(Grass { weeds: 1 }));
            }
        }
        // Grazing animals trim some grass.
        let grazing = self.animals.iter().filter(|a| a.went_outside).count();
        let mut eaten = 0;
        let keys: Vec<TilePos> = farm.state.terrain.iter().filter(|(_, t)| matches!(t, TerrainFeature::Grass(_))).map(|(p, _)| *p).collect();
        for p in keys {
            if eaten >= grazing {
                break;
            }
            farm.state.terrain.remove(&p);
            eaten += 1;
        }
    }

    /// During storms, lightning may strike the farm (approx. odds).
    pub(crate) fn maybe_lightning(&mut self) {
        if self.weather != crate::time::Weather::Storm {
            return;
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, self.clock.time as i64, 4242]));
        if !rng.chance(0.125) {
            return;
        }
        let battery = BATTERY.iter().find(|id| self.items.objects.contains_key(**id)).map(|s| s.to_string());
        let Some(farm) = self.locations.get_mut("Farm") else { return };
        let rod = farm.state.objects.iter().find(|(_, o)| RODS.contains(&o.item_id.as_str()) && matches!(o.kind, ObjKind::Machine { output: None, .. } | ObjKind::Placed)).map(|(p, _)| *p);
        if let Some(p) = rod {
            if let Some(o) = farm.state.objects.get_mut(&p) {
                o.kind = ObjKind::Machine { output: battery.map(|b| Item::object(&b, 1)), minutes_left: 1440, ready: false };
                o.shake = 20;
            }
        } else {
            let (w, h) = (farm.width(), farm.height());
            let t = TilePos::new(rng.range(0, w), rng.range(0, h));
            match farm.state.terrain.get_mut(&t) {
                Some(TerrainFeature::HoeDirt(d)) => {
                    if let Some(c) = d.crop.as_mut() {
                        c.dead = true;
                    }
                }
                Some(TerrainFeature::Tree(tr)) if tr.mature() => {
                    tr.stump = true;
                    tr.health = crate::terrain::STUMP_HEALTH;
                }
                _ => {}
            }
        }
        if self.current == "Farm" || self.location().outdoors {
            self.sound("thunder");
            self.emit(WorldEvent::Lightning);
        }
    }

    /// Public wrapper for the hay hopper (tests, console).
    pub fn hay_hopper_public(&mut self) -> bool {
        self.hay_hopper()
    }

    pub fn is_lightning_rod(&self, o: &PlacedObject) -> bool {
        RODS.contains(&o.item_id.as_str())
    }
}
