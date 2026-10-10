//! Festivals: `Data/Festivals/FestivalDates` plus one string table per
//! festival (`Data/Festivals/<season><day>`) with "name", "conditions"
//! ("Location/start end"), "set-up" and "mainEvent" scripts and per-NPC
//! festival dialogue. The set-up runs as an event; afterwards the farmer
//! roams freely until talking to the host starts the main event.

use std::collections::{HashMap, HashSet};

use engine::Vec2;

use crate::collision::box_blocked;
use crate::dialogue::{self, Substitutions};
use crate::events::split_commands;
use crate::farmer::Farmer;
use crate::input::PlayerInput;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos, TILE_F};

#[derive(Clone, Debug, PartialEq)]
pub struct FestivalState {
    pub key: String,
    pub name: String,
    pub host: Option<String>,
    pub main_event: Vec<String>,
    pub dialogue: HashMap<String, String>,
    pub roaming: bool,
    pub main_started: bool,
    pub talked: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FestivalInfo {
    pub key: String,
    pub name: String,
    pub location: String,
    pub start: i32,
    pub end: i32,
    pub data: HashMap<String, String>,
}

pub fn parse_festival(key: &str, data: HashMap<String, String>) -> Option<FestivalInfo> {
    let cond = data.get("conditions")?;
    let (loc, times) = cond.split_once('/')?;
    let t: Vec<i32> = times.split_whitespace().filter_map(|v| v.parse().ok()).collect();
    Some(FestivalInfo {
        key: key.to_string(),
        name: data.get("name").cloned().unwrap_or_else(|| key.to_string()),
        location: loc.to_string(),
        start: *t.first().unwrap_or(&900),
        end: *t.get(1).unwrap_or(&1400),
        data,
    })
}

/// Host of a festival: an explicit "host" key, else the first actor in
/// the main event script.
fn host_of(f: &FestivalInfo) -> Option<String> {
    if let Some(h) = f.data.get("host") {
        return Some(h.trim().to_string());
    }
    let main = f.data.get("mainEvent")?;
    let cmds = split_commands(main);
    cmds.iter().find_map(|c| c.strip_prefix("speak ").map(|r| r.split_whitespace().next().unwrap_or("").to_string()))
}

impl World {
    pub fn festival_today(&self) -> Option<&FestivalInfo> {
        let key = format!("{}{}", self.clock.season.key(), self.clock.day);
        self.festivals.get(&key)
    }

    /// Called when entering a location: start today's festival there.
    pub(crate) fn check_festival(&mut self) -> bool {
        if self.event.is_some() || self.festival_done_today {
            return false;
        }
        let Some(f) = self.festival_today().cloned() else { return false };
        if f.location != self.current {
            return false;
        }
        if self.clock.time < f.start || self.clock.time >= f.end {
            if self.clock.time < f.start {
                self.emit(WorldEvent::Message(format!("The {} hasn't started yet. Come back at {}.", f.name, crate::time::Clock::time_string(f.start))));
            }
            return false;
        }
        let Some(setup) = f.data.get("set-up").cloned() else { return false };
        self.start_event(&format!("festival_{}", f.key), &setup);
        if let Some(e) = self.event.as_mut() {
            e.festival = Some(FestivalState {
                key: f.key.clone(),
                name: f.name.clone(),
                host: host_of(&f),
                main_event: f.data.get("mainEvent").map(|m| split_commands(m)).unwrap_or_default(),
                dialogue: f.data.clone(),
                roaming: false,
                main_started: false,
                talked: HashSet::new(),
            });
            // Make sure the farmer is an actor.
            if !e.actors.iter().any(|a| a.name == "farmer") {
                let t = self.farmer.tile();
                e.actors.push(crate::events::Actor {
                    name: "farmer".into(),
                    pos: Vec2::new(t.x as f32 * TILE_F, t.y as f32 * TILE_F - 16.0),
                    facing: self.farmer.facing,
                    texture: self.farmer.appearance.texture_name(),
                    moving: None,
                    walk_ms: 0,
                    emote: None,
                });
            }
        }
        true
    }

    pub fn festival_roaming(&self) -> bool {
        self.event.as_ref().and_then(|e| e.festival.as_ref()).is_some_and(|f| f.roaming)
    }

