//! Rendering of the simulation into [`engine::DrawList`]s.
//!
//! Depth bands (higher draws on top):
//! * 0.00..0.10 map Back layer and ground decals (tilled soil)
//! * 0.10..0.20 map Buildings layer
//! * 0.20..0.70 y-sorted world objects (characters, crops, trees, debris)
//! * 0.80 map Front layer, 0.90 AlwaysFront
//! * 0.95 weather / lighting overlays
//! * >= 1.0 HUD and menus

pub mod assets;
pub mod camera;
pub mod farmer_compose;
pub mod layouts;
pub mod ui;
pub mod world_draw;

pub use assets::Assets;
pub use camera::Camera;
pub use world_draw::draw_world;

pub const DEPTH_BACK: f32 = 0.0;
pub const DEPTH_GROUND: f32 = 0.05;
pub const DEPTH_BUILDINGS: f32 = 0.1;
pub const DEPTH_SORT_MIN: f32 = 0.2;
pub const DEPTH_SORT_MAX: f32 = 0.7;
pub const DEPTH_FRONT: f32 = 0.8;
pub const DEPTH_ALWAYS_FRONT: f32 = 0.9;
pub const DEPTH_OVERLAY: f32 = 0.95;
pub const DEPTH_HUD: f32 = 1.0;
