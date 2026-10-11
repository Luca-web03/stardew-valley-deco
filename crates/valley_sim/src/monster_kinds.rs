//! Species behaviours layered on the basic walker/hopper/flyer movement.
//! `Data/Monsters` only holds stats, so the behaviour is picked from the
//! monster's name (keywords below). Timings and numbers are approximations.
//!
//! * Crabs ("crab") sit disguised as rocks until the farmer comes close,
//!   and their shell soaks up the first three hits.
//! * Burrowers ("duggy", "mole") travel underground (untouchable) and pop
//!   up next to the farmer for a moment.
//! * Shooters throw bones ("skeleton"), cast spells ("mage", "shaman") or
//!   spit fireballs ("squid", "lurk", "rex", "spitter") at the farmer.
//! * Shamans ("shaman") also heal monsters around them now and then.
//! * Mummies ("mummy") collapse instead of dying and get back up unless a
//!   bomb finishes them.
//! * Hot heads ("hot head", "bomber") light a fuse when hit and explode.
//! * Big slimes ("big slime", "big blob") split into small ones.
//! * Grubs ("grub", "larva") turn into flies after a while.
//! * Dust sprites ("dust") are extra quick.

use engine::{Rng, Vec2};

use crate::combat::Monster;
use crate::world::World;
use crate::TILE_F;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShotKind {
    Bone,
    Fireball,
    Spell,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Species {
    Normal,
    Crab,
    Burrower,
    Shooter(ShotKind),
    Shaman,
    Mummy,
    HotHead,
    BigSlime,
    Grub,
    Dust,
}

pub fn species_for(name: &str) -> Species {
    let n = name.to_ascii_lowercase();
    let has = |k: &str| n.contains(k);
    if has("crab") {
        Species::Crab
    } else if has("duggy") || has("mole") {
        Species::Burrower
    } else if has("shaman") {
        Species::Shaman
    } else if has("skeleton") && has("mage") || has("mage") {
        Species::Shooter(ShotKind::Spell)
    } else if has("skeleton") {
        Species::Shooter(ShotKind::Bone)
    } else if has("squid") || has("lurk") || has("rex") || has("spitter") {
        Species::Shooter(ShotKind::Fireball)
    } else if has("mummy") {
        Species::Mummy
    } else if has("hot head") || has("bomber") {
        Species::HotHead
    } else if has("big slime") || has("big blob") {
        Species::BigSlime
    } else if has("grub") || has("larva") {
        Species::Grub
    } else if has("dust") {
        Species::Dust
    } else {
        Species::Normal
    }
}

/// A monster's projectile flying at the farmer.
#[derive(Clone, Debug, PartialEq)]
pub struct EnemyShot {
    pub location: String,
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: i32,
    pub kind: ShotKind,
    pub ttl_ms: u32,
}

pub const SHELL_HITS: i32 = 3;
pub const MUMMY_DOWN_MS: u32 = 10_000;
pub const FUSE_MS: u32 = 1500;
pub const GRUB_MS: u32 = 12_000;

/// What the per-tick species step decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Use the normal movement this tick.
    Move,
    /// Stand still this tick (no movement).
    Hold,
}

/// Things a species wants done after the monster loop.
#[derive(Default)]
pub struct SpeciesEffects {
    pub shots: Vec<EnemyShot>,
    pub heals: Vec<(String, Vec2)>,
    pub explode: Vec<u32>,
    pub metamorph: Vec<u32>,
}

/// Per-tick species behaviour for one monster near `target` (the farmer).
pub fn species_step(m: &mut Monster, dt: u32, target: Vec2, rng: &mut Rng, fx: &mut SpeciesEffects) -> Step {
    m.age_ms += dt;
    let dist = m.center().distance(target);
    match m.species {
        Species::Crab if m.hidden => {
            if dist < TILE_F * 1.6 {
                m.hidden = false;
            }
            Step::Hold
        }
        Species::Burrower => {
            m.state_ms = m.state_ms.saturating_sub(dt);
            if m.state_ms == 0 {
                m.hidden = !m.hidden;
                m.state_ms = if m.hidden { rng.range(1500, 3000) as u32 } else { 1200 };
                if !m.hidden {
                    // Surface right beside the farmer.
                    let off = Vec2::new(rng.range(-1, 2) as f32, rng.range(-1, 2) as f32) * TILE_F;
                    m.pos = target - Vec2::new(32.0, 40.0) + off;
                }
            }
            if m.hidden { Step::Move } else { Step::Hold }
        }
        Species::Mummy if m.collapsed_ms > 0 => {
            m.collapsed_ms = m.collapsed_ms.saturating_sub(dt);
            if m.collapsed_ms == 0 {
                m.health = m.info.health;
            }
            Step::Hold
        }
        Species::HotHead if m.fuse_ms.is_some() => {
            let left = m.fuse_ms.unwrap_or(0).saturating_sub(dt);
            m.fuse_ms = Some(left);
            if left == 0 {
                fx.explode.push(m.id);
            }
            Step::Hold
        }
        Species::Shooter(_) | Species::Shaman if dist < TILE_F * 7.0 => {
            let kind = match m.species {
                Species::Shooter(k) => k,
                _ => ShotKind::Spell,
            };
            m.shot_cd = m.shot_cd.saturating_sub(dt);
            if m.shot_cd == 0 {
                let speed = match kind {
                    ShotKind::Bone => 5.0,
                    ShotKind::Fireball => 4.0,
                    ShotKind::Spell => 3.5,
                };
                let dir = (target - m.center()).normalized();
                fx.shots.push(EnemyShot { location: m.location.clone(), pos: m.center(), vel: dir * speed, damage: m.info.damage, kind, ttl_ms: 3000 });
                m.shot_cd = rng.range(2000, 3500) as u32;
            }
            if m.species == Species::Shaman {
                m.state_ms = m.state_ms.saturating_sub(dt);
                if m.state_ms == 0 {
                    fx.heals.push((m.location.clone(), m.center()));
                    m.state_ms = 6000;
                }
            }
            Step::Move
        }
        Species::Grub if m.age_ms >= GRUB_MS => {
            fx.metamorph.push(m.id);
            Step::Move
        }
        _ => Step::Move,
    }
}

