//! Fishing: charge-and-cast, waiting for a bite, and the catch minigame.
//!
//! Fish definitions come from `Data/Fish` (slash-delimited, public format:
//! name/difficulty/behavior/minSize/maxSize/times/seasons/weather/...) and
//! per-location lists in `Data/Locations` ("Fish" entries). Minigame tuning
//! values are approximations of the original feel.

use engine::{Rng, Vec2};
use serde::{Deserialize, Serialize};

use crate::farmer::Skill;
use crate::gsq::{self, QueryCtx};
use crate::item::{Item, Quality};
use crate::world::{World, WorldEvent};
use crate::{TilePos, TILE_F};

/// Height of the minigame water column in "bar units".
pub const TRACK: f32 = 568.0;

#[derive(Clone, Debug, PartialEq)]
pub struct FishInfo {
    pub id: String,
    pub difficulty: i32,
    pub behavior: Behavior,
    pub min_size: i32,
    pub max_size: i32,
    /// (start, end) time windows.
    pub times: Vec<(i32, i32)>,
    pub seasons: Vec<String>,
    /// "sunny", "rainy" or "both".
    pub weather: String,
    pub min_level: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Behavior {
    Mixed,
    Dart,
    Smooth,
    Sinker,
    Floater,
}

pub fn parse_fish(id: &str, s: &str) -> Option<FishInfo> {
    let f: Vec<&str> = s.split('/').collect();
    if f.get(1) == Some(&"trap") {
        return None;
    }
    let difficulty = f.get(1)?.parse().ok()?;
    let behavior = match f.get(2)?.to_ascii_lowercase().as_str() {
        "dart" => Behavior::Dart,
        "smooth" => Behavior::Smooth,
        "sinker" => Behavior::Sinker,
        "floater" => Behavior::Floater,
        _ => Behavior::Mixed,
    };
    let min_size = f.get(3).and_then(|v| v.parse().ok()).unwrap_or(1);
    let max_size = f.get(4).and_then(|v| v.parse().ok()).unwrap_or(min_size + 1);
    let tv: Vec<i32> = f.get(5).map(|t| t.split_whitespace().filter_map(|x| x.parse().ok()).collect()).unwrap_or_default();
    let times = tv.chunks(2).filter(|c| c.len() == 2).map(|c| (c[0], c[1])).collect();
    let seasons = f.get(6).map(|t| t.split_whitespace().map(|s| s.to_ascii_lowercase()).collect()).unwrap_or_default();
    let weather = f.get(7).unwrap_or(&"both").to_ascii_lowercase();
    let min_level = f.get(12).and_then(|v| v.parse().ok()).unwrap_or(0);
    Some(FishInfo { id: id.to_string(), difficulty, behavior, min_size, max_size, times, seasons, weather, min_level })
}

/// The catch minigame state.
#[derive(Clone, Debug, PartialEq)]
pub struct Minigame {
    pub fish: FishInfo,
    /// Fish position (0 = top, TRACK = bottom) and motion target.
    pub fish_pos: f32,
    pub fish_target: f32,
    pub fish_speed: f32,
    /// Player bar top position, height and velocity.
    pub bar_pos: f32,
    pub bar_height: f32,
    pub bar_vel: f32,
    /// Catch progress 0..1.
    pub progress: f32,
    pub perfect: bool,
    pub elapsed_ms: u32,
}

impl Minigame {
    pub fn new(fish: FishInfo, fishing_level: i32) -> Self {
        let bar_height = 96.0 + fishing_level as f32 * 8.0;
        Self {
            fish,
            fish_pos: TRACK - 100.0,
            fish_target: TRACK / 2.0,
            fish_speed: 0.0,
            bar_pos: TRACK - bar_height,
            bar_height,
            bar_vel: 0.0,
            progress: 0.3,
            perfect: true,
            elapsed_ms: 0,
        }
    }

    pub fn in_bar(&self) -> bool {
        self.fish_pos + 16.0 >= self.bar_pos && self.fish_pos + 16.0 <= self.bar_pos + self.bar_height
    }

