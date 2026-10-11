//! Cutscene events (`Data/Events/<Location>`), following the publicly
//! documented script format: key = "id/precondition/...", value =
//! "music/viewportX viewportY/actor x y dir .../command/command/...".
//!
//! Supported commands: actor movement and looks (`move`, `advancedMove`,
//! `faceDirection`, `warp`, `speed`, `halt`, `showFrame`, `animate`,
//! `stopAnimation`, `positionOffset`, `jump`, `shake`, `emote`,
//! `textAboveHead`, `addTemporaryActor`, `removeTemporaryActor`,
//! `changeSprite`, `waitForAllStationary`), talk (`speak`, `splitSpeak`,
//! `message`, `question`, `quickQuestion`), flow (`pause`, `fork`,
//! `switchEvent`, `skippable`, `end` and its variants), the camera
//! (`viewport`), sound (`playSound`, `playMusic`, `stopMusic`), screen
//! effects (`globalFade`, `globalFadeToClear`, `screenFlash`,
//! `ambientLight`), props (`addObject`, `addBigProp`, `removeObject`,
//! `temporaryAnimatedSprite`, `removeTemporarySprites`, `removeSprite`),
//! the map (`changeMapTile`, `removeTile`, `changeToTemporaryMap`,
//! `changeLocation`, `doAction`), and rewards/state (`addItem`,
//! `removeItem`, `addMailReceived`, `mail`, `friendship`, `money`,
//! `addQuest`, `removeQuest`, `addCookingRecipe`, `addCraftingRecipe`,
//! `addConversationTopic`, `eventSeen`, `awardFestivalPrize`).
//! Code-only commands (`specificTemporarySprite`, `cutscene`, ...) are
//! skipped.

use std::collections::VecDeque;

use engine::{Color, Rng, Vec2};

use crate::dialogue::{self, Substitutions};
use crate::gsq::{self, QueryCtx};
use crate::time::Weather;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos, TILE_F};

/// An actor's frame animation (`animate`).
#[derive(Clone, Debug, PartialEq)]
pub struct ActorAnim {
    pub frames: Vec<i32>,
    pub frame_ms: u32,
    pub looping: bool,
    pub flip: bool,
    pub elapsed: u32,
}

impl ActorAnim {
    pub fn current(&self) -> i32 {
        if self.frames.is_empty() {
            return 0;
        }
        let i = (self.elapsed / self.frame_ms.max(1)) as usize;
        if self.looping {
            self.frames[i % self.frames.len()]
        } else {
            self.frames[i.min(self.frames.len() - 1)]
        }
    }
}

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
    /// Fixed sprite frame (`showFrame`).
    pub frame: Option<i32>,
    pub anim: Option<ActorAnim>,
    /// Hop height and its velocity (`jump`), in world pixels.
    pub z: f32,
    pub vz: f32,
    pub shake_ms: u32,
    pub visible: bool,
    /// Speech bubble text and remaining ms (`textAboveHead`).
    pub speech: Option<(String, u32)>,
    /// Sprite frame size in art pixels.
    pub size: (i32, i32),
    /// Walking speed in world pixels per tick.
    pub speed: f32,
    /// Queued moves for `advancedMove` (tile offsets) and whether they loop.
    pub path: VecDeque<Vec2>,
    pub path_loop: bool,
    pub flip: bool,
}

impl Actor {
    pub fn new(name: &str, pos: Vec2, facing: Dir, texture: String) -> Self {
        Self {
            name: name.to_string(),
            pos,
            facing,
            texture,
            moving: None,
            walk_ms: 0,
            emote: None,
            frame: None,
            anim: None,
            z: 0.0,
            vz: 0.0,
            shake_ms: 0,
            visible: true,
            speech: None,
            size: (16, 32),
            speed: 3.0,
            path: VecDeque::new(),
            path_loop: false,
            flip: false,
        }
    }

    /// Sprite frame to draw: animation, fixed frame, or walking.
    pub fn sprite_frame(&self) -> Option<i32> {
        self.anim.as_ref().map(|a| a.current()).or(self.frame)
    }
}