impl World {
    /// Spawn a monster of `name` (or a fallback with `fallback_kw` in its
    /// name) at a world position.
    pub(crate) fn spawn_monster_at(&mut self, name: &str, fallback_kw: &str, location: &str, pos: Vec2) -> bool {
        let info = self.monster_info.get(name).cloned().or_else(|| self.monster_info.values().find(|m| m.name.to_ascii_lowercase().contains(fallback_kw)).cloned());
        let Some(info) = info else { return false };
        self.next_monster_id += 1;
        let mut m = Monster::new(self.next_monster_id, info, location, crate::TilePos::from_world(pos));
        m.pos = pos - Vec2::new(32.0, 40.0);
        m.hidden = false;
        self.monsters.push(m);
        true
    }

    /// After the monster loop: shots, heals, explosions, metamorphoses.
    pub(crate) fn apply_species_effects(&mut self, fx: SpeciesEffects) {
        self.enemy_shots.extend(fx.shots);
        for (loc, c) in fx.heals {
            for m in self.monsters.iter_mut().filter(|m| m.location == loc && m.center().distance(c) < TILE_F * 3.0 && m.collapsed_ms == 0) {
                m.health = (m.health + m.info.health / 4).min(m.info.health);
            }
        }
        for id in fx.explode {
            let Some(i) = self.monsters.iter().position(|m| m.id == id) else { continue };
            let m = self.monsters.remove(i);
            let c = m.center();
            self.explosions.push(crate::equipment::Explosion { location: m.location.clone(), center: c, radius: 2, ms: 400 });
            self.sound("explosion");
            if m.location == self.current && self.farmer.center().distance(c) < TILE_F * 2.0 {
                self.damage_farmer(m.info.damage * 2, c);
            }
            self.monster_killed(m, true);
        }
        for id in fx.metamorph {
            let Some(i) = self.monsters.iter().position(|m| m.id == id) else { continue };
            let (loc, pos) = (self.monsters[i].location.clone(), self.monsters[i].center());
            let fly = self.monster_info.keys().find(|k| k.to_ascii_lowercase().contains("fly")).cloned();
            if let Some(fly) = fly {
                self.monsters.remove(i);
                self.spawn_monster_at(&fly, "fly", &loc, pos);
            } else {
                self.monsters[i].age_ms = 0;
            }
        }
    }

    /// Move monster projectiles; they hurt the farmer on contact.
    pub(crate) fn update_enemy_shots(&mut self, dt: u32) {
        if self.enemy_shots.is_empty() {
            return;
        }
        let here = self.current.clone();
        let fb = self.farmer.bbox();
        let mut hit = None;
        let loc = self.locations.get(&here);
        for s in self.enemy_shots.iter_mut() {
            s.ttl_ms = s.ttl_ms.saturating_sub(dt);
            s.pos += s.vel;
            if s.location != here {
                continue;
            }
            if s.kind != ShotKind::Spell {
                if let Some(l) = loc {
                    let t = crate::TilePos::from_world(s.pos);
                    if l.in_bounds(t) && !crate::collision::tile_passable(l, t) {
                        s.ttl_ms = 0;
                    }
                }
            }
            if s.ttl_ms > 0 && fb.contains(s.pos.x as i32, s.pos.y as i32) {
                hit = Some((s.damage, s.pos));
                s.ttl_ms = 0;
            }
        }
        self.enemy_shots.retain(|s| s.ttl_ms > 0);
        if let Some((dmg, from)) = hit {
            self.damage_farmer(dmg, from);
        }
    }
}
