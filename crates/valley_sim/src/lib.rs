//! Headless game simulation.
//!
//! Everything here is deterministic given a seed and a sequence of
//! [`input::PlayerInput`]s, and nothing touches graphics, so whole days can be
//! simulated in tests. Mechanics are reimplemented from publicly documented
//! behaviour; constants that are best-effort approximations are marked as such.

pub mod buffs;
pub mod collision;
pub mod combat;
pub mod crafting;
pub mod crop;
pub mod machines;
pub mod mines;
pub mod dialogue;
pub mod events;
pub mod farm;
pub mod farmer;
pub mod fishing;
pub mod gsq;
pub mod input;
pub mod inventory;
pub mod item;
pub mod location;
pub mod mail;
pub mod npc;
pub mod objects;
pub mod pathfind;
pub mod progress;
pub mod quests;
pub mod save;
pub mod schedule;
pub mod shop;
pub mod terrain;
pub mod time;
pub mod tools;
pub mod world;

pub use world::World;

/// World pixels per map tile (16px art drawn at 4x).
pub const TILE: i32 = 64;
pub const TILE_F: f32 = TILE as f32;
/// Pixel art scale.
pub const ZOOM: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum Dir {
    Up,
    Right,
    #[default]
    Down,
    Left,
}

impl Dir {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Right => (1, 0),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
        }
    }

    /// Numeric facing used by map/schedule data: 0 up, 1 right, 2 down, 3 left.
    pub fn from_index(i: i32) -> Dir {
        match i.rem_euclid(4) {
            0 => Dir::Up,
            1 => Dir::Right,
            2 => Dir::Down,
            _ => Dir::Left,
        }
    }

    pub fn index(self) -> i32 {
        match self {
            Dir::Up => 0,
            Dir::Right => 1,
            Dir::Down => 2,
            Dir::Left => 3,
        }
    }

    pub fn parse(s: &str) -> Option<Dir> {
        match s.trim().to_ascii_lowercase().as_str() {
            "up" | "0" => Some(Dir::Up),
            "right" | "1" => Some(Dir::Right),
            "down" | "2" => Some(Dir::Down),
            "left" | "3" => Some(Dir::Left),
            _ => None,
        }
    }

    pub fn toward(dx: i32, dy: i32) -> Dir {
        if dx.abs() > dy.abs() {
            if dx > 0 { Dir::Right } else { Dir::Left }
        } else if dy < 0 {
            Dir::Up
        } else {
            Dir::Down
        }
    }
}

/// Integer tile coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default, serde::Serialize, serde::Deserialize)]
pub struct TilePos {
    pub x: i32,
    pub y: i32,
}

impl TilePos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn offset(self, d: Dir) -> Self {
        let (dx, dy) = d.delta();
        Self::new(self.x + dx, self.y + dy)
    }
    pub fn world_center(self) -> engine::Vec2 {
        engine::Vec2::new(self.x as f32 * TILE_F + TILE_F / 2.0, self.y as f32 * TILE_F + TILE_F / 2.0)
    }
    pub fn from_world(p: engine::Vec2) -> Self {
        Self::new((p.x / TILE_F).floor() as i32, (p.y / TILE_F).floor() as i32)
    }
    pub fn manhattan(self, o: TilePos) -> i32 {
        (self.x - o.x).abs() + (self.y - o.y).abs()
    }
}