/// A prop placed by the event (drawn only while it runs).
#[derive(Clone, Debug, PartialEq)]
pub enum EventProp {
    /// An object or big craftable icon on a tile.
    Object { tile: TilePos, id: String, big: bool },
    /// `temporaryAnimatedSprite`: frames laid out left to right.
    Sprite {
        texture: String,
        src: (i32, i32, i32, i32),
        /// World pixel position.
        pos: Vec2,
        interval_ms: u32,
        frames: i32,
        /// Loops left (-1 forever).
        loops: i32,
        flip: bool,
        scale: f32,
        alpha: f32,
        alpha_fade: f32,
        rotation: f32,
        rotation_change: f32,
        elapsed: u32,
    },
}

/// How the event ends (`end <variant>`).
#[derive(Clone, Debug, PartialEq)]
pub enum EndAction {
    /// Back where the farmer was before the event.
    Return,
    /// Stay where the farmer's actor stands.
    Stay,
    Position(TilePos),
    WarpOut,
    Bed,
    NewDay,
}

#[derive(Clone, Debug)]
pub struct EventRunner {
    pub id: String,
    pub location: String,
    pub commands: Vec<String>,
    pub pc: usize,
    pub actors: Vec<Actor>,
    /// Camera center in tiles.
    pub viewport: Vec2,
    /// Camera pan: (velocity in tiles/ms, ms left).
    pub viewport_pan: Option<(Vec2, u32)>,
    pub wait_ms: u32,
    /// Waiting for the UI to close a dialogue/message.
    pub waiting_ui: bool,
    /// Waiting for an actor to finish moving.
    pub waiting_move: Option<String>,
    /// Waiting until no actor moves (`waitForAllStationary`).
    pub waiting_still: bool,
    pub skippable: bool,
    pub finished: bool,
    /// Present while this event is a festival.
    pub festival: Option<crate::festivals::FestivalState>,
    /// Original map while a temporary map or tile edits are shown.
    pub saved_map: Option<valley_content::Map>,
    /// Answer index that triggers `fork` (from `question forkN`).
    pub fork_answer: Option<usize>,
    pub fork_ok: bool,
    pub last_answer: Option<usize>,
    /// `quickQuestion` branches waiting for the answer.
    pub quick_branches: Option<Vec<String>>,
    /// A question is open; only an answer resumes the script.
    pub awaiting_answer: bool,
    pub props: Vec<EventProp>,
    /// Ambient light tint (`ambientLight`).
    pub ambient: Option<Color>,
    /// Where the farmer was before the event (location, position, facing).
    pub start: (String, Vec2, Dir),
    pub end: EndAction,
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

/// Pixel position of an actor standing on tile (x, y).
fn tile_pos(x: i32, y: i32) -> Vec2 {
    Vec2::new(x as f32 * TILE_F, y as f32 * TILE_F - 16.0)
}

fn truthy(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
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
                    let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng, world: Some(self) };
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

    fn actor_texture(&self, name: &str) -> String {
        if name == "farmer" {
            self.farmer.appearance.texture_name()
        } else {
            self.npcs.iter().find(|n| n.name == name).map(|n| n.texture.clone()).unwrap_or_else(|| format!("Characters/{name}"))
        }
    }

