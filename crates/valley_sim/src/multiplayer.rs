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

    /// Make farmhand `i` the active player (until [`World::end_farmhand`]).
    /// While active, `update` only runs that player's own actions.
    pub fn begin_farmhand(&mut self, i: usize) -> bool {
        if self.active_farmhand.is_some() || i >= self.farmhands.len() {
            return false;
        }
        let mut p = std::mem::replace(&mut self.farmhands[i], PlayerState::new(Farmer::new("", ""), ""));
        self.swap_player(&mut p);
        // The slot now holds the host's state until we swap back.
        self.farmhands[i] = p;
        self.active_farmhand = Some(i);
        true
    }

    pub fn end_farmhand(&mut self) {
        let Some(i) = self.active_farmhand.take() else { return };
        let mut p = std::mem::replace(&mut self.farmhands[i], PlayerState::new(Farmer::new("", ""), ""));
        self.swap_player(&mut p);
        self.farmhands[i] = p;
    }

    /// Run `f` as farmhand `i` (their farmer, location and prompts are
    /// active); events produced go to that farmhand.
    pub fn as_farmhand<R>(&mut self, i: usize, f: impl FnOnce(&mut World) -> R) -> Option<R> {
        if !self.begin_farmhand(i) {
            return None;
        }
        let r = f(self);
        self.end_farmhand();
        Some(r)
    }

    /// Add a farmhand; they move into a free cabin if there is one, else
    /// they share the farmhouse. Returns their index.
    pub fn add_farmhand(&mut self, name: &str) -> usize {
        let mut farmer = Farmer::new(name, &self.farmer.farm_name);
        farmer.home_location = self.claim_cabin(name);
        let i = self.farmhands.len();
        let (home, tile) = self.home_wake(&farmer, i);
        farmer.set_tile(tile);
        farmer.facing = Dir::Down;
        self.farmhands.push(PlayerState::new(farmer, &home));
        i
    }

    /// The farmhand with this name, if one has played here before.
    pub fn farmhand_index(&self, name: &str) -> Option<usize> {
        self.farmhands.iter().position(|p| p.farmer.name == name)
    }

    pub fn is_cabin_kind(&self, kind: &str) -> bool {
        kind == "Cabin" || self.buildings_data.get(kind).and_then(|d| d.custom_fields.as_ref()).is_some_and(|f| f.contains_key("Port.Cabin"))
    }

    /// Give `name` a finished cabin nobody else owns; its interior name.
    pub fn claim_cabin(&mut self, name: &str) -> Option<String> {
        let i = self.buildings.iter().position(|b| self.is_cabin_kind(&b.kind) && b.interior.is_some() && b.owner.as_deref() == Some(name)).or_else(|| {
            self.buildings.iter().position(|b| self.is_cabin_kind(&b.kind) && b.interior.is_some() && b.owner.is_none())
        })?;
        self.buildings[i].owner = Some(name.to_string());
        self.buildings[i].interior.clone()
    }

    /// Where a player wakes: beside the bed of their home, or the farmhouse.
    fn home_wake(&self, f: &Farmer, i: usize) -> (String, TilePos) {
        if let Some(home) = f.home_location.as_ref().filter(|h| self.locations.contains_key(*h)) {
            let loc = &self.locations[home];
            let bed = (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).find(|p| loc.tile_prop("Back", *p, "Bed").is_some());
            if let Some(bed) = bed {
                for d in [Dir::Left, Dir::Down, Dir::Right, Dir::Up] {
                    let p = bed.offset(d);
                    if crate::collision::tile_passable(loc, p) && loc.tile_prop("Back", p, "Bed").is_none() {
                        return (home.clone(), p);
                    }
                }
            }
            if let Some(p) = self.arrival_tile(home) {
                return (home.clone(), p);
            }
        }
        let wake = self.wake_tile();
        ("FarmHouse".into(), TilePos::new(wake.x - 1 - i as i32, wake.y + 1))
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
        for i in 0..self.farmhands.len() {
            // A cabin finished today becomes the home of a farmhand without one.
            if self.farmhands[i].farmer.home_location.is_none() {
                let name = self.farmhands[i].farmer.name.clone();
                self.farmhands[i].farmer.home_location = self.claim_cabin(&name);
            }
            let (home, tile) = self.home_wake(&self.farmhands[i].farmer, i);
            self.as_farmhand(i, |w| {
                w.farmer_night_reset(passed_out);
                w.current = home;
                w.farmer.set_tile(tile);
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
