//! Villagers: schedule-driven movement across locations, dialogue, gifts.

use std::collections::{HashMap, VecDeque};

use engine::{IRect, Vec2};
use serde::{Deserialize, Serialize};

use crate::schedule::ScheduleEntry;
use crate::{Dir, TilePos, TILE_F};

/// NPC walking speed in world pixels per tick (approximation).
pub const NPC_SPEED: f32 = 2.0;

/// One leg of a route: walk `path` inside `location`; if `warp_to` is set,
/// the last tile is a warp into another location.
#[derive(Clone, Debug, PartialEq)]
pub struct RouteLeg {
    pub location: String,
    pub path: VecDeque<TilePos>,
    pub warp_to: Option<(String, TilePos)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NpcHome {
    pub location: String,
    pub tile: TilePos,
    pub facing: Dir,
}

#[derive(Clone, Debug)]
pub struct Npc {
    pub name: String,
    pub display_name: String,
    pub texture: String,
    pub portrait: String,
    pub location: String,
    /// World-space top-left of the 64x64 footprint (same convention as the farmer).
    pub pos: Vec2,
    pub facing: Dir,
    pub home: NpcHome,
    /// Home from the character data (where they return after a divorce).
    pub original_home: NpcHome,
    pub birthday: Option<(String, i32)>,
    pub can_romance: bool,
    pub dialogue: HashMap<String, String>,
    pub schedule: Vec<ScheduleEntry>,
    pub next_entry: usize,
    pub route: VecDeque<RouteLeg>,
    pub end_facing: Dir,
    pub moving: bool,
    pub walk_ms: u32,
    /// Text shown in a speech bubble above the head, with remaining ms.
    pub bubble: Option<(String, u32)>,
    /// An emote over the head: (emote id, ms shown so far).
    pub emote: Option<(i32, u32)>,
    /// Pause after being talked to (ms) during which the NPC stands still.
    pub pause_ms: u32,
    /// Outfit choices from `Appearance`, the default sprite/portrait, and
    /// today's (sprite, portrait) picks for indoors and outdoors.
    pub appearances: Vec<valley_content::data::CharacterAppearance>,
    pub base_look: (String, String),
    pub look_indoors: (String, String),
    pub look_outdoors: (String, String),
    /// `UnlockConditions`: the NPC is only around while this passes.
    pub unlock: Option<String>,
    /// The rest of the NPC's `Data/Characters` entry (social settings).
    pub data: std::sync::Arc<valley_content::data::CharacterData>,
}

impl Npc {
    pub fn bbox(&self) -> IRect {
        IRect::new(self.pos.x as i32 + 8, self.pos.y as i32 + 32, 48, 32)
    }

    pub fn tile(&self) -> TilePos {
        TilePos::from_world(self.bbox().center())
    }

    pub fn set_tile(&mut self, t: TilePos) {
        self.pos = Vec2::new(t.x as f32 * TILE_F, t.y as f32 * TILE_F - 16.0);
    }

    fn tile_pos_vec(t: TilePos) -> Vec2 {
        Vec2::new(t.x as f32 * TILE_F, t.y as f32 * TILE_F - 16.0)
    }

    /// Advance movement by one tick. Returns a location change if the NPC
    /// passed through a warp: (new location).
    pub fn step(&mut self, dt_ms: u32) -> Option<String> {
        if let Some((_, ms)) = &mut self.bubble {
            *ms = ms.saturating_sub(dt_ms);
            if *ms == 0 {
                self.bubble = None;
            }
        }
        if let Some((_, ms)) = &mut self.emote {
            *ms += dt_ms;
            if *ms > 1600 {
                self.emote = None;
            }
        }
        if self.pause_ms > 0 {
            self.pause_ms = self.pause_ms.saturating_sub(dt_ms);
            self.moving = false;
            return None;
        }
        let Some(leg) = self.route.front_mut() else {
            if self.moving {
                self.facing = self.end_facing;
            }
            self.moving = false;
            return None;
        };
        if leg.location != self.location {
            // Stale route.
            self.route.clear();
            self.moving = false;
            return None;
        }
        let Some(&next) = leg.path.front() else {
            let leg = self.route.pop_front().unwrap();
            if let Some((loc, tile)) = leg.warp_to {
                self.location = loc.clone();
                self.set_tile(tile);
                return Some(loc);
            }
            return None;
        };
        let target = Self::tile_pos_vec(next);
        let d = target - self.pos;
        let dist = d.length();
        self.moving = true;
        self.walk_ms += dt_ms;
        if dist <= NPC_SPEED {
            self.pos = target;
            leg.path.pop_front();
        } else {
            let v = d.normalized() * NPC_SPEED;
            self.pos += v;
            self.facing = Dir::toward(d.x.round() as i32, d.y.round() as i32);
        }
        None
    }

    pub fn is_traveling(&self) -> bool {
        !self.route.is_empty()
    }
}

impl crate::world::World {
    /// Morning: pick today's outfits (indoors and outdoors) from each
    /// NPC's `Appearance` list, lowest precedence first and then by
    /// weight; hide NPCs whose `UnlockConditions` fail.
    pub(crate) fn npc_appearances_morning(&mut self) {
        let season = self.clock.season;
        for i in 0..self.npcs.len() {
            let n = &self.npcs[i];
            if let Some(c) = n.unlock.clone() {
                let mut rng = self.day_rng(600 + i as i64);
                if !self.check_query(Some(&c), None, None, &mut rng) {
                    self.npcs[i].location.clear();
                    continue;
                }
            }
            if n.appearances.is_empty() {
                continue;
            }
            let mut picks = Vec::new();
            for indoors in [true, false] {
                let mut rng = self.day_rng(700 + i as i64 * 2 + indoors as i64);
                let ok: Vec<&valley_content::data::CharacterAppearance> = n
                    .appearances
                    .iter()
                    .filter(|a| !a.is_island_attire)
                    .filter(|a| a.season.as_deref().map_or(true, |s| crate::time::Season::parse(s) == Some(season)))
                    .filter(|a| if indoors { a.indoors } else { a.outdoors })
                    .filter(|a| self.check_query(a.condition.as_deref(), None, None, &mut rng.clone()))
                    .collect();
                let best = ok.iter().map(|a| a.precedence).min();
                let group: Vec<&&valley_content::data::CharacterAppearance> = ok.iter().filter(|a| Some(a.precedence) == best).collect();
                let total: i32 = group.iter().map(|a| a.weight.max(1)).sum();
                let mut roll = if total > 0 { rng.range(0, total) } else { 0 };
                let chosen = group.iter().find(|a| {
                    roll -= a.weight.max(1);
                    roll < 0
                });
                let look = match chosen {
                    Some(a) => (
                        a.sprite.clone().map(|s| s.replace('\\', "/")).unwrap_or_else(|| n.base_look.0.clone()),
                        a.portrait.clone().map(|s| s.replace('\\', "/")).unwrap_or_else(|| n.base_look.1.clone()),
                    ),
                    None => n.base_look.clone(),
                };
                picks.push(look);
            }
            let n = &mut self.npcs[i];
            n.look_indoors = picks[0].clone();
            n.look_outdoors = picks[1].clone();
        }
        self.refresh_npc_looks();
    }

    /// Dress NPCs for where they are now.
    pub(crate) fn refresh_npc_looks(&mut self) {
        for n in self.npcs.iter_mut() {
            let outdoors = self.locations.get(&n.location).is_some_and(|l| l.outdoors);
            let look = if outdoors { &n.look_outdoors } else { &n.look_indoors };
            n.texture = look.0.clone();
            n.portrait = look.1.clone();
        }
    }
}
