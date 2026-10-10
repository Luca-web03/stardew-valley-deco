//! Weapon special moves, the slingshot and projectiles. Timings, cooldowns
//! and ammo damage are approximations kept as constants.

use engine::{IRect, Rng, Vec2};

use crate::collision::box_blocked;
use crate::combat::{Floater, MONSTER_INVINCIBLE_MS};
use crate::farmer::Skill;
use crate::item::Item;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos, TILE_F};

/// `Type` values in Data/Weapons.
pub const TYPE_STAB_SWORD: i32 = 0;
pub const TYPE_DAGGER: i32 = 1;
pub const TYPE_CLUB: i32 = 2;
pub const TYPE_SLASH_SWORD: i32 = 3;
pub const TYPE_SLINGSHOT: i32 = 4;

pub const BLOCK_MS: u32 = 600;
pub const SWORD_COOLDOWN_MS: u32 = 3000;
pub const FLURRY_HITS: u32 = 4;
pub const FLURRY_INTERVAL_MS: u32 = 120;
pub const DAGGER_COOLDOWN_MS: u32 = 3000;
pub const SLAM_WINDUP_MS: u32 = 500;
pub const SLAM_RADIUS: i32 = 2 * 64;
pub const CLUB_COOLDOWN_MS: u32 = 6000;
pub const SLING_FULL_PULL_MS: u32 = 600;
pub const PROJECTILE_SPEED: f32 = 0.9;
pub const PROJECTILE_LIFE_MS: u32 = 1200;

/// Ammo damage for well-known item ids (approx.); content can set
/// `Port.AmmoDamage` on any object instead.
const AMMO: &[(&str, i32)] = &[("390", 5), ("388", 2), ("378", 10), ("382", 15), ("380", 20), ("384", 30), ("386", 50)];

/// One melee/projectile strike's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub min: i32,
    pub max: i32,
    pub knockback: f32,
    pub crit: f32,
    pub crit_mult: f32,
    /// Strike monsters that are still flashing from a previous hit.
    pub ignore_invincible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpecialMove {
    /// Sword: brief invulnerability that repels attackers.
    Block { ms_left: u32 },
    /// Dagger: a quick series of stabs.
    Flurry { hits_left: u32, next_ms: u32 },
    /// Club: wind up, then hit everything around the farmer.
    Slam { windup_ms: u32 },
}

/// Slingshot being pulled back.
#[derive(Clone, Debug, PartialEq)]
pub struct SlingPull {
    pub slot: usize,
    pub ms: u32,
    /// World-space point the shot aims at.
    pub aim: Option<Vec2>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Projectile {
    pub location: String,
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: i32,
    pub ms_left: u32,
    /// Ammo item (drawn as the projectile).
    pub item_id: String,
}

impl Projectile {
    pub fn bbox(&self) -> IRect {
        IRect::new(self.pos.x as i32 - 8, self.pos.y as i32 - 8, 16, 16)
    }
}

/// Cooldown after a weapon type's special move.
pub fn special_cooldown(kind: i32) -> u32 {
    match kind {
        TYPE_DAGGER => DAGGER_COOLDOWN_MS,
        TYPE_CLUB => CLUB_COOLDOWN_MS,
        _ => SWORD_COOLDOWN_MS,
    }
}

impl World {
    pub fn held_weapon_type(&self) -> Option<i32> {
        let id = self.farmer.current_item()?.weapon_id()?;
        self.items.weapons.get(id).map(|w| w.kind)
    }

    fn held_hit(&self) -> Option<Hit> {
        let id = self.farmer.current_item()?.weapon_id()?;
        let w = self.items.weapons.get(id)?;
        Some(Hit { min: w.min_damage, max: w.max_damage, knockback: w.knockback, crit: w.crit_chance, crit_mult: w.crit_multiplier, ignore_invincible: false })
    }

