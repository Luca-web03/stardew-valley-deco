//! Wallpaper and flooring (public behaviour). Items `(WP)<n>` and
//! `(FL)<n>` index the base sheet `Maps/walls_and_floors` (112 wallpapers,
//! 56 floors) or a set from `Data/AdditionalWallpaperFlooring`
//! (`<SetId>:<n>`). Using one in a decoratable location (farmhouse,
//! cabins, sheds, or a map with the `Port.Decoratable` property) paints the
//! room under the cursor: wallpaper covers the wall tiles above the room's
//! floor, flooring its floor tiles. A room is a connected area of floor
//! tiles (a `Back` tile with nothing on `Buildings`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::Deserialize;

use crate::item::{Item, ItemDb};
use crate::location::GameLocation;
use crate::world::World;
use crate::TilePos;

pub const BASE_WALLPAPERS: i32 = 112;
pub const BASE_FLOORS: i32 = 56;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct DecorSet {
    pub id: String,
    pub texture: String,
    pub is_flooring: bool,
    pub count: i32,
}

/// Does this wallpaper/flooring id exist?
pub fn decor_exists(db: &ItemDb, id: &str, floor: bool) -> bool {
    match id.split_once(':') {
        Some((set, n)) => {
            let n: i32 = n.parse().unwrap_or(-1);
            db.decor_sets.iter().any(|s| s.id == set && s.is_flooring == floor && n >= 0 && n < s.count.max(1))
        }
        None => {
            let n: i32 = id.parse().unwrap_or(-1);
            db.base_decor && n >= 0 && n < if floor { BASE_FLOORS } else { BASE_WALLPAPERS }
        }
    }
}

/// (texture, index) of a wallpaper/flooring id.
pub fn decor_source(db: &ItemDb, id: &str) -> (String, i32) {
    match id.split_once(':') {
        Some((set, n)) => {
            let tex = db.decor_sets.iter().find(|s| s.id == set).map(|s| s.texture.replace('\\', "/")).unwrap_or_default();
            (tex, n.parse().unwrap_or(0))
        }
        None => ("Maps/walls_and_floors".into(), id.parse().unwrap_or(0)),
    }
}

/// A room: its floor tiles and the wall tiles above them (bottom row
/// first in each column).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Room {
    /// Key used for saved paint: the room's first floor tile ("x y").
    pub key: String,
    pub floor: BTreeSet<TilePos>,
    /// (wall tile, row above the floor: 0 = lowest).
    pub walls: Vec<(TilePos, i32)>,
}

fn is_floor(loc: &GameLocation, p: TilePos) -> bool {
    loc.in_bounds(p) && loc.has_tile("Back", p) && !loc.has_tile("Buildings", p) && !loc.is_water(p)
}

/// The rooms of a location.
pub fn rooms(loc: &GameLocation) -> Vec<Room> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for y in 0..loc.height() {
        for x in 0..loc.width() {
            let start = TilePos::new(x, y);
            if seen.contains(&start) || !is_floor(loc, start) {
                continue;
            }
            let mut floor = BTreeSet::new();
            let mut q = VecDeque::from([start]);
            seen.insert(start);
            while let Some(p) = q.pop_front() {
                floor.insert(p);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = TilePos::new(p.x + dx, p.y + dy);
                    if !seen.contains(&n) && is_floor(loc, n) {
                        seen.insert(n);
                        q.push_back(n);
                    }
                }
            }
            let mut walls = Vec::new();
            for p in &floor {
                if floor.contains(&TilePos::new(p.x, p.y - 1)) {
                    continue;
                }
                let mut column = Vec::new();
                let mut top = p.y - 1;
                for row in 0..3 {
                    let w = TilePos::new(p.x, p.y - 1 - row);
                    if !loc.in_bounds(w) || !loc.has_tile("Buildings", w) {
                        break;
                    }
                    column.push((w, row));
                    top = w.y - 1;
                }
                // Something standing in the room (floor beyond it) isn't a wall.
                if floor.contains(&TilePos::new(p.x, top)) {
                    continue;
                }
                // Fixtures with actions (TV, doors) keep their look.
                walls.extend(column.into_iter().filter(|(w, _)| loc.tile_prop("Buildings", *w, "Action").is_none()));
            }
            out.push(Room { key: format!("{} {}", start.x, start.y), floor, walls });
        }
    }
    out
}

pub fn is_decoratable(loc: &GameLocation, name: &str) -> bool {
    !loc.outdoors && (name.starts_with("FarmHouse") || name.starts_with("Cabin") || name.contains("Shed") || loc.map.prop("Port.Decoratable").is_some())
}

/// Saved paint per location: room key -> wallpaper / flooring id.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, Deserialize)]
pub struct RoomPaint {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub wallpaper: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub flooring: BTreeMap<String, String>,
}

impl World {
    /// Use held wallpaper/flooring on tile `t`.
    pub(crate) fn apply_decor(&mut self, t: TilePos) -> bool {
        let Some(Item::Decor { id, floor }) = self.farmer.current_item().cloned() else { return false };
        let here = self.current.clone();
        if !is_decoratable(self.location(), &here) {
            self.hud("You can only decorate your own home.");
            return true;
        }
        let rooms = rooms(self.location());
        let room = rooms.iter().find(|r| r.floor.contains(&t) || r.walls.iter().any(|(w, _)| *w == t));
        let Some(room) = room else { return false };
        let key = room.key.clone();
        let paint = &mut self.location_mut().state.paint;
        let slot = if floor { &mut paint.flooring } else { &mut paint.wallpaper };
        if slot.get(&key) == Some(&id) {
            return true;
        }
        slot.insert(key, id);
        self.farmer.inventory.reduce(self.farmer.current_slot, 1);
        self.sound(if floor { "coin" } else { "paintBrush" });
        true
    }
}
