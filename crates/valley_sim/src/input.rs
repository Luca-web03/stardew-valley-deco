//! Device-independent player intent for one tick. The front end maps keys,
//! mouse and gamepads to this, so the simulation never sees raw devices.

use crate::TilePos;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerInput {
    /// -1, 0 or 1 per axis.
    pub move_x: i32,
    pub move_y: i32,
    /// Use the held tool / item (left click, C).
    pub use_tool: bool,
    /// Check / interact / eat / plant / harvest (right click, X).
    pub action: bool,
    /// Tile the mouse cursor points at, if any (for click targeting).
    pub cursor_tile: Option<TilePos>,
    /// Directly select a toolbar slot.
    pub select_slot: Option<usize>,
    /// Scroll toolbar selection by this many slots.
    pub scroll: i32,
    /// Hold to walk instead of run.
    pub walk: bool,
}
