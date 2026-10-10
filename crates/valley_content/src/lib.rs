//! Content pipeline.
//!
//! The game's own assets are never bundled with this project. At runtime the
//! [`ContentManager`] reads them from the player's own copy of the game,
//! unpacked to plain files (PNG / JSON / TMX / TBIN), e.g. with StardewXNBHack.
//! Raw `.xnb` textures and maps are also understood where possible.
//!
//! When no game folder is configured, an original placeholder pack
//! ([`dev`]) is served through exactly the same code paths, so the engine is
//! fully runnable and testable on its own.

pub mod data;
pub mod dev;
pub mod manager;
pub mod map;
pub mod props;
pub mod source;
pub mod strings;
pub mod tbin;
pub mod tmx;
pub mod xnb;

pub use manager::ContentManager;
pub use map::{AnimatedTile, Layer, Map, Tile, TileSheet};
pub use props::{PropValue, Props};
pub use source::{ContentSource, DirSource, LayeredSource, MemSource};
