//! Stables and horses. A finished stable (building id `Stable` or the
//! `Port.Stable` custom field) gets a horse. Riding is faster; the horse
//! waits outside when the farmer goes indoors and walks home overnight.
//! The speed boost is an approximation.

use engine::Vec2;
use serde::{Deserialize, Serialize};

use crate::farm::FarmAnimal;
use crate::world::World;
use crate::{Dir, TilePos, TILE_F};

/// Riding multiplies running speed by this much (approx.).
pub const HORSE_SPEED: f32 = 1.6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Horse {
    pub name: String,
    pub location: String,
    pub pos: Vec2,
    pub facing: Dir,
    #[serde(skip)]
    pub moving: bool,
    #[serde(skip)]
    pub walk_ms: u32,
}

impl World {
    pub fn is_stable(&self, kind: &str) -> bool {
        kind == "Stable" || self.buildings_data.get(kind).is_some_and(|d| d.custom_fields.as_ref().is_some_and(|f| f.contains_key("Port.Stable")))
    }

    /// Where a stable's horse waits: just below the building, middle column.
    fn stable_spot(&self, bi: usize) -> TilePos {
        let b = &self.buildings[bi];
        TilePos::new(b.tile.x + b.size.0 / 2, b.tile.y + b.size.1)
    }

    /// Give finished stables a horse; send wandering horses home.
    pub(crate) fn horses_overnight(&mut self) {
        self.farmer.mounted = None;
        for bi in 0..self.buildings.len() {
            if !self.is_stable(&self.buildings[bi].kind) || self.buildings[bi].days_left > 0 {
                continue;
            }
            let spot = self.stable_spot(bi);
            let pos = Vec2::new(spot.x as f32 * TILE_F, spot.y as f32 * TILE_F - 16.0);
            let horse = self.buildings[bi].horse.get_or_insert_with(|| Horse {
                name: "Horse".into(),
                location: "Farm".into(),
                pos,
                facing: Dir::Down,
                moving: false,
                walk_ms: 0,
            });
            horse.location = "Farm".into();
            horse.pos = pos;
            horse.facing = Dir::Down;
        }
    }

    /// Index of the building whose horse stands on/next to `t` here.
    fn horse_at(&self, t: TilePos) -> Option<usize> {
        let me = self.farmer.tile();
        self.buildings.iter().position(|b| {
            b.horse.as_ref().is_some_and(|h| {
                let ht = TilePos::from_world(FarmAnimal::bbox_at(h.pos).center());
                h.location == self.current && (ht == t || (ht.manhattan(me) <= 1 && ht.manhattan(t) <= 1))
            })
        })
    }

    /// Action button: mount a nearby horse.
    pub(crate) fn try_mount(&mut self, t: TilePos) -> bool {
        if self.farmer.mounted.is_some() {
            return false;
        }
        let Some(bi) = self.horse_at(t) else { return false };
        let id = self.buildings[bi].id;
        if let Some(h) = self.buildings[bi].horse.as_ref() {
            self.farmer.pos = h.pos;
            self.farmer.facing = h.facing;
        }
        self.farmer.mounted = Some(id);
        self.sound("dwop");
        true
    }

    /// Get off where the farmer is.
    pub fn dismount(&mut self) {
        let Some(id) = self.farmer.mounted.take() else { return };
        let (here, pos, facing) = (self.current.clone(), self.farmer.pos, self.farmer.facing);
        if let Some(h) = self.buildings.iter_mut().find(|b| b.id == id).and_then(|b| b.horse.as_mut()) {
            h.location = here;
            h.pos = pos;
            h.facing = facing;
        }
        self.sound("dwop");
    }

    /// The ridden horse follows the farmer; indoors it is left at the door.
    pub(crate) fn horse_follow(&mut self, from: &str, from_pos: Vec2) {
        let Some(id) = self.farmer.mounted else { return };
        let indoors = self.locations.get(&self.current).is_some_and(|l| !l.outdoors);
        if !indoors {
            self.update_horse();
            return;
        }
        {
            self.farmer.mounted = None;
            if let Some(h) = self.buildings.iter_mut().find(|b| b.id == id).and_then(|b| b.horse.as_mut()) {
                h.location = from.to_string();
                h.pos = from_pos;
            }
        }
    }

    /// Per tick: keep the ridden horse under the farmer.
    pub(crate) fn update_horse(&mut self) {
        let Some(id) = self.farmer.mounted else { return };
        let (here, pos, facing, moving, walk) = (self.current.clone(), self.farmer.pos, self.farmer.facing, self.farmer.moving, self.farmer.walk_ms);
        if let Some(h) = self.buildings.iter_mut().find(|b| b.id == id).and_then(|b| b.horse.as_mut()) {
            h.location = here;
            h.pos = pos;
            h.facing = facing;
            h.moving = moving;
            h.walk_ms = walk;
        }
    }

    /// Horses visible in a location (for drawing): (horse, ridden?).
    pub fn horses_in(&self, location: &str) -> Vec<(&Horse, bool)> {
        self.buildings
            .iter()
            .filter_map(|b| b.horse.as_ref().map(|h| (h, self.farmer.mounted == Some(b.id))))
            .filter(|(h, _)| h.location == location)
            .collect()
    }
}
