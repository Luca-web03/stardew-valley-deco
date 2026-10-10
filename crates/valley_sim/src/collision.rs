//! Tile and box collision for characters.

use engine::IRect;

use crate::location::{truthy, GameLocation};
use crate::{TilePos, TILE};

/// Whether a character may stand on this tile (map + placed things).
pub fn tile_passable(loc: &GameLocation, p: TilePos) -> bool {
    if !loc.in_bounds(p) {
        return false;
    }
    if !loc.has_tile("Back", p) {
        return false;
    }
    if let Some(v) = loc.tile_prop("Back", p, "Passable") {
        if !truthy(&v) {
            return false;
        }
    }
    if loc.is_water(p) {
        return false;
    }
    if loc.has_tile("Buildings", p) {
        let explicit = loc.tile_prop("Buildings", p, "Passable").is_some_and(|v| truthy(&v));
        let shadow = loc.tile_prop("Buildings", p, "Shadow").is_some();
        if !explicit && !shadow {
            return false;
        }
    }
    if let Some(o) = loc.object(p) {
        if !o.passable() {
            return false;
        }
    }
    if let Some(t) = loc.terrain(p) {
        if !t.passable() {
            return false;
        }
    }
    true
}

/// Does a world-space box overlap any blocked tile?
pub fn box_blocked(loc: &GameLocation, b: IRect) -> bool {
    let x0 = b.x.div_euclid(TILE);
    let y0 = b.y.div_euclid(TILE);
    let x1 = (b.right() - 1).div_euclid(TILE);
    let y1 = (b.bottom() - 1).div_euclid(TILE);
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !tile_passable(loc, TilePos::new(x, y)) {
                return true;
            }
        }
    }
    false
}
