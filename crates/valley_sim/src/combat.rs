//! Monsters, weapon swings, damage and fainting.
//!
//! Monster stats come from `Data/Monsters` (slash-delimited, public format:
//! health/damage/minCoins/maxCoins/isGlider/randomMoveDuration/drops/
//! resilience/jitteriness/chaseDistance/speed/missChance/isMineMonster/exp/
//! displayName). AI and timing constants are approximations.

use engine::{IRect, Rng, Vec2};

use crate::collision::box_blocked;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos, TILE_F};

#[derive(Clone, Debug, PartialEq)]
pub struct MonsterInfo {
    pub name: String,
    pub display_name: String,
    pub health: i32,
    pub damage: i32,
    pub glider: bool,
    pub drops: Vec<(String, f64)>,
    pub resilience: i32,
    pub chase_tiles: i32,
    pub speed: f32,
    pub miss_chance: f64,
    pub exp: i32,
}

pub fn parse_monster(name: &str, s: &str) -> Option<MonsterInfo> {
    let f: Vec<&str> = s.split('/').collect();
    let num = |i: usize, d: i32| f.get(i).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(d);
    let drops_raw: Vec<&str> = f.get(6).map(|d| d.split_whitespace().collect()).unwrap_or_default();
    let drops = drops_raw
        .chunks(2)
        .filter_map(|c| Some((c.first()?.to_string(), c.get(1)?.parse::<f64>().ok()?)))
        .collect();
    Some(MonsterInfo {
        name: name.to_string(),
        display_name: f.get(14).map(|s| s.to_string()).filter(|s| !s.is_empty()).unwrap_or_else(|| name.to_string()),
        health: num(0, 10),
        damage: num(1, 3),
        glider: f.get(4).is_some_and(|v| v.trim().eq_ignore_ascii_case("true")),
        drops,
        resilience: num(7, 0),
        chase_tiles: num(9, 6).max(1),
        speed: num(10, 2) as f32,
        miss_chance: f.get(11).and_then(|v| v.parse().ok()).unwrap_or(0.0),
        exp: num(13, 1),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    Walker,
    Hopper,
    Flyer,
}

pub fn movement_for(info: &MonsterInfo) -> Movement {
    if info.glider {
        Movement::Flyer
    } else {
        let n = info.name.to_ascii_lowercase();
        if n.contains("slime") || n.contains("jelly") || n.contains("blob") {
            Movement::Hopper
        } else {
            Movement::Walker
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Monster {
    pub id: u32,
    pub info: MonsterInfo,
    pub location: String,
    /// World-space top-left of the 64x64 footprint.
    pub pos: Vec2,
    pub knock: Vec2,
    pub health: i32,
    pub movement: Movement,
    pub facing: Dir,
    pub anim_ms: u32,
    pub invincible_ms: u32,
    /// Hop height (hoppers) in pixels.
    pub z: f32,
    pub vz: f32,
    pub hop_cooldown: u32,
    pub wander: Vec2,
    pub wander_ms: u32,
    /// Species behaviour (from the name) and its state.
    pub species: crate::monster_kinds::Species,
    /// Disguised (crab) or underground (burrower): no contact, no hits.
    pub hidden: bool,
    pub state_ms: u32,
    /// Hits the shell still absorbs (crabs).
    pub shell: i32,
    pub shot_cd: u32,
    /// Collapsed mummy: ms until it gets back up.
    pub collapsed_ms: u32,
    /// Lit fuse (hot heads).
    pub fuse_ms: Option<u32>,
    pub age_ms: u32,
}

impl Monster {
    pub fn new(id: u32, info: MonsterInfo, location: &str, tile: TilePos) -> Self {
        use crate::monster_kinds::Species;
        let movement = movement_for(&info);
        let species = crate::monster_kinds::species_for(&info.name);
        Self {
            species,
            hidden: matches!(species, Species::Crab),
            state_ms: 1000,
            shell: if species == Species::Crab { crate::monster_kinds::SHELL_HITS } else { 0 },
            shot_cd: 1500,
            collapsed_ms: 0,
            fuse_ms: None,
            age_ms: 0,
            id,
            health: info.health,
            info,
            location: location.to_string(),
            pos: Vec2::new(tile.x as f32 * TILE_F, tile.y as f32 * TILE_F),
            knock: Vec2::ZERO,
            movement,
            facing: Dir::Down,
            anim_ms: 0,
            invincible_ms: 0,
            z: 0.0,
            vz: 0.0,
            hop_cooldown: 0,
            wander: Vec2::ZERO,
            wander_ms: 0,
        }
    }

    pub fn bbox_at(p: Vec2) -> IRect {
        IRect::new(p.x as i32 + 12, p.y as i32 + 20, 40, 40)
    }

    pub fn bbox(&self) -> IRect {
        Self::bbox_at(self.pos)
    }

    pub fn center(&self) -> Vec2 {
        self.bbox().center()
    }

    pub fn tile(&self) -> TilePos {
        TilePos::from_world(self.center())
    }
}

/// A floating damage number / text in world space.
#[derive(Clone, Debug, PartialEq)]
pub struct Floater {
    pub location: String,
    pub pos: Vec2,
    pub text: String,
    pub color: engine::Color,
    pub ms: u32,
}

/// In-progress melee swing.
#[derive(Clone, Debug, PartialEq)]
pub struct WeaponSwing {
    pub slot: usize,
    pub elapsed_ms: u32,
    pub total_ms: u32,
    pub applied: bool,
}

pub const PLAYER_INVINCIBLE_MS: u32 = 1200;
pub const MONSTER_INVINCIBLE_MS: u32 = 250;

impl World {
    /// Start swinging the held weapon.
    pub fn begin_weapon_swing(&mut self) -> bool {
        let slot = self.farmer.current_slot;
        let Some(item) = self.farmer.inventory.get(slot).filter(|i| i.weapon_id().is_some()).cloned() else { return false };
        let speed = self.weapon_stats(&item).map(|w| w.speed).unwrap_or(0) ;
        // Faster weapons swing quicker (approx. 10% per speed point, clamped).
        let total = (400.0 * (1.0 - speed as f32 * 0.1).clamp(0.4, 1.8)) as u32;
        self.weapon_swing = Some(WeaponSwing { slot, elapsed_ms: 0, total_ms: total, applied: false });
        self.sound("swordswipe");
        true
    }

    pub(crate) fn advance_weapon_swing(&mut self, dt: u32) {
        let Some(mut s) = self.weapon_swing.take() else { return };
        s.elapsed_ms += dt;
        if !s.applied && s.elapsed_ms >= s.total_ms / 3 {
            s.applied = true;
            let item = self.farmer.inventory.get(s.slot).cloned();
            if let Some(w) = item.as_ref().and_then(|i| self.weapon_stats(i)) {
                self.melee_hit(w.min, w.max, w.knockback, w.crit, w.crit_mult, w.aoe);
                // Blades cut grass and weeds too.
                let area = self.swing_area(w.aoe);
                self.weapon_cut(area);
            }
        }
        if s.elapsed_ms < s.total_ms {
            self.weapon_swing = Some(s);
        }
    }

    /// A weapon swing cuts the grass and weeds it passes through.
    pub(crate) fn weapon_cut(&mut self, area: IRect) {
        let ts = crate::TILE;
        let (x0, y0) = (area.x.div_euclid(ts), area.y.div_euclid(ts));
        let (x1, y1) = ((area.x + area.w - 1).div_euclid(ts), (area.y + area.h - 1).div_euclid(ts));
        for y in y0..=y1 {
            for x in x0..=x1 {
                let t = TilePos::new(x, y);
                let weeds = matches!(self.location().object(t).map(|o| &o.kind), Some(crate::objects::ObjKind::Debris { debris: crate::objects::DebrisKind::Weeds, .. }));
                if weeds {
                    self.break_weeds(t);
                } else {
                    self.cut_grass_with(t, false);
                }
            }
        }
    }

    /// Area in front of the farmer hit by a melee swing.
    pub fn swing_area(&self, extra: i32) -> IRect {
        let c = self.farmer.center();
        let (cx, cy) = (c.x as i32, c.y as i32);
        let reach = 72 + extra;
        let wide = 112 + extra;
        match self.farmer.facing {
            Dir::Up => IRect::new(cx - wide / 2, cy - reach - 24, wide, reach),
            Dir::Down => IRect::new(cx - wide / 2, cy, wide, reach),
            Dir::Left => IRect::new(cx - reach - 8, cy - wide / 2 - 16, reach, wide),
            Dir::Right => IRect::new(cx + 8, cy - wide / 2 - 16, reach, wide),
        }
    }

    /// Apply a melee hit to every monster in the swing area.
    pub fn melee_hit(&mut self, min: i32, max: i32, knockback: f32, crit: f32, crit_mult: f32, aoe: i32) {
        let area = self.swing_area(aoe);
        let hit = crate::arsenal::Hit { min, max, knockback, crit, crit_mult, ignore_invincible: false };
        if self.hit_area(area, hit) == 0 {
            self.sound("hitEnemy");
        }
    }

    /// Per-tick monster AI and contact damage.
    pub(crate) fn update_monsters(&mut self, dt: u32) {
        let here = self.current.clone();
        let target = self.farmer.center();
        let fb = self.farmer.bbox();
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 47]));
        let mut contact_damage = None;
        let loc = match self.locations.get(&here) {
            Some(l) => l,
            None => return,
        };
        let mut fx = crate::monster_kinds::SpeciesEffects::default();
        for m in self.monsters.iter_mut().filter(|m| m.location == here) {
            m.anim_ms += dt;
            m.invincible_ms = m.invincible_ms.saturating_sub(dt);
            if crate::monster_kinds::species_step(m, dt, target, &mut rng, &mut fx) == crate::monster_kinds::Step::Hold {
                m.knock = Vec2::ZERO;
                continue;
            }
            // Knockback first.
            if m.knock.length() > 0.2 {
                let np = m.pos + m.knock;
                if m.movement == Movement::Flyer || !box_blocked(loc, Monster::bbox_at(np)) {
                    m.pos = np;
                }
                m.knock = m.knock * 0.8;
                continue;
            }
            let to = target - m.center();
            let dist = to.length();
            let chasing = dist < m.info.chase_tiles as f32 * TILE_F;
            let dir = if chasing {
                to.normalized()
            } else {
                m.wander_ms = m.wander_ms.saturating_sub(dt);
                if m.wander_ms == 0 {
                    m.wander = Vec2::new(rng.range(-10, 11) as f32, rng.range(-10, 11) as f32).normalized();
                    m.wander_ms = rng.range(800, 2500) as u32;
                }
                m.wander * 0.5
            };
            let speed = m.info.speed.max(1.0) * if m.species == crate::monster_kinds::Species::Dust { 1.6 } else { 1.0 };
            match m.movement {
                Movement::Flyer => {
                    let wobble = Vec2::new((m.anim_ms as f32 / 200.0).sin(), (m.anim_ms as f32 / 260.0).cos()) * 0.6;
                    m.pos += (dir + wobble) * speed;
                }
                Movement::Hopper => {
                    if m.z > 0.0 || m.vz > 0.0 {
                        m.z += m.vz;
                        m.vz -= 0.5;
                        let np = m.pos + dir * (speed * 1.2);
                        if !box_blocked(loc, Monster::bbox_at(np)) {
                            m.pos = np;
                        }
                        if m.z <= 0.0 {
                            m.z = 0.0;
                            m.vz = 0.0;
                            m.hop_cooldown = rng.range(400, 1000) as u32;
                        }
                    } else if m.hop_cooldown > 0 {
                        m.hop_cooldown = m.hop_cooldown.saturating_sub(dt);
                    } else if chasing || rng.chance(0.02) {
                        m.vz = 6.0;
                    }
                }
                Movement::Walker => {
                    for d in [Vec2::new(dir.x * speed, 0.0), Vec2::new(0.0, dir.y * speed)] {
                        let np = m.pos + d;
                        if !box_blocked(loc, Monster::bbox_at(np)) {
                            m.pos = np;
                        }
                    }
                }
            }
            if dir.length() > 0.1 {
                m.facing = Dir::toward((dir.x * 100.0) as i32, (dir.y * 100.0) as i32);
            }
            if contact_damage.is_none() && !m.hidden && m.bbox().intersects(&fb) && m.z < 16.0 {
                contact_damage = Some((m.info.damage, m.center()));
            }
        }
        // Collapsed mummies and fused hot heads stand still but still hurt.
        if contact_damage.is_none() {
            contact_damage = self.monsters.iter().filter(|m| m.location == here && m.collapsed_ms == 0 && !m.hidden && m.bbox().intersects(&fb)).map(|m| (m.info.damage, m.center())).next();
        }
        if let Some((dmg, from)) = contact_damage {
            self.damage_farmer(dmg, from);
        }
        self.apply_species_effects(fx);
        self.update_enemy_shots(dt);
        for f in self.floaters.iter_mut() {
            f.ms = f.ms.saturating_sub(dt);
            f.pos.y -= 0.8;
        }
        self.floaters.retain(|f| f.ms > 0);
    }

    pub fn damage_farmer(&mut self, dmg: i32, from: Vec2) {
        if self.farmer.invincible_ms > 0 {
            return;
        }
        if self.is_blocking() {
            self.sound("parry");
            self.farmer.invincible_ms = PLAYER_INVINCIBLE_MS / 2;
            return;
        }
        let defense = self.farmer_defense();
        let dmg = (dmg - defense).max(1);
        self.farmer.health -= dmg;
        self.farmer.invincible_ms = PLAYER_INVINCIBLE_MS;
        let away = (self.farmer.center() - from).normalized();
        self.farmer.knockback = away * 8.0;
        self.floaters.push(Floater { location: self.current.clone(), pos: self.farmer.center() - Vec2::new(0.0, 80.0), text: dmg.to_string(), color: engine::Color::RED, ms: 900 });
        self.sound("ow");
        if self.farmer.health <= 0 {
            self.faint();
        }
    }

    pub fn farmer_defense(&self) -> i32 {
        self.total_attrs().defense + self.farmer.current_item().and_then(|i| self.weapon_stats(i)).map(|w| w.defense).unwrap_or(0)
    }

    /// Knocked out: lose some money, wake up at home with partial health.
    /// (Approximation of the original's rescue behaviour.)
    pub fn faint(&mut self) {
        let lost = (self.farmer.money / 10).min(1000);
        self.farmer.money -= lost;
        self.farmer.health = (self.farmer.max_health / 2).max(1);
        self.farmer.stamina = self.farmer.stamina.max(self.farmer.max_stamina as f32 * 0.25);
        self.farmer.invincible_ms = 0;
        self.weapon_swing = None;
        if crate::volcano::volcano_floor(&self.current).is_some() {
            self.locations.retain(|k, _| crate::volcano::volcano_floor(k).is_none());
            self.monsters.retain(|m| crate::volcano::volcano_floor(&m.location).is_none());
        }
        self.special = None;
        self.sling = None;
        self.projectiles.clear();
        self.farmer.tool_use = None;
        self.leave_mines();
        let wake = self.bed_tile;
        let home = if self.locations.contains_key("Hospital") { "Hospital" } else { "FarmHouse" };
        let tile = if home == "FarmHouse" { TilePos::new(wake.x - 1, wake.y) } else { TilePos::new(10, 10) };
        self.warp_farmer(home, tile, Some(Dir::Down));
        self.clock.time = (crate::time::Clock::add_minutes(self.clock.time, 120)).min(2500);
        self.emit(WorldEvent::Message(format!("You were knocked out... Someone found you and brought you home. You lost {lost}g.")));
    }

    /// Farmer movement step applying knockback (called before normal movement).
    pub(crate) fn apply_farmer_knockback(&mut self, dt: u32) {
        self.farmer.invincible_ms = self.farmer.invincible_ms.saturating_sub(dt);
        let k = self.farmer.knockback;
        if k.length() > 0.2 {
            let np = self.farmer.pos + k;
            if !box_blocked(self.location(), crate::farmer::Farmer::bbox_at(np)) {
                self.farmer.pos = np;
            }
            self.farmer.knockback = k * 0.75;
        } else {
            self.farmer.knockback = Vec2::ZERO;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_monster_strings() {
        let m = parse_monster("Blob", "24/5/0/0/false/1000/A .5 B .1/1/.01/4/2/.0/true/3/Mud Blob").unwrap();
        assert_eq!(m.health, 24);
        assert_eq!(m.damage, 5);
        assert_eq!(m.drops, vec![("A".to_string(), 0.5), ("B".to_string(), 0.1)]);
        assert_eq!(m.exp, 3);
        assert_eq!(m.display_name, "Mud Blob");
        assert_eq!(movement_for(&m), Movement::Hopper);
        let b = parse_monster("Moth", "10/3/0/0/true/1000/A .5/0/.01/8/3/.1/true/2/Moth").unwrap();
        assert_eq!(movement_for(&b), Movement::Flyer);
    }
}
