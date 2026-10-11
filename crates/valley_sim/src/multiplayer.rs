//! Farmhands: extra players sharing the world (co-op). The host's farmer
//! lives in `World::farmer`; each farmhand keeps its own farmer and the
//! per-player parts of the world (location, swing, fishing, prompts...).
//! A farmhand's input runs through the same code as the host's by swapping
//! its state in, updating, and swapping it back.

use serde::{Deserialize, Serialize};

use crate::farmer::Farmer;
use crate::input::PlayerInput;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos};

/// Per-player state that lives on `World` for the active player.
#[derive(Clone, Debug)]
pub struct PlayerState {
    pub farmer: Farmer,
    pub current: String,
    pub in_bed: bool,
    pub fishing: crate::fishing::FishingState,
    pub weapon_swing: Option<crate::combat::WeaponSwing>,
    pub special: Option<crate::arsenal::SpecialMove>,
    pub special_cooldown_ms: u32,
    pub sling: Option<crate::arsenal::SlingPull>,
    pub pending_question: Option<(String, crate::dialogue::Question)>,
    /// Events produced for this player, waiting to be sent to them.
    pub events: Vec<WorldEvent>,
}

/// What gets saved per farmhand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FarmhandSave {
    pub farmer: Farmer,
    pub current: String,
}

impl PlayerState {
    pub fn new(farmer: Farmer, current: &str) -> Self {
        Self {
            farmer,
            current: current.to_string(),
            in_bed: false,
            fishing: Default::default(),
            weapon_swing: None,
            special: None,
            special_cooldown_ms: 0,
            sling: None,
            pending_question: None,
            events: Vec::new(),
        }
    }
}

impl World {
    /// Exchange the active player's state with `p`.
    fn swap_player(&mut self, p: &mut PlayerState) {
        std::mem::swap(&mut self.farmer, &mut p.farmer);
        std::mem::swap(&mut self.current, &mut p.current);
        std::mem::swap(&mut self.in_bed, &mut p.in_bed);
        std::mem::swap(&mut self.fishing, &mut p.fishing);
        std::mem::swap(&mut self.weapon_swing, &mut p.weapon_swing);
        std::mem::swap(&mut self.special, &mut p.special);
        std::mem::swap(&mut self.special_cooldown_ms, &mut p.special_cooldown_ms);
        std::mem::swap(&mut self.sling, &mut p.sling);
        std::mem::swap(&mut self.pending_question, &mut p.pending_question);
        std::mem::swap(&mut self.events, &mut p.events);
    }

    /// Run `f` as farmhand `i` (their farmer, location and prompts are
    /// active); events produced go to that farmhand.
    pub fn as_farmhand<R>(&mut self, i: usize, f: impl FnOnce(&mut World) -> R) -> Option<R> {
        let mut p = self.farmhands.get_mut(i).map(|p| std::mem::replace(p, PlayerState::new(Farmer::new("", ""), "")))?;
        self.swap_player(&mut p);
        self.active_farmhand = Some(i);
        let r = f(self);
        self.active_farmhand = None;
        self.swap_player(&mut p);
        self.farmhands[i] = p;
        Some(r)
    }

    /// Add a farmhand who wakes up in the farmhouse; returns their index.
    pub fn add_farmhand(&mut self, name: &str) -> usize {
        let mut farmer = Farmer::new(name, &self.farmer.farm_name);
        let wake = self.wake_tile();
        farmer.set_tile(TilePos::new(wake.x - 1, wake.y));
        farmer.facing = Dir::Down;
        self.farmhands.push(PlayerState::new(farmer, "FarmHouse"));
        self.farmhands.len() - 1
    }

    /// One tick of a farmhand's own actions (the shared world is advanced
    /// by the host's `update`).
    pub fn update_farmhand(&mut self, i: usize, input: &PlayerInput) {
        let dt = crate::world::TICK_MS;
        self.as_farmhand(i, |w| w.update_player(input, dt));
    }

    /// Events waiting for farmhand `i`.
    pub fn drain_farmhand_events(&mut self, i: usize) -> Vec<WorldEvent> {
        self.farmhands.get_mut(i).map(|p| std::mem::take(&mut p.events)).unwrap_or_default()
    }

    /// Night: farmhands rest too and wake up indoors.
    pub(crate) fn farmhands_overnight(&mut self, passed_out: bool) {
        let wake = self.wake_tile();
        for i in 0..self.farmhands.len() {
            self.as_farmhand(i, |w| {
                w.farmer_night_reset(passed_out);
                w.current = "FarmHouse".into();
                w.farmer.set_tile(TilePos::new(wake.x - 1 - i as i32, wake.y + 1));
                w.farmer.facing = Dir::Down;
                w.weapon_swing = None;
                w.special = None;
                w.sling = None;
                w.fishing = Default::default();
            });
        }
    }

    /// Every player's (name, location, position, facing, moving, walk ms)
    /// for drawing other players.
    pub fn players(&self) -> Vec<(&Farmer, &str)> {
        let mut v = vec![(&self.farmer, self.current.as_str())];
        v.extend(self.farmhands.iter().map(|p| (&p.farmer, p.current.as_str())));
        v
    }

    pub fn farmhand_saves(&self) -> Vec<FarmhandSave> {
        self.farmhands.iter().map(|p| FarmhandSave { farmer: p.farmer.clone(), current: p.current.clone() }).collect()
    }

    pub fn restore_farmhands(&mut self, saves: Vec<FarmhandSave>) {
        self.farmhands = saves.into_iter().map(|s| PlayerState::new(s.farmer, &s.current)).collect();
    }
}