    /// Damage every monster in `area`; returns how many were struck.
    pub fn hit_area(&mut self, area: IRect, hit: Hit) -> usize {
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 31]));
        let here = self.current.clone();
        let center = self.farmer.center();
        let targets: Vec<usize> = self
            .monsters
            .iter()
            .enumerate()
            .filter(|(_, m)| m.location == here && (hit.ignore_invincible || m.invincible_ms == 0) && m.bbox().intersects(&area))
            .map(|(i, _)| i)
            .collect();
        if targets.is_empty() {
            return 0;
        }
        let attack_bonus = self.total_attrs().attack;
        let crit = if self.farmer.has_profession("scout") { hit.crit * 1.5 } else { hit.crit };
        let mut mult = 1.0 + attack_bonus as f32 * 0.03;
        if self.farmer.has_profession("fighter") {
            mult *= 1.1;
        }
        if self.farmer.has_profession("brute") {
            mult *= 1.15;
        }
        let mut killed = Vec::new();
        for &i in &targets {
            let m = &mut self.monsters[i];
            if rng.chance(m.info.miss_chance) {
                self.floaters.push(Floater { location: here.clone(), pos: m.center(), text: "Miss".into(), color: engine::Color::WHITE, ms: 800 });
                continue;
            }
            let mut dmg = rng.range(hit.min.max(0), hit.max.max(hit.min) + 1);
            dmg = (dmg as f32 * mult).round() as i32;
            let is_crit = rng.chance(crit as f64);
            if is_crit {
                dmg = (dmg as f32 * hit.crit_mult) as i32;
            }
            dmg = (dmg - m.info.resilience).max(1);
            m.health -= dmg;
            m.invincible_ms = MONSTER_INVINCIBLE_MS;
            let away = (m.center() - center).normalized();
            m.knock = away * (8.0 * hit.knockback.max(0.3));
            self.floaters.push(Floater {
                location: here.clone(),
                pos: m.center() - Vec2::new(0.0, 40.0),
                text: dmg.to_string(),
                color: if is_crit { engine::Color::YELLOW } else { engine::Color::rgb(255, 120, 90) },
                ms: 900,
            });
            if m.health <= 0 {
                killed.push(i);
            }
        }
        self.sound(if killed.is_empty() { "hitEnemy" } else { "monsterKilled" });
        killed.sort_unstable();
        for i in killed.into_iter().rev() {
            let m = self.monsters.remove(i);
            if let Some(l) = self.farmer.skills.gain(Skill::Combat, m.info.exp) {
                self.emit(WorldEvent::LevelUp(Skill::Combat, l));
            }
            self.monsters_killed += 1;
            self.quest_killed(&m.info.name.clone());
            self.quest_killed(&m.info.display_name.clone());
            for (id, chance) in &m.info.drops {
                if rng.chance(*chance) {
                    if let Some(item) = self.items.create(id, 1) {
                        self.spawn_drop(item, m.center());
                    }
                }
            }
        }
        targets.len()
    }

    // ------------------------------------------------------------ specials

    /// Action button with a melee weapon: start its special move.
    pub fn begin_special_move(&mut self) -> bool {
        let Some(kind) = self.held_weapon_type() else { return false };
        if kind == TYPE_SLINGSHOT || self.special.is_some() || self.weapon_swing.is_some() {
            return false;
        }
        if self.special_cooldown_ms > 0 {
            return true;
        }
        let (mv, cue) = match kind {
            TYPE_DAGGER => (SpecialMove::Flurry { hits_left: FLURRY_HITS, next_ms: 0 }, "daggerswipe"),
            TYPE_CLUB => (SpecialMove::Slam { windup_ms: SLAM_WINDUP_MS }, "clubhit"),
            _ => (SpecialMove::Block { ms_left: BLOCK_MS }, "swordswipe"),
        };
        self.special = Some(mv);
        self.special_cooldown_ms = special_cooldown(kind);
        self.sound(cue);
        true
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self.special, Some(SpecialMove::Block { .. }))
    }

    /// Advance the active special move; true while it holds the farmer still.
    pub(crate) fn update_special(&mut self, dt: u32) -> bool {
        self.special_cooldown_ms = self.special_cooldown_ms.saturating_sub(dt);
        let Some(mv) = self.special.take() else { return false };
        let hit = self.held_hit();
        let next = match mv {
            SpecialMove::Block { ms_left } => {
                // Push back anything touching the farmer.
                let fb = self.farmer.bbox();
                let c = self.farmer.center();
                let here = self.current.clone();
                for m in self.monsters.iter_mut().filter(|m| m.location == here && m.bbox().intersects(&fb.inflate(16))) {
                    m.knock = (m.center() - c).normalized() * 10.0;
                }
                (ms_left > dt).then(|| SpecialMove::Block { ms_left: ms_left - dt })
            }
            SpecialMove::Flurry { hits_left, next_ms } => {
                if next_ms > dt {
                    Some(SpecialMove::Flurry { hits_left, next_ms: next_ms - dt })
                } else {
                    if let Some(h) = hit {
                        let area = self.swing_area(0);
                        self.hit_area(area, Hit { ignore_invincible: true, knockback: h.knockback * 0.3, ..h });
                    }
                    (hits_left > 1).then_some(SpecialMove::Flurry { hits_left: hits_left - 1, next_ms: FLURRY_INTERVAL_MS })
                }
            }
            SpecialMove::Slam { windup_ms } => {
                if windup_ms > dt {
                    Some(SpecialMove::Slam { windup_ms: windup_ms - dt })
                } else {
                    if let Some(h) = hit {
                        let c = self.farmer.center();
                        let area = IRect::new(c.x as i32 - SLAM_RADIUS, c.y as i32 - SLAM_RADIUS, SLAM_RADIUS * 2, SLAM_RADIUS * 2);
                        self.hit_area(area, Hit { ignore_invincible: true, knockback: h.knockback * 1.5, ..h });
                    }
                    self.sound("explosion");
                    None
                }
            }
        };
        self.special = next;
        true
    }

    // ------------------------------------------------------------ slingshot

    /// The ammo the slingshot would fire: (inventory slot, damage).
    pub fn slingshot_ammo(&self) -> Option<(usize, i32)> {
        self.farmer.inventory.slots.iter().enumerate().find_map(|(i, s)| match s {
            Some(Item::Object { id, .. }) => self.ammo_damage(id).map(|d| (i, d)),
            _ => None,
        })
    }

    pub fn ammo_damage(&self, id: &str) -> Option<i32> {
        if let Some(d) = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.AmmoDamage")).and_then(|v| v.parse().ok()) {
            return Some(d);
        }
        AMMO.iter().find(|(a, _)| *a == id).map(|(_, d)| *d)
    }

    /// Use button with a slingshot: start pulling back.
    pub(crate) fn begin_sling_pull(&mut self, aim: Option<TilePos>) -> bool {
        if self.slingshot_ammo().is_none() {
            self.hud("Out of ammo.");
            return true;
        }
        let aim = aim.map(|t| Vec2::new(t.x as f32 * TILE_F + TILE_F / 2.0, t.y as f32 * TILE_F + TILE_F / 2.0));
        self.sling = Some(SlingPull { slot: self.farmer.current_slot, ms: 0, aim });
        self.sound("slingshot");
        true
    }

    /// Hold to pull, release to fire. True while pulling.
    pub(crate) fn update_sling(&mut self, held: bool, aim: Option<TilePos>, dt: u32) -> bool {
        let Some(mut pull) = self.sling.take() else { return false };
        if let Some(t) = aim {
            pull.aim = Some(Vec2::new(t.x as f32 * TILE_F + TILE_F / 2.0, t.y as f32 * TILE_F + TILE_F / 2.0));
        }
        if held {
            pull.ms = (pull.ms + dt).min(SLING_FULL_PULL_MS);
            if let Some(a) = pull.aim {
                let d = a - self.farmer.center();
                if d.length() > 1.0 {
                    self.farmer.facing = Dir::toward((d.x * 10.0) as i32, (d.y * 10.0) as i32);
                }
            }
            self.sling = Some(pull);
            return true;
        }
        self.fire_slingshot(&pull);
        false
    }

    fn fire_slingshot(&mut self, pull: &SlingPull) {
        let Some((slot, ammo)) = self.slingshot_ammo() else { return };
        let Some(Item::Object { id, .. }) = self.farmer.inventory.get(slot).cloned() else { return };
        let weapon = self.farmer.inventory.get(pull.slot).and_then(|i| i.weapon_id()).and_then(|w| self.items.weapons.get(w)).cloned();
        let power = (pull.ms as f32 / SLING_FULL_PULL_MS as f32).clamp(0.25, 1.0);
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 77]));
        let bonus = weapon.as_ref().map(|w| rng.range(w.min_damage.max(0), w.max_damage.max(w.min_damage) + 1)).unwrap_or(0);
        let damage = ((ammo + bonus) as f32 * power).round().max(1.0) as i32;
        let start = self.farmer.center() - Vec2::new(0.0, 24.0);
        let dir = match pull.aim {
            Some(a) if (a - start).length() > 4.0 => (a - start).normalized(),
            _ => {
                let (dx, dy) = self.farmer.facing.delta();
                Vec2::new(dx as f32, dy as f32)
            }
        };
        self.farmer.inventory.reduce(slot, 1);
        self.projectiles.push(Projectile {
            location: self.current.clone(),
            pos: start,
            vel: dir * PROJECTILE_SPEED * (0.6 + 0.4 * power),
            damage,
            ms_left: PROJECTILE_LIFE_MS,
            item_id: id,
        });
        self.sound("slingshot");
    }

    pub(crate) fn update_projectiles(&mut self, dt: u32) {
        let mut live = std::mem::take(&mut self.projectiles);
        let mut i = 0;
        while i < live.len() {
            let p = &mut live[i];
            p.ms_left = p.ms_left.saturating_sub(dt);
            p.pos = p.pos + p.vel * dt as f32;
            let gone = p.ms_left == 0
                || self.locations.get(&p.location).map_or(true, |l| {
                    let b = p.bbox();
                    // Only walls stop shots; the shooter's own tile is fine.
                    b.x < 0 || b.y < 0 || b.x > l.width() * 64 || b.y > l.height() * 64 || box_blocked(l, IRect::new(b.x + 4, b.y + 4, 8, 8))
                });
            if gone {
                live.remove(i);
                continue;
            }
            if p.location == self.current {
                let (area, dmg) = (p.bbox(), p.damage);
                let hit = Hit { min: dmg, max: dmg, knockback: 0.6, crit: 0.02, crit_mult: 2.0, ignore_invincible: false };
                if self.hit_area(area, hit) > 0 {
                    live.remove(i);
                    continue;
                }
            }
            i += 1;
        }
        self.projectiles = live;
    }
}
