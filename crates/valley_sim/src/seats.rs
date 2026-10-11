//! Sitting: map seats from `Data/ChairTiles` (public format: key
//! `<tilesheet image>/<x>/<y>` naming a tile of a sheet by its column and
//! row, value `<width>/<height>/<direction>/<type>/<draw x>/<draw y>/
//! <seasonal>/<alternate sheet>`) on the `Buildings` layer, and seat
//! furniture (chairs, benches, couches, armchairs) facing their rotation.
//! Moving or using a tool stands the farmer back up where they were.

use std::collections::HashMap;

use engine::Vec2;

use crate::world::World;
use crate::{Dir, TilePos};

#[derive(Clone, Debug, PartialEq)]
pub struct Sitting {
    pub location: String,
    /// Farmer position while seated.
    pub pos: Vec2,
    pub facing: Dir,
    /// Where they stand back up.
    pub from: Vec2,
    /// The seat tile.
    pub tile: TilePos,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChairTile {
    pub width: i32,
    pub height: i32,
    pub direction: String,
    pub kind: String,
}

pub fn parse_chair_tile(v: &str) -> Option<ChairTile> {
    let f: Vec<&str> = v.split('/').collect();
    Some(ChairTile {
        width: f.first()?.trim().parse().unwrap_or(1),
        height: f.get(1).and_then(|s| s.trim().parse().ok()).unwrap_or(1),
        direction: f.get(2).unwrap_or(&"down").trim().to_ascii_lowercase(),
        kind: f.get(3).unwrap_or(&"default").trim().to_string(),
    })
}

/// `Data/ChairTiles`, keyed by lower-case "<sheet image>/<x>/<y>".
pub fn load(cm: &mut valley_content::ContentManager) -> HashMap<String, ChairTile> {
    cm.load_string_dict("Data/ChairTiles").into_iter().filter_map(|(k, v)| Some((k.to_ascii_lowercase(), parse_chair_tile(&v)?))).collect()
}

fn dir_from(s: &str, approach: Dir) -> Dir {
    match s {
        "up" => Dir::Up,
        "left" => Dir::Left,
        "right" => Dir::Right,
        "opposite" => approach.opposite(),
        _ => Dir::Down,
    }
}

/// Facing for a furniture rotation.
pub fn rotation_dir(rotation: i32) -> Dir {
    match rotation.rem_euclid(4) {
        1 => Dir::Right,
        2 => Dir::Up,
        3 => Dir::Left,
        _ => Dir::Down,
    }
}

/// Image name without folders, extension or season prefix.
fn sheet_key(image: &str) -> (String, String) {
    let base = image.rsplit(['/', '\\']).next().unwrap_or(image);
    let base = base.split('.').next().unwrap_or(base).to_ascii_lowercase();
    let unseasoned = ["spring_", "summer_", "fall_", "winter_"].iter().find_map(|p| base.strip_prefix(p)).map(str::to_string).unwrap_or_else(|| base.clone());
    (base, unseasoned)
}

impl World {
    /// A map seat covering tile `t`: (seat origin tile, facing).
    pub fn map_seat_at(&self, t: TilePos) -> Option<(TilePos, Dir)> {
        if self.chair_tiles.is_empty() {
            return None;
        }
        let map = &self.location().map;
        // The seat may start up to a few tiles left/up of `t`.
        for oy in 0..3 {
            for ox in 0..4 {
                let o = TilePos::new(t.x - ox, t.y - oy);
                let Some(tile) = map.tile("Buildings", o.x, o.y) else { continue };
                let Some(sheet) = map.sheets.get(tile.sheet as usize) else { continue };
                let cols = sheet.sheet_w.max(1);
                let (sx, sy) = (tile.index % cols, tile.index / cols);
                let (full, plain) = sheet_key(&sheet.image);
                let seat = [full, plain, sheet.id.to_ascii_lowercase()].iter().find_map(|k| self.chair_tiles.get(&format!("{k}/{sx}/{sy}")));
                let Some(seat) = seat else { continue };
                if ox < seat.width.max(1) && oy < seat.height.max(1) {
                    return Some((t, dir_from(&seat.direction, self.farmer.facing)));
                }
            }
        }
        None
    }

    /// Sit on a seat at `t` (furniture or map). True if seated.
    pub(crate) fn try_sit(&mut self, t: TilePos) -> bool {
        if self.farmer.sitting.is_some() || self.farmer.mounted.is_some() || self.farmer.swimming {
            return false;
        }
        if matches!(self.farmer.current_item(), Some(crate::item::Item::Furniture { .. })) {
            return false;
        }
        let furniture = self.location().state.furniture.iter().find(|f| f.covers(t)).cloned();
        let seat = match furniture {
            Some(f) => {
                let Some(info) = self.items.furniture.get(&f.id) else { return false };
                if !info.is_seat() {
                    return false;
                }
                // Anyone already sitting there?
                Some((t, rotation_dir(f.rotation)))
            }
            None => self.map_seat_at(t),
        };
        let Some((tile, facing)) = seat else { return false };
        let pos = Vec2::new(tile.x as f32 * crate::TILE_F, tile.y as f32 * crate::TILE_F - 16.0);
        let from = self.farmer.pos;
        self.farmer.sitting = Some(Sitting { location: self.current.clone(), pos, facing, from, tile });
        self.farmer.pos = pos;
        self.farmer.facing = facing;
        self.farmer.moving = false;
        self.sound("woodyStep");
        true
    }

    /// Get up from a seat.
    pub fn stand_up(&mut self) {
        if let Some(s) = self.farmer.sitting.take() {
            if s.location == self.current {
                self.farmer.pos = s.from;
            }
        }
    }

    pub fn is_sitting(&self) -> bool {
        self.farmer.sitting.is_some()
    }
}
