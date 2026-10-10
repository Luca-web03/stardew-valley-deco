//! Game-agnostic engine core.
//!
//! Everything a game needs to talk to a platform backend lives here:
//! textures, a depth-sorted [`DrawList`], [`InputState`], a deterministic
//! [`Rng`], and the [`GameModule`] trait. Games never call a graphics API
//! directly; they emit draw commands, which lets several games (modules)
//! share one window and one backend, and lets every frame be rendered
//! headlessly by the [`soft`] renderer for tests and screenshots.

pub mod audio;
pub mod draw;
pub mod font;
pub mod input;
pub mod math;
pub mod module;
pub mod rng;
pub mod soft;
pub mod texture;

pub use audio::{AudioQueue, SoundCmd};
pub use draw::{DrawCmd, DrawList, Sprite};
pub use input::{InputState, Key, MouseButton};
pub use math::{Color, FRect, IRect, Vec2};
pub use module::{EngineCtx, GameModule, ModuleHost, TICKS_PER_SECOND, TICK_SECONDS};
pub use rng::Rng;
pub use texture::{Image, TexId, TextureStore};