    /// Advance one tick. Returns Some(true) when caught, Some(false) when lost.
    pub fn step(&mut self, holding: bool, rng: &mut Rng, dt: u32) -> Option<bool> {
        self.elapsed_ms += dt;
        let diff = self.fish.difficulty as f32;
        // Fish picks new targets more often at higher difficulty.
        let retarget = (diff / 4000.0).max(0.002);
        if rng.chance(retarget as f64) || (self.fish_target - self.fish_pos).abs() < 4.0 {
            let span = (diff / 100.0 * TRACK).clamp(40.0, TRACK);
            let mut t = self.fish_pos + rng.range(-(span as i32), span as i32 + 1) as f32;
            match self.fish.behavior {
                Behavior::Sinker => t += 40.0,
                Behavior::Floater => t -= 40.0,
                _ => {}
            }
            self.fish_target = t.clamp(0.0, TRACK - 32.0);
        }
        let accel = match self.fish.behavior {
            Behavior::Smooth => 0.03,
            Behavior::Dart => 0.12,
            _ => 0.06,
        } * (0.5 + diff / 100.0);
        let want = (self.fish_target - self.fish_pos).signum() * (diff / 15.0 + 1.0);
        self.fish_speed += (want - self.fish_speed) * accel;
        if self.fish.behavior == Behavior::Dart && rng.chance(0.01) {
            self.fish_speed *= 3.0;
        }
        self.fish_pos = (self.fish_pos + self.fish_speed).clamp(0.0, TRACK - 32.0);

        // Player bar: lift while holding, gravity otherwise; bounce off bottom.
        self.bar_vel += if holding { -0.25 } else { 0.25 };
        self.bar_vel = self.bar_vel.clamp(-8.0, 8.0);
        self.bar_pos += self.bar_vel;
        if self.bar_pos < 0.0 {
            self.bar_pos = 0.0;
            self.bar_vel = 0.0;
        }
        let bottom = TRACK - self.bar_height;
        if self.bar_pos > bottom {
            self.bar_pos = bottom;
            self.bar_vel = -self.bar_vel * 0.4;
            if self.bar_vel.abs() < 1.0 {
                self.bar_vel = 0.0;
            }
        }

        if self.in_bar() {
            self.progress += 0.002;
        } else {
            self.progress -= 0.003;
            if self.elapsed_ms > 300 {
                self.perfect = false;
            }
        }
        if self.progress >= 1.0 {
            Some(true)
        } else if self.progress <= 0.0 {
            Some(false)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub enum FishingState {
    #[default]
    Idle,
    /// Holding the button: power oscillates 0..1.
    Charging { power: f32, rising: bool },
    /// Bobber flying to the water.
    Casting { bobber: Vec2, target: TilePos, ms: u32 },
    /// Waiting for a bite.
    Waiting { bobber: TilePos, bite_at_ms: u32, ms: u32 },
    /// A fish is biting; press within the window.
    Nibble { bobber: TilePos, ms: u32 },
    Minigame { bobber: TilePos, game: Box<Minigame> },
    /// Showing the catch.
    Caught { item: Item, size: i32, perfect: bool, ms: u32 },
}

impl FishingState {
    pub fn active(&self) -> bool {
        !matches!(self, FishingState::Idle)
    }
}

/// Reaction window after a bite (approx.).
pub const NIBBLE_MS: u32 = 1000;

impl World {
    pub fn fish_db(&self) -> &std::collections::BTreeMap<String, FishInfo> {
        &self.fish
    }

    /// Choose what bites at `tile` in the current location right now.
    pub fn choose_fish(&self, rng: &mut Rng) -> Option<FishInfo> {
        let data = self.location_data.get(&self.current)?;
        let level = self.farmer.skills.level(Skill::Fishing);
        let raining = self.weather.is_raining();
        let season = self.clock.season.key();
        let mail = self.farmer.mail_received.clone();
        let mut entries = data.fish.clone();
        // Shuffle so each entry gets a fair chance roll.
        for i in (1..entries.len()).rev() {
            let j = rng.range(0, i as i32 + 1) as usize;
            entries.swap(i, j);
        }
        for e in entries {
            let Some(id) = e.item_id.as_deref() else { continue };
            let (_, id) = crate::item::split_qualified(id);
            if let Some(s) = &e.season {
                if !s.eq_ignore_ascii_case(season) {
                    continue;
                }
            }
            let mut rng2 = rng.clone();
            let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &[], location: &self.current, rng: &mut rng2 };
            if !gsq::check(e.condition.as_deref(), &mut ctx) {
                continue;
            }
            let Some(info) = self.fish.get(id) else {
                // Non-fish catches (trash etc.) are handled as fixed-difficulty.
                if self.items.objects.contains_key(id) && rng.chance(e.chance.unwrap_or(1.0)) {
                    return Some(FishInfo { id: id.into(), difficulty: 0, behavior: Behavior::Floater, min_size: 0, max_size: 0, times: vec![], seasons: vec![], weather: "both".into(), min_level: 0 });
                }
                continue;
            };
            if level < info.min_level {
                continue;
            }
            if !info.seasons.is_empty() && !info.seasons.iter().any(|s| s == season) {
                continue;
            }
            if (info.weather == "sunny" && raining) || (info.weather == "rainy" && !raining) {
                continue;
            }
            if !info.times.is_empty() && !info.times.iter().any(|(a, b)| self.clock.time >= *a && self.clock.time < *b) {
                continue;
            }
            if rng.chance(e.chance.unwrap_or(1.0)) {
                return Some(info.clone());
            }
        }
        None
    }

    pub(crate) fn start_fishing(&mut self) {
        self.fishing = FishingState::Charging { power: 0.0, rising: true };
    }

    /// Advance fishing by one tick. `use_pressed` / `use_held` come from input.
    pub(crate) fn update_fishing(&mut self, use_pressed: bool, use_held: bool, dt: u32) {
        let state = std::mem::take(&mut self.fishing);
        self.fishing = match state {
            FishingState::Idle => FishingState::Idle,
            FishingState::Charging { mut power, mut rising } => {
                if use_held {
                    power += if rising { 0.02 } else { -0.02 };
                    if power >= 1.0 {
                        power = 1.0;
                        rising = false;
                    } else if power <= 0.0 {
                        power = 0.0;
                        rising = true;
                    }
                    FishingState::Charging { power, rising }
                } else {
                    // Released: cast 1..=6 tiles ahead.
                    let level = self.farmer.skills.level(Skill::Fishing);
                    self.farmer.spend_stamina(crate::tools::stamina_cost(crate::item::ToolKind::FishingRod, level));
                    let dist = 1 + (power * 5.0).round() as i32;
                    let (dx, dy) = self.farmer.facing.delta();
                    let me = self.farmer.tile();
                    let target = TilePos::new(me.x + dx * dist, me.y + dy * dist);
                    self.sound("fishingRodBend");
                    FishingState::Casting { bobber: self.farmer.center(), target, ms: 0 }
                }
            }
            FishingState::Casting { bobber, target, ms } => {
                let ms = ms + dt;
                let to = target.world_center();
                let t = (ms as f32 / 500.0).min(1.0);
                let pos = bobber + (to - bobber) * t;
                if t >= 1.0 {
                    if self.location().is_water(target) {
                        self.sound("waterSlosh");
                        let mut rng = self.fishing_rng();
                        let bite = rng.range(1500, 12000) as u32;
                        FishingState::Waiting { bobber: target, bite_at_ms: bite, ms: 0 }
                    } else {
                        self.hud("The bobber needs to land in water.");
                        FishingState::Idle
                    }
                } else {
                    let _ = pos;
                    FishingState::Casting { bobber, target, ms }
                }
            }
            FishingState::Waiting { bobber, bite_at_ms, ms } => {
                if use_pressed {
                    self.sound("fishingRodBend");
                    FishingState::Idle
                } else if ms + dt >= bite_at_ms {
                    self.sound("fishBite");
                    FishingState::Nibble { bobber, ms: 0 }
                } else {
                    FishingState::Waiting { bobber, bite_at_ms, ms: ms + dt }
                }
            }
            FishingState::Nibble { bobber, ms } => {
                if use_pressed {
                    let mut rng = self.fishing_rng();
                    match self.choose_fish(&mut rng) {
                        Some(f) if f.difficulty > 0 => {
                            let level = self.farmer.skills.level(Skill::Fishing);
                            self.sound("FishHit");
                            FishingState::Minigame { bobber, game: Box::new(Minigame::new(f, level)) }
                        }
                        Some(f) => {
                            // Trash and other non-fish: no minigame.
                            FishingState::Caught { item: Item::object(&f.id, 1), size: 0, perfect: false, ms: 0 }
                        }
                        None => {
                            self.hud("Nothing's biting here.");
                            FishingState::Idle
                        }
                    }
                } else if ms + dt > NIBBLE_MS {
                    self.hud("The fish got away...");
                    FishingState::Idle
                } else {
                    FishingState::Nibble { bobber, ms: ms + dt }
                }
            }
            FishingState::Minigame { bobber, mut game } => {
                let mut rng = self.fishing_rng();
                match game.step(use_held, &mut rng, dt) {
                    Some(true) => {
                        let f = &game.fish;
                        let size_t = rng.next_f64() as f32;
                        let size = f.min_size + ((f.max_size - f.min_size) as f32 * size_t) as i32;
                        let mut q = if size_t < 0.33 { 0 } else if size_t < 0.66 { 1 } else { 2 };
                        if game.perfect {
                            q += 1;
                        }
                        let quality = Quality::from_level(if q >= 3 { 4 } else { q });
                        let xp = ((q + 1) * 3 + f.difficulty / 3) as f32 * if game.perfect { 2.4 } else { 1.0 };
                        if let Some(l) = self.farmer.skills.gain(Skill::Fishing, xp as i32) {
                            self.emit(WorldEvent::LevelUp(Skill::Fishing, l));
                        }
                        self.sound("jingle1");
                        FishingState::Caught { item: Item::object_q(&f.id, 1, quality), size, perfect: game.perfect, ms: 0 }
                    }
                    Some(false) => {
                        self.sound("fishEscape");
                        self.hud("The fish escaped.");
                        FishingState::Idle
                    }
                    None => FishingState::Minigame { bobber, game },
                }
            }
            FishingState::Caught { item, size, perfect, ms } => {
                let ms = ms + dt;
                if ms > 600 && use_pressed || ms > 4000 {
                    self.give_item(item);
                    FishingState::Idle
                } else {
                    FishingState::Caught { item, size, perfect, ms }
                }
            }
        };
    }

    fn fishing_rng(&mut self) -> Rng {
        let s = self.rng.next_u32();
        Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, s as i64]))
    }