    /// Free movement and talking during a festival.
    pub(crate) fn update_festival_roam(&mut self, input: &PlayerInput, dt: u32) {
        let Some(mut e) = self.event.take() else { return };
        let speed = crate::farmer::RUN_SPEED;
        let loc_name = e.location.clone();
        let Some(fi) = e.actors.iter().position(|a| a.name == "farmer") else {
            self.event = Some(e);
            return;
        };
        let mut moved = false;
        if input.move_x != 0 || input.move_y != 0 {
            let facing = if input.move_y < 0 { Dir::Up } else if input.move_y > 0 { Dir::Down } else if input.move_x > 0 { Dir::Right } else { Dir::Left };
            e.actors[fi].facing = facing;
            for d in [Vec2::new(input.move_x as f32 * speed, 0.0), Vec2::new(0.0, input.move_y as f32 * speed)] {
                if d.x == 0.0 && d.y == 0.0 {
                    continue;
                }
                let np = e.actors[fi].pos + d;
                let bb = Farmer::bbox_at(np);
                let blocked_by_map = self.locations.get(&loc_name).map_or(true, |l| box_blocked(l, bb));
                let blocked_by_actor = e.actors.iter().enumerate().any(|(i, a)| i != fi && Farmer::bbox_at(a.pos).intersects(&bb));
                if !blocked_by_map && !blocked_by_actor {
                    e.actors[fi].pos = np;
                    moved = true;
                }
            }
        }
        if moved {
            e.actors[fi].walk_ms += dt;
            e.actors[fi].moving = Some(Vec2::new(0.1, 0.0));
        } else {
            e.actors[fi].moving = None;
        }
        // Camera follows the farmer while roaming.
        let c = e.actors[fi].pos;
        e.viewport = Vec2::new(c.x / TILE_F, (c.y + 16.0) / TILE_F);
        if input.action {
            let me = TilePos::from_world(Farmer::bbox_at(e.actors[fi].pos).center());
            let target = me.offset(e.actors[fi].facing);
            let hit = e.actors.iter().position(|a| a.name != "farmer" && {
                let t = TilePos::from_world(Farmer::bbox_at(a.pos).center());
                t == target || t == me
            });
            if let Some(i) = hit {
                let name = e.actors[i].name.clone();
                let toward = Dir::toward(me.x - target.x, me.y - target.y);
                e.actors[i].facing = toward;
                let fest = e.festival.as_mut().unwrap();
                if fest.host.as_deref() == Some(name.as_str()) {
                    self.emit(WorldEvent::FestivalHost { festival: fest.name.clone() });
                } else {
                    let raw = fest.dialogue.get(&name).cloned().unwrap_or_else(|| "Isn't this fun? I look forward to this all year.".into());
                    if fest.talked.insert(name.clone()) {
                        let f = self.farmer.friendship(&name);
                        if !f.talked_today {
                            f.talked_today = true;
                            f.points += 20;
                        }
                        f.met = true;
                    }
                    let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
                    let pages = dialogue::parse(&raw, &subs);
                    self.emit(WorldEvent::Dialogue { npc: Some(name), pages });
                }
            }
        }
        self.event = Some(e);
    }

    /// Talked to the host and agreed: run the main event.
    pub fn festival_begin_main(&mut self) {
        let Some(e) = self.event.as_mut() else { return };
        let Some(f) = e.festival.as_mut() else { return };
        f.roaming = false;
        f.main_started = true;
        e.commands = f.main_event.clone();
        e.pc = 0;
        e.finished = false;
        e.waiting_ui = false;
    }

    /// Leave a festival early (or after it ends).
    pub fn leave_festival(&mut self) {
        if self.event.as_ref().is_some_and(|e| e.festival.is_some()) {
            self.finish_event();
        }
    }

    /// After a festival the day skips to 10pm at home.
    pub(crate) fn end_festival_day(&mut self) {
        self.festival_done_today = true;
        self.clock.time = self.clock.time.max(2200);
        let wake = self.bed_tile;
        self.warp_farmer("FarmHouse", TilePos::new(wake.x - 1, wake.y), Some(Dir::Down));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_conditions_and_host() {
        let mut d = HashMap::new();
        d.insert("conditions".to_string(), "Town/900 1400".to_string());
        d.insert("mainEvent".to_string(), "pause 100/speak Mara \"Welcome!\"/end".to_string());
        let f = parse_festival("fall16", d).unwrap();
        assert_eq!((f.location.as_str(), f.start, f.end), ("Town", 900, 1400));
        assert_eq!(host_of(&f).as_deref(), Some("Mara"));
    }
}
