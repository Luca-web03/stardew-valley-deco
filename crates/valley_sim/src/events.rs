//! Cutscene events (`Data/Events/<Location>`), following the publicly
//! documented script format: key = "id/precondition/...", value =
//! "music/viewportX viewportY/actor x y dir .../command/command/...".
//! A core subset of commands is interpreted; unknown commands are skipped.

use engine::{Rng, Vec2};

use crate::dialogue::{self, Substitutions};
use crate::gsq::{self, QueryCtx};
use crate::time::Weather;
use crate::world::{World, WorldEvent};
use crate::{Dir, TILE_F};

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    /// NPC name or "farmer".
    pub name: String,
    pub pos: Vec2,
    pub facing: Dir,
    pub texture: String,
    /// Remaining movement in pixels (dx, dy).
    pub moving: Option<Vec2>,
    pub walk_ms: u32,
    pub emote: Option<(i32, u32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventRunner {
    pub id: String,
    pub location: String,
    pub commands: Vec<String>,
    pub pc: usize,
    pub actors: Vec<Actor>,
    /// Camera center in tiles.
    pub viewport: Vec2,
    pub wait_ms: u32,
    /// Waiting for the UI to close a dialogue/message.
    pub waiting_ui: bool,
    /// Waiting for an actor to finish moving.
    pub waiting_move: Option<String>,
    pub skippable: bool,
    pub finished: bool,
}

/// Split a script on '/' while keeping quoted text intact.
pub fn split_commands(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            '/' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

/// Tokenize a command (quoted strings stay whole, without quotes).
pub fn tokens(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in cmd.chars() {
        match c {
            '"' => {
                if quoted {
                    out.push(std::mem::take(&mut cur));
                }
                quoted = !quoted;
            }
            ' ' if !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

impl World {
    pub fn in_event(&self) -> bool {
        self.event.is_some()
    }

    /// Check a key's preconditions ("id/f Mara 250/t 600 1200/...").
    pub fn event_preconditions(&self, key: &str) -> bool {
        let mut parts = key.split('/');
        let _id = parts.next();
        for p in parts {
            let t: Vec<&str> = p.split_whitespace().collect();
            let Some(&c) = t.first() else { continue };
            let args = &t[1..];
            let ok = match c {
                "f" => args.chunks(2).all(|ch| ch.len() == 2 && self.farmer.friendships.get(ch[0]).map(|f| f.points).unwrap_or(0) >= ch[1].parse().unwrap_or(0)),
                "t" => args.len() >= 2 && self.clock.time >= args[0].parse().unwrap_or(0) && self.clock.time <= args[1].parse().unwrap_or(2600),
                "w" => match args.first().copied() {
                    Some("rainy") => self.weather.is_raining(),
                    Some("sunny") => !self.weather.is_raining(),
                    _ => true,
                },
                "d" => !args.iter().any(|d| d.eq_ignore_ascii_case(self.clock.weekday())),
                "z" => !args.iter().any(|s| s.eq_ignore_ascii_case(self.clock.season.key())),
                "e" => args.iter().all(|e| self.farmer.events_seen.iter().any(|x| x == e)),
                "k" => !args.iter().any(|e| self.farmer.events_seen.iter().any(|x| x == e)),
                "n" | "Hn" => args.iter().all(|m| self.farmer.mail_received.iter().any(|x| x == m)),
                "l" | "Hl" => !args.iter().any(|m| self.farmer.mail_received.iter().any(|x| x == m)),
                "y" => self.clock.year >= args.first().and_then(|v| v.parse().ok()).unwrap_or(1),
                "u" => args.iter().any(|d| d.parse::<i32>().ok() == Some(self.clock.day)),
                "G" => {
                    let mut rng = Rng::new(self.seed);
                    let q = args.join(" ");
                    let mail = self.farmer.mail_received.clone();
                    let seen = self.farmer.events_seen.clone();
                    let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng };
                    gsq::check(Some(&q), &mut ctx)
                }
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    }

    /// Look for an event to start in the current location.
    pub fn check_location_events(&mut self) {
        if self.event.is_some() {
            return;
        }
        let Some(events) = self.event_data.get(&self.current).cloned() else { return };
        let mut keys: Vec<&String> = events.keys().collect();
        keys.sort();
        for key in keys {
            let id = key.split('/').next().unwrap_or("").to_string();
            if id.is_empty() || self.farmer.events_seen.contains(&id) || !key.contains('/') && events.len() > 1 && id.parse::<i64>().is_err() {
                continue;
            }
            if !self.event_preconditions(key) {
                continue;
            }
            self.start_event(&id, &events[key]);
            return;
        }
    }

    pub fn start_event(&mut self, id: &str, script: &str) {
        let cmds = split_commands(script);
        if cmds.len() < 3 {
            return;
        }
        let vp: Vec<f32> = cmds[1].split_whitespace().filter_map(|v| v.parse().ok()).collect();
        let viewport = if vp.len() >= 2 { Vec2::new(vp[0], vp[1]) } else { let t = self.farmer.tile(); Vec2::new(t.x as f32, t.y as f32) };
        let mut actors = Vec::new();
        let at: Vec<&str> = cmds[2].split_whitespace().collect();
        for c in at.chunks(4) {
            if c.len() < 4 {
                continue;
            }
            let (Ok(x), Ok(y)) = (c[1].parse::<i32>(), c[2].parse::<i32>()) else { continue };
            let facing = c[3].parse::<i32>().map(Dir::from_index).unwrap_or(Dir::Down);
            let texture = if c[0] == "farmer" {
                "Characters/Farmer/dev_farmer".to_string()
            } else {
                self.npcs.iter().find(|n| n.name == c[0]).map(|n| n.texture.clone()).unwrap_or_else(|| format!("Characters/{}", c[0]))
            };
            actors.push(Actor { name: c[0].to_string(), pos: Vec2::new(x as f32 * TILE_F, y as f32 * TILE_F - 16.0), facing, texture, moving: None, walk_ms: 0, emote: None });
        }
        if !cmds[0].is_empty() && cmds[0] != "none" && cmds[0] != "continue" {
            self.events.push(WorldEvent::Music(cmds[0].clone()));
        }
        self.event = Some(EventRunner {
            id: id.to_string(),
            location: self.current.clone(),
            commands: cmds[3..].to_vec(),
            pc: 0,
            actors,
            viewport,
            wait_ms: 0,
            waiting_ui: false,
            waiting_move: None,
            skippable: false,
            finished: false,
        });
        self.farmer.tool_use = None;
        self.weapon_swing = None;
    }

    /// The UI closed the dialogue the event was waiting on.
    pub fn event_resume(&mut self) {
        if let Some(e) = self.event.as_mut() {
            e.waiting_ui = false;
        }
    }

    /// Skip to the end (if the event allows it).
    pub fn skip_event(&mut self) {
        if self.event.as_ref().is_some_and(|e| e.skippable) {
            self.finish_event();
        }
    }

    fn finish_event(&mut self) {
        let Some(e) = self.event.take() else { return };
        if !self.farmer.events_seen.contains(&e.id) {
            self.farmer.events_seen.push(e.id.clone());
        }
        // The farmer ends where their actor stood.
        if let Some(a) = e.actors.iter().find(|a| a.name == "farmer") {
            self.farmer.pos = a.pos;
            self.farmer.facing = a.facing;
        }
        self.emit(WorldEvent::EventEnded);
    }

    /// Advance the running event by one tick.
    pub(crate) fn update_event(&mut self, dt: u32) {
        let Some(mut e) = self.event.take() else { return };
        // Move actors.
        let mut arrived = Vec::new();
        for a in e.actors.iter_mut() {
            if let Some(off) = a.moving {
                let step = 3.0f32;
                let d = if off.length() <= step { off } else { off.normalized() * step };
                a.pos += d;
                a.walk_ms += dt;
                let left = off - d;
                if left.length() < 0.5 {
                    a.moving = None;
                    arrived.push(a.name.clone());
                } else {
                    a.moving = Some(left);
                }
            }
            if let Some((id, ms)) = a.emote {
                a.emote = if ms > dt { Some((id, ms - dt)) } else { None };
            }
        }
        if let Some(w) = &e.waiting_move {
            if arrived.contains(w) || !e.actors.iter().any(|a| &a.name == w && a.moving.is_some()) {
                e.waiting_move = None;
            }
        }
        if e.wait_ms > 0 {
            e.wait_ms = e.wait_ms.saturating_sub(dt);
        }
        // Run commands until one blocks.
        let mut guard = 0;
        while !e.waiting_ui && e.waiting_move.is_none() && e.wait_ms == 0 && e.pc < e.commands.len() && guard < 64 {
            guard += 1;
            let cmd = e.commands[e.pc].clone();
            e.pc += 1;
            if self.run_event_command(&mut e, &cmd) {
                break;
            }
        }
        if e.finished || (e.pc >= e.commands.len() && !e.waiting_ui && e.waiting_move.is_none() && e.wait_ms == 0) {
            self.event = Some(e);
            self.finish_event();
            return;
        }
        self.event = Some(e);
    }

    /// Execute one command; returns true to stop processing this tick.
    fn run_event_command(&mut self, e: &mut EventRunner, cmd: &str) -> bool {
        let t = tokens(cmd);
        let Some(name) = t.first().map(|s| s.as_str()) else { return false };
        let arg = |i: usize| t.get(i).cloned().unwrap_or_default();
        let num = |i: usize| t.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        match name {
            "pause" => {
                e.wait_ms = num(1).max(0) as u32;
                true
            }
            "move" => {
                // move <actor> <dx> <dy> <facing> [continue]
                let mut i = 1;
                let mut last = None;
                while i + 3 < t.len() + 1 && t.get(i + 3).is_some() {
                    let actor = arg(i);
                    let (dx, dy) = (num(i + 1), num(i + 2));
                    let facing = Dir::from_index(num(i + 3));
                    if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                        a.moving = Some(Vec2::new(dx as f32 * TILE_F, dy as f32 * TILE_F));
                        a.facing = if dx != 0 || dy != 0 { Dir::toward(dx, dy) } else { facing };
                        last = Some((actor, facing));
                    }
                    i += 4;
                    if t.get(i).map(|s| s == "true" || s == "false").unwrap_or(false) {
                        if t[i] == "true" {
                            last = None;
                        }
                        i += 1;
                    }
                }
                if let Some((actor, facing)) = last {
                    e.waiting_move = Some(actor.clone());
                    // Final facing applies on arrival.
                    e.commands.insert(e.pc, format!("faceDirection {actor} {} true", facing.index()));
                    return true;
                }
                false
            }
            "faceDirection" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.facing = Dir::from_index(num(2));
                }
                false
            }
            "warp" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.pos = Vec2::new(num(2) as f32 * TILE_F, num(3) as f32 * TILE_F - 16.0);
                    a.moving = None;
                }
                false
            }
            "speak" => {
                let who = arg(1);
                let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
                let pages = dialogue::parse(&arg(2), &subs);
                self.emit(WorldEvent::Dialogue { npc: Some(who), pages });
                e.waiting_ui = true;
                true
            }
            "message" => {
                let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
                let text = dialogue::substitute(&arg(1), &subs).replace('^', "\n");
                self.emit(WorldEvent::Message(text));
                e.waiting_ui = true;
                true
            }
            "emote" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.emote = Some((num(2), 1200));
                }
                if arg(3) != "true" {
                    e.wait_ms = 1200;
                    return true;
                }
                false
            }
            "viewport" => {
                if arg(1) == "move" {
                    e.viewport += Vec2::new(num(2) as f32, num(3) as f32);
                } else if t.len() >= 3 {
                    e.viewport = Vec2::new(num(1) as f32, num(2) as f32);
                }
                false
            }
            "playSound" => {
                self.sound(&arg(1));
                false
            }
            "playMusic" => {
                self.events.push(WorldEvent::Music(arg(1)));
                false
            }
            "addMailReceived" | "mail" => {
                let m = arg(1);
                if name == "mail" {
                    self.farmer.mailbox.push(m);
                } else if !self.farmer.mail_received.contains(&m) {
                    self.farmer.mail_received.push(m);
                }
                false
            }
            "addItem" => {
                let n = if t.len() > 2 { num(2).max(1) } else { 1 };
                if let Some(item) = self.items.create(&arg(1), n) {
                    self.give_item(item);
                }
                false
            }
            "friendship" => {
                let f = self.farmer.friendship(&arg(1));
                f.points = (f.points + num(2)).clamp(0, 2500);
                f.met = true;
                false
            }
            "money" => {
                self.farmer.money += num(1);
                false
            }
            "skippable" => {
                e.skippable = true;
                false
            }
            "globalFade" | "globalFadeToClear" | "fade" => {
                self.emit(WorldEvent::Fade { to_black: name == "globalFade" });
                e.wait_ms = 500;
                true
            }
            "end" => {
                e.finished = true;
                if arg(1) == "warpOut" || arg(1) == "position" {
                    // Leave the actor positions as they are.
                }
                true
            }
            "addQuest" => {
                self.add_quest(&arg(1));
                false
            }
            _ => false,
        }
    }

    /// Camera focus while an event runs (world pixels).
    pub fn event_camera_focus(&self) -> Option<Vec2> {
        let e = self.event.as_ref()?;
        Some(Vec2::new(e.viewport.x * TILE_F + TILE_F / 2.0, e.viewport.y * TILE_F + TILE_F / 2.0))
    }

    /// Weather helper for event scripts.
    pub fn is_sunny(&self) -> bool {
        self.weather == Weather::Sun
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_tokenizes_with_quotes() {
        let c = split_commands("none/5 5/farmer 1 2 0/speak Mara \"Hi/there\"/end");
        assert_eq!(c.len(), 5);
        assert_eq!(c[3], "speak Mara \"Hi/there\"");
        assert_eq!(tokens(&c[3]), vec!["speak", "Mara", "Hi/there"]);
    }
}