    pub fn start_event(&mut self, id: &str, script: &str) {
        let cmds = split_commands(script);
        if cmds.len() < 3 {
            return;
        }
        let vp: Vec<f32> = cmds[1].split_whitespace().filter_map(|v| v.parse().ok()).collect();
        let viewport = if vp.len() >= 2 {
            Vec2::new(vp[0], vp[1])
        } else {
            let t = self.farmer.tile();
            Vec2::new(t.x as f32, t.y as f32)
        };
        let mut actors = Vec::new();
        let at: Vec<&str> = cmds[2].split_whitespace().collect();
        for c in at.chunks(4) {
            if c.len() < 4 {
                continue;
            }
            let (Ok(x), Ok(y)) = (c[1].parse::<i32>(), c[2].parse::<i32>()) else { continue };
            let facing = c[3].parse::<i32>().map(Dir::from_index).unwrap_or(Dir::Down);
            let texture = self.actor_texture(c[0]);
            actors.push(Actor::new(c[0], tile_pos(x, y), facing, texture));
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
            viewport_pan: None,
            wait_ms: 0,
            waiting_ui: false,
            waiting_move: None,
            waiting_still: false,
            skippable: false,
            finished: false,
            festival: None,
            saved_map: None,
            fork_answer: None,
            fork_ok: false,
            last_answer: None,
            quick_branches: None,
            awaiting_answer: false,
            props: Vec::new(),
            ambient: None,
            start: (self.current.clone(), self.farmer.pos, self.farmer.facing),
            end: EndAction::Return,
        });
        self.farmer.tool_use = None;
        self.weapon_swing = None;
    }

    /// The UI closed the dialogue the event was waiting on.
    pub fn event_resume(&mut self) {
        if let Some(e) = self.event.as_mut() {
            if !e.awaiting_answer {
                e.waiting_ui = false;
            }
        }
    }

    /// The answer to an event `question` / `quickQuestion`.
    pub fn event_answer(&mut self, i: usize) {
        let Some(e) = self.event.as_mut() else { return };
        e.awaiting_answer = false;
        e.last_answer = Some(i);
        if e.fork_answer == Some(i) {
            e.fork_ok = true;
        }
        if let Some(branches) = e.quick_branches.take() {
            if let Some(b) = branches.get(i) {
                let cmds: Vec<String> = b.split('\\').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                for (k, c) in cmds.into_iter().enumerate() {
                    e.commands.insert(e.pc + k, c);
                }
            }
        }
        e.waiting_ui = false;
    }

    /// Skip to the end (if the event allows it).
    pub fn skip_event(&mut self) {
        if self.event.as_ref().is_some_and(|e| e.skippable) {
            self.finish_event();
        }
    }

    pub(crate) fn finish_event(&mut self) {
        let Some(mut e) = self.event.take() else { return };
        if let Some(m) = e.saved_map.take() {
            if let Some(loc) = self.locations.get_mut(&e.location) {
                loc.map = m;
            }
        }
        if e.festival.is_some() {
            self.end_festival_day();
            self.emit(WorldEvent::EventEnded);
            return;
        }
        if !self.farmer.events_seen.contains(&e.id) {
            self.farmer.events_seen.push(e.id.clone());
        }
        let farmer_actor = e.actors.iter().find(|a| a.name == "farmer").map(|a| (a.pos, a.facing));
        match e.end.clone() {
            EndAction::Return => {
                let (loc, pos, facing) = e.start.clone();
                if loc == self.current {
                    self.farmer.pos = pos;
                    self.farmer.facing = facing;
                } else if let Some((pos, facing)) = farmer_actor {
                    self.farmer.pos = pos;
                    self.farmer.facing = facing;
                }
            }
            EndAction::Stay => {
                if let Some((pos, facing)) = farmer_actor {
                    self.farmer.pos = pos;
                    self.farmer.facing = facing;
                }
            }
            EndAction::Position(t) => self.farmer.set_tile(t),
            EndAction::WarpOut => {
                let exit = self.location().warps.first().map(|w| (w.target.clone(), w.to));
                if let Some((loc, to)) = exit {
                    self.warp_farmer(&loc, to, None);
                }
            }
            EndAction::Bed => {
                let wake = self.wake_tile();
                self.warp_farmer("FarmHouse", wake, Some(Dir::Down));
            }
            EndAction::NewDay => {
                self.emit(WorldEvent::EventEnded);
                self.end_day(false);
                return;
            }
        }
        self.emit(WorldEvent::EventEnded);
    }

    /// Advance the running event by one tick.
    pub(crate) fn update_event(&mut self, dt: u32) {
        let Some(mut e) = self.event.take() else { return };
        // Move and animate actors.
        let mut arrived = Vec::new();
        for a in e.actors.iter_mut() {
            if a.moving.is_none() {
                if let Some(next) = a.path.pop_front() {
                    if a.path_loop {
                        a.path.push_back(next);
                    }
                    a.facing = Dir::toward(next.x as i32, next.y as i32);
                    a.moving = Some(next * TILE_F);
                }
            }
            if let Some(off) = a.moving {
                let step = a.speed.max(0.5);
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
            if let Some(anim) = a.anim.as_mut() {
                anim.elapsed += dt;
            }
            if a.z > 0.0 || a.vz != 0.0 {
                a.z += a.vz;
                a.vz -= 0.5;
                if a.z <= 0.0 {
                    a.z = 0.0;
                    a.vz = 0.0;
                }
            }
            a.shake_ms = a.shake_ms.saturating_sub(dt);
            if let Some((_, ms)) = a.speech.as_mut() {
                *ms = ms.saturating_sub(dt);
            }
            if a.speech.as_ref().is_some_and(|s| s.1 == 0) {
                a.speech = None;
            }
        }
        for p in e.props.iter_mut() {
            if let EventProp::Sprite { elapsed, alpha, alpha_fade, rotation, rotation_change, .. } = p {
                *elapsed += dt;
                *alpha = (*alpha - *alpha_fade * dt as f32 / 16.0).max(0.0);
                *rotation += *rotation_change * dt as f32 / 16.0;
            }
        }
        e.props.retain(|p| match p {
            EventProp::Sprite { elapsed, interval_ms, frames, loops, alpha, .. } => {
                let total = (*interval_ms).max(1) * (*frames).max(1) as u32;
                *alpha > 0.0 && (*loops < 0 || *elapsed < total * (*loops as u32 + 1))
            }
            _ => true,
        });
        if let Some((v, ms)) = e.viewport_pan {
            let step = dt.min(ms);
            e.viewport += v * step as f32;
            e.viewport_pan = if ms > dt { Some((v, ms - dt)) } else { None };
        }
        if let Some(w) = &e.waiting_move {
            if arrived.contains(w) || !e.actors.iter().any(|a| &a.name == w && a.moving.is_some()) {
                e.waiting_move = None;
            }
        }
        if e.waiting_still && !e.actors.iter().any(|a| a.moving.is_some() || !a.path.is_empty() && !a.path_loop) {
            e.waiting_still = false;
        }
        if e.wait_ms > 0 {
            e.wait_ms = e.wait_ms.saturating_sub(dt);
        }
        // Run commands until one blocks.
        let mut guard = 0;
        while !e.waiting_ui && e.waiting_move.is_none() && !e.waiting_still && e.wait_ms == 0 && e.pc < e.commands.len() && guard < 64 {
            guard += 1;
            let cmd = e.commands[e.pc].clone();
            e.pc += 1;
            if self.run_event_command(&mut e, &cmd) {
                break;
            }
        }
        let done = e.finished || (e.pc >= e.commands.len() && !e.waiting_ui && e.waiting_move.is_none() && !e.waiting_still && e.wait_ms == 0);
        if done {
            if let Some(f) = e.festival.as_mut() {
                if !f.main_started && !f.roaming {
                    // Set-up finished: let the farmer walk around.
                    f.roaming = true;
                    e.finished = false;
                    self.event = Some(e);
                    return;
                }
            }
            self.event = Some(e);
            self.finish_event();
            return;
        }
        self.event = Some(e);
    }

    fn subs(&self) -> Substitutions<'_> {
        Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male }
    }

    /// Script of another event in this location (`fork`, `switchEvent`).
    fn event_script(&self, location: &str, id: &str) -> Option<String> {
        let events = self.event_data.get(location)?;
        events.get(id).or_else(|| events.iter().find(|(k, _)| k.split('/').next() == Some(id)).map(|(_, v)| v)).cloned()
    }

    fn switch_to_script(&self, e: &mut EventRunner, script: &str) {
        let cmds = split_commands(script);
        // Full events (music/viewport/actors header) skip their header.
        let looks_full = cmds.len() >= 3 && cmds[1].split_whitespace().count() == 2 && cmds[1].split_whitespace().all(|v| v.parse::<i32>().is_ok());
        e.commands = if looks_full { cmds[3..].to_vec() } else { cmds };
        e.pc = 0;
    }

    /// Execute one command; returns true to stop processing this tick.
    fn run_event_command(&mut self, e: &mut EventRunner, cmd: &str) -> bool {
        let t = tokens(cmd);
        let Some(name) = t.first().map(|s| s.as_str()) else { return false };
        let arg = |i: usize| t.get(i).cloned().unwrap_or_default();
        let num = |i: usize| t.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
        let numf = |i: usize| t.get(i).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        match name {
            "pause" => {
                e.wait_ms = num(1).max(0) as u32;
                true
            }
            "move" => {
                // move <actor> <dx> <dy> <facing> [continue] (repeatable)
                let mut i = 1;
                let mut last = None;
                while t.get(i + 3).is_some() {
                    let actor = arg(i);
                    let (dx, dy) = (num(i + 1), num(i + 2));
                    let facing = Dir::from_index(num(i + 3));
                    if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                        a.moving = Some(Vec2::new(dx as f32 * TILE_F, dy as f32 * TILE_F));
                        a.anim = None;
                        a.frame = None;
                        a.facing = if dx != 0 || dy != 0 { Dir::toward(dx, dy) } else { facing };
                        last = Some((actor, facing));
                    }
                    i += 4;
                    if t.get(i).is_some_and(|s| s == "true" || s == "false") {
                        if t[i] == "true" {
                            if let Some((actor, facing)) = last.take() {
                                e.commands.insert(e.pc, format!("faceDirection {actor} {} true", facing.index()));
                            }
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
            "advancedMove" => {
                // advancedMove <actor> <loop> <dx dy | facing ms>...
                let actor = arg(1);
                let looping = truthy(&arg(2));
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.path.clear();
                    a.path_loop = looping;
                    let mut i = 3;
                    while t.get(i + 1).is_some() {
                        let (x, y) = (num(i), num(i + 1));
                        // Pairs with one zero are moves; "facing ms" pauses are skipped.
                        if x == 0 || y == 0 {
                            a.path.push_back(Vec2::new(x as f32, y as f32));
                        }
                        i += 2;
                    }
                }
                false
            }
            "stopAdvancedMoves" => {
                for a in e.actors.iter_mut() {
                    a.path.clear();
                }
                false
            }
            "faceDirection" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.facing = Dir::from_index(num(2));
                    a.frame = None;
                }
                false
            }
            "warp" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.pos = tile_pos(num(2), num(3));
                    a.moving = None;
                }
                false
            }
            "speed" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.speed = numf(2).max(0.5) * 1.5;
                }
                false
            }
            "halt" => {
                for a in e.actors.iter_mut() {
                    a.moving = None;
                    a.path.clear();
                }
                e.waiting_move = None;
                false
            }
            "waitForAllStationary" => {
                e.waiting_still = true;
                true
            }
            "showFrame" => {
                // showFrame [actor] <frame> (no actor = farmer)
                let (actor, frame) = if t.len() >= 3 { (arg(1), num(2)) } else { ("farmer".to_string(), num(1)) };
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.frame = Some(frame);
                    a.anim = None;
                }
                false
            }
            "animate" => {
                // animate <actor> <flip> <loop> <frameMs> <frames...>
                let actor = arg(1);
                let frames: Vec<i32> = t.iter().skip(5).filter_map(|f| f.parse().ok()).collect();
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.anim = Some(ActorAnim { frames, frame_ms: num(4).max(1) as u32, looping: truthy(&arg(3)), flip: truthy(&arg(2)), elapsed: 0 });
                    a.flip = truthy(&arg(2));
                }
                false
            }
            "stopAnimation" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.anim = None;
                    a.flip = false;
                    a.frame = t.get(2).and_then(|v| v.parse().ok());
                }
                false
            }
            "positionOffset" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.pos += Vec2::new(numf(2), numf(3));
                }
                false
            }
            "jump" => {
                let actor = arg(1);
                let intensity = if t.len() > 2 { numf(2) } else { 8.0 };
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.vz = intensity;
                    a.z = 0.01;
                }
                false
            }
            "shake" => {
                let actor = arg(1);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.shake_ms = num(2).max(0) as u32;
                }
                false
            }
            "textAboveHead" => {
                let actor = arg(1);
                let text = dialogue::substitute(&arg(2), &self.subs());
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.speech = Some((text, 3000));
                }
                false
            }
            "addTemporaryActor" => {
                // addTemporaryActor <sprite> <w> <h> <x> <y> <dir> [breather] [Character|Animal|Monster]
                let sprite = arg(1);
                let kind = arg(8);
                let texture = match kind.as_str() {
                    "Animal" => format!("Animals/{sprite}"),
                    "Monster" => format!("Characters/Monsters/{sprite}"),
                    _ => format!("Characters/{sprite}"),
                };
                let mut a = Actor::new(&sprite.replace(' ', "_"), tile_pos(num(4), num(5)), Dir::from_index(num(6)), texture);
                a.size = (num(2).max(1), num(3).max(1));
                e.actors.retain(|x| x.name != a.name);
                e.actors.push(a);
                false
            }
            "removeTemporaryActor" => {
                let actor = arg(1);
                e.actors.retain(|a| a.name != actor);
                false
            }
            "changeSprite" => {
                let actor = arg(1);
                let suffix = arg(2);
                if let Some(a) = e.actors.iter_mut().find(|a| a.name == actor) {
                    a.texture = if suffix.is_empty() || suffix == "none" { format!("Characters/{actor}") } else { format!("Characters/{actor}_{suffix}") };
                }
                false
            }
            "speak" => {
                let who = arg(1);
                let (text, _) = self.prepare_dialogue(&arg(2));
                let pages = dialogue::parse(&text, &self.subs());
                self.emit(WorldEvent::Dialogue { npc: Some(who), pages });
                e.waiting_ui = true;
                true
            }
            "splitSpeak" => {
                let who = arg(1);
                let parts: Vec<&str> = t.get(2).map(|s| s.split('~').collect()).unwrap_or_default();
                let text = parts.get(e.last_answer.unwrap_or(0)).or(parts.first()).copied().unwrap_or("");
                let pages = dialogue::parse(text, &self.subs());
                self.emit(WorldEvent::Dialogue { npc: Some(who), pages });
                e.waiting_ui = true;
                true
            }
            "message" => {
                let text = dialogue::substitute(&arg(1), &self.subs()).replace('^', "\n");
                self.emit(WorldEvent::Message(text));
                e.waiting_ui = true;
                true
            }
            "question" => {
                // question <forkN|null> "<question>#<answer>#<answer>..."
                let fork = arg(1).strip_prefix("fork").and_then(|n| n.parse::<usize>().ok());
                let raw = dialogue::substitute(&arg(2), &self.subs());
                let mut parts: Vec<String> = raw.split('#').map(|s| s.to_string()).collect();
                let text = if parts.is_empty() { String::new() } else { parts.remove(0) };
                e.fork_answer = fork;
                e.fork_ok = false;
                e.quick_branches = None;
                self.emit(WorldEvent::EventQuestion { text, options: parts });
                e.waiting_ui = true;
                e.awaiting_answer = true;
                true
            }
            "quickQuestion" => {
                // quickQuestion <question>#<a>#<b>(break)<cmds a>(break)<cmds b>
                let rest = cmd.trim_start_matches("quickQuestion").trim();
                let mut sections: Vec<String> = rest.split("(break)").map(|s| s.to_string()).collect();
                let head = if sections.is_empty() { String::new() } else { sections.remove(0) };
                let head = dialogue::substitute(head.trim().trim_matches('"'), &self.subs());
                let mut parts: Vec<String> = head.split('#').map(|s| s.to_string()).collect();
                let text = if parts.is_empty() { String::new() } else { parts.remove(0) };
                e.quick_branches = Some(sections);
                e.fork_answer = None;
                self.emit(WorldEvent::EventQuestion { text, options: parts });
                e.waiting_ui = true;
                e.awaiting_answer = true;
                true
            }
            "fork" => {
                // fork [req] <event id>
                let (req, id) = if t.len() >= 3 { (Some(arg(1)), arg(2)) } else { (None, arg(1)) };
                let go = match &req {
                    Some(r) => self.farmer.mail_received.iter().any(|m| m == r) || self.farmer.events_seen.iter().any(|m| m == r) || self.farmer.dialogue_answers.iter().any(|m| m == r),
                    None => e.fork_ok,
                };
                if go {
                    if let Some(script) = self.event_script(&e.location, &id) {
                        e.fork_ok = false;
                        self.switch_to_script(e, &script);
                    }
                }
                false
            }
            "switchEvent" => {
                if let Some(script) = self.event_script(&e.location, &arg(1)) {
                    self.switch_to_script(e, &script);
                }
                false
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
                    // viewport move <dx> <dy> <ms>
                    let ms = num(4).max(0) as u32;
                    if ms == 0 {
                        e.viewport += Vec2::new(num(2) as f32, num(3) as f32);
                    } else {
                        let total = Vec2::new(num(2) as f32, num(3) as f32);
                        e.viewport_pan = Some((total * (1.0 / ms as f32), ms));
                    }
                } else if t.len() >= 3 {
                    if let (Ok(x), Ok(y)) = (arg(1).parse::<i32>(), arg(2).parse::<i32>()) {
                        e.viewport = Vec2::new(x as f32, y as f32);
                    } else if let Some(a) = e.actors.iter().find(|a| a.name == arg(1)) {
                        e.viewport = Vec2::new(a.pos.x / TILE_F, (a.pos.y + 16.0) / TILE_F);
                    }
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
            "stopMusic" => {
                self.events.push(WorldEvent::Music("none".into()));
                false
            }
            "action" => {
                // The rest of the command, quotes kept for the action's own arguments.
                let a = cmd.trim_start().strip_prefix("action").unwrap_or("").trim().to_string();
                if let Err(e) = self.run_game_action(&a) {
                    eprintln!("event action: {e}");
                }
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
            "removeItem" => {
                let raw = arg(1);
                let (_, id) = crate::item::split_qualified(&raw);
                let n = if t.len() > 2 { num(2).max(1) } else { 1 };
                self.farmer.inventory.remove_object(id, n);
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
                self.emit(WorldEvent::Fade { to_black: name == "globalFade" || name == "fade" && arg(1) != "unfade" });
                e.wait_ms = 500;
                true
            }
            "screenFlash" => {
                self.emit(WorldEvent::Flash);
                false
            }
            "ambientLight" => {
                e.ambient = if t.len() >= 4 { Some(Color::rgb(num(1).clamp(0, 255) as u8, num(2).clamp(0, 255) as u8, num(3).clamp(0, 255) as u8)) } else { None };
                false
            }
            "end" => {
                e.finished = true;
                e.end = match arg(1).as_str() {
                    "position" => EndAction::Position(TilePos::new(num(2), num(3))),
                    "warpOut" | "dialogueWarpOut" | "invisibleWarpOut" => EndAction::WarpOut,
                    "bed" | "beginGame" => EndAction::Bed,
                    "newDay" => EndAction::NewDay,
                    "" => EndAction::Return,
                    _ => EndAction::Stay,
                };
                if matches!(arg(1).as_str(), "dialogue" | "dialogueWarpOut") {
                    let (npc, text) = (arg(2), arg(3));
                    if let Some(n) = self.npcs.iter_mut().find(|n| n.name == npc) {
                        n.dialogue.insert("__next".into(), text);
                    }
                }
                true
            }
            "addQuest" => {
                self.add_quest(&arg(1));
                false
            }
            "removeQuest" => {
                let id = arg(1);
                self.farmer.quests.retain(|q| q.id != id);
                false
            }
            "addCookingRecipe" => {
                self.farmer.cooking_recipes.entry(t[1..].join(" ")).or_insert(0);
                false
            }
            "addCraftingRecipe" => {
                self.farmer.crafting_recipes.entry(t[1..].join(" ")).or_insert(0);
                false
            }
            "addConversationTopic" => {
                let days = if t.len() > 2 { num(2) } else { 4 };
                self.farmer.conversation_topics.insert(arg(1), days);
                false
            }
            "eventSeen" => {
                let id = arg(1);
                let seen = t.get(2).map(|v| truthy(v)).unwrap_or(true);
                self.farmer.events_seen.retain(|x| *x != id);
                if seen {
                    self.farmer.events_seen.push(id);
                }
                false
            }
            "awardFestivalPrize" => {
                if t.len() > 1 {
                    if let Some(item) = self.items.create(&arg(1), 1) {
                        self.give_item(item);
                    }
                }
                false
            }
            "addObject" | "addBigProp" => {
                let id = arg(3);
                e.props.push(EventProp::Object { tile: TilePos::new(num(1), num(2)), id: crate::item::split_qualified(&id).1.to_string(), big: name == "addBigProp" });
                false
            }
            "removeObject" => {
                let p = TilePos::new(num(1), num(2));
                e.props.retain(|x| !matches!(x, EventProp::Object { tile, .. } if *tile == p));
                false
            }
            "removeTemporarySprites" => {
                e.props.retain(|x| !matches!(x, EventProp::Sprite { .. }));
                false
            }
            "removeSprite" => {
                let p = Vec2::new(num(1) as f32 * TILE_F, num(2) as f32 * TILE_F);
                e.props.retain(|x| !matches!(x, EventProp::Sprite { pos, .. } if pos.distance(p) < TILE_F));
                false
            }
            "temporaryAnimatedSprite" => {
                // <texture> <x> <y> <w> <h> <interval> <frames> <loops> <tileX> <tileY> <flicker> <flip> <depth> <alphaFade> <scale> <scaleChange> <rotation> <rotationChange>
                if t.len() >= 11 {
                    e.props.push(EventProp::Sprite {
                        texture: arg(1).replace('\\', "/"),
                        src: (num(2), num(3), num(4), num(5)),
                        pos: Vec2::new(numf(9) * TILE_F, numf(10) * TILE_F),
                        interval_ms: numf(6).max(1.0) as u32,
                        frames: num(7).max(1),
                        loops: num(8),
                        flip: truthy(&arg(12)),
                        scale: if t.len() > 15 { numf(15).max(0.1) } else { 4.0 },
                        alpha: 1.0,
                        alpha_fade: if t.len() > 14 { numf(14).max(0.0) } else { 0.0 },
                        rotation: if t.len() > 17 { numf(17) } else { 0.0 },
                        rotation_change: if t.len() > 18 { numf(18) } else { 0.0 },
                        elapsed: 0,
                    });
                }
                false
            }
            "changeMapTile" => {
                // changeMapTile <layer> <x> <y> <tile index>
                self.edit_event_tile(e, &arg(1), num(2), num(3), Some(num(4)));
                false
            }
            "removeTile" => {
                // removeTile <x> <y> <layer>
                self.edit_event_tile(e, &arg(3), num(1), num(2), None);
                false
            }
            "doAction" => {
                let p = TilePos::new(num(1), num(2));
                if let Some(action) = self.locations.get(&e.location).and_then(|l| l.action_at(p)) {
                    self.run_action_pub(&action);
                }
                false
            }
            "changeToTemporaryMap" | "changeLocation" => {
                if let Some(m) = self.festival_maps.get(&arg(1)).cloned() {
                    if let Some(loc) = self.locations.get_mut(&e.location) {
                        let old = std::mem::replace(&mut loc.map, m);
                        if e.saved_map.is_none() {
                            e.saved_map = Some(old);
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Change one map tile for the rest of the event (restored at the end).
    fn edit_event_tile(&mut self, e: &mut EventRunner, layer: &str, x: i32, y: i32, index: Option<i32>) {
        let Some(loc) = self.locations.get_mut(&e.location) else { return };
        if e.saved_map.is_none() {
            e.saved_map = Some(loc.map.clone());
        }
        let Some(l) = loc.map.layer_mut(layer) else { return };
        match index {
            None => l.set(x, y, None),
            Some(i) => {
                // Keep the tile's sheet when there is one; else the first sheet.
                let sheet = l.get(x, y).map(|t| t.sheet).unwrap_or(0);
                l.set(x, y, Some(valley_content::map::Tile::new(sheet, i)));
            }
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

    #[test]
    fn looping_animation_cycles_frames() {
        let a = ActorAnim { frames: vec![4, 5, 6], frame_ms: 100, looping: true, flip: false, elapsed: 350 };
        assert_eq!(a.current(), 4);
        let b = ActorAnim { frames: vec![4, 5, 6], frame_ms: 100, looping: false, flip: false, elapsed: 950 };
        assert_eq!(b.current(), 6);
    }
}