    /// World position of the bobber, if one is out.
    pub fn bobber_pos(&self) -> Option<Vec2> {
        match &self.fishing {
            FishingState::Casting { bobber, target, ms } => {
                let to = target.world_center();
                let t = (*ms as f32 / 500.0).min(1.0);
                let arc = (t * std::f32::consts::PI).sin() * TILE_F * 2.0;
                let p = *bobber + (to - *bobber) * t;
                Some(Vec2::new(p.x, p.y - arc))
            }
            FishingState::Waiting { bobber, .. } | FishingState::Nibble { bobber, .. } | FishingState::Minigame { bobber, .. } => Some(bobber.world_center()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fish_strings() {
        let f = parse_fish("X", "Test Fish/40/dart/5/20/600 1200 1800 2400/spring fall/rainy/690 .4 685 .1/3/.3/.4/2/false").unwrap();
        assert_eq!(f.difficulty, 40);
        assert_eq!(f.behavior, Behavior::Dart);
        assert_eq!(f.times, vec![(600, 1200), (1800, 2400)]);
        assert_eq!(f.seasons, vec!["spring", "fall"]);
        assert_eq!(f.weather, "rainy");
        assert_eq!(f.min_level, 2);
        assert!(parse_fish("Y", "Crab/trap/.1/684 .45/ocean/1/20/false").is_none());
    }

    #[test]
    fn perfect_play_catches_and_idle_loses() {
        let fish = parse_fish("X", "F/30/mixed/1/10/600 2600/spring/both/x/1/1/1/0/false").unwrap();
        let mut rng = Rng::new(9);
        // A "player" that tracks the fish catches it.
        let mut g = Minigame::new(fish.clone(), 0);
        let mut result = None;
        for _ in 0..20000 {
            let fish_center = g.fish_pos + 16.0;
            let bar_center = g.bar_pos + g.bar_height / 2.0;
            let hold = fish_center < bar_center - 4.0 || (fish_center < bar_center + 8.0 && g.bar_vel > 2.0);
            if let Some(r) = g.step(hold, &mut rng, 16) {
                result = Some(r);
                break;
            }
        }
        assert_eq!(result, Some(true));
        // Never pressing loses (the fish starts away from the resting bar).
        let mut g = Minigame::new(fish, 0);
        g.fish_target = 0.0;
        g.fish_pos = 0.0;
        let mut r = None;
        for _ in 0..20000 {
            if let Some(x) = g.step(false, &mut rng, 16) {
                r = Some(x);
                break;
            }
        }
        assert_eq!(r, Some(false));
    }
}
