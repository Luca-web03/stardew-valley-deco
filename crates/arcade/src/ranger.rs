//! "Dust Ranger": an original top-down arena shooter. Critters pour in
//! through four gates; survive each 60-second round, pick up coins and
//! power-ups, and spend coins at the trading post between rounds. Eight
//! rounds to win.

use engine::{Color, Rng};

use crate::{overlaps, ArcadeGame, ArcadeInput, ArcadeResult, ArcadeStatus, Canvas, SCREEN_W};

const TOP: f32 = 16.0;
const TILE: f32 = 16.0;
const COLS: i32 = 16;
const ROWS: i32 = 13;
const ROUND_TICKS: u32 = 60 * 60;
pub const ROUNDS: u32 = 8;
const POWER_TICKS: u32 = 600;
const PLAYER: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Title,
    Playing,
    /// Lost a life; the arena clears for a moment.
    Down(u32),
    /// Round cleared; trading post.
    Shop,
    Victory,
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Critter {
    Bunny,
    Beetle,
    Wisp,
}

impl Critter {
    fn hp(self) -> i32 {
        match self {
            Critter::Bunny => 1,
            Critter::Beetle => 3,
            Critter::Wisp => 2,
        }
    }
    fn speed(self) -> f32 {
        match self {
            Critter::Bunny => 0.7,
            Critter::Beetle => 0.45,
            Critter::Wisp => 0.85,
        }
    }
    fn size(self) -> f32 {
        match self {
            Critter::Beetle => 12.0,
            _ => 10.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    Coin,
    Rapid,
    Spread,
    Boots,
}

#[derive(Clone, Debug)]
struct Enemy {
    kind: Critter,
    x: f32,
    y: f32,
    hp: i32,
    flash: u32,
    anim: u32,
}

#[derive(Clone, Debug)]
struct Bullet {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

#[derive(Clone, Debug)]
struct Pickup {
    kind: Power,
    x: f32,
    y: f32,
    ttl: u32,
}

pub struct DustRanger {
    rng: Rng,
    pub phase: Phase,
    pub round: u32,
    pub lives: i32,
    pub coins: i32,
    pub score: i32,
    pub damage: i32,
    pub speed_level: i32,
    px: f32,
    py: f32,
    facing_left: bool,
    walk: u32,
    invuln: u32,
    cooldown: u32,
    timer: u32,
    spawn_in: u32,
    rapid: u32,
    spread: u32,
    boots: u32,
    rocks: Vec<(i32, i32)>,
    enemies: Vec<Enemy>,
    bullets: Vec<Bullet>,
    pickups: Vec<Pickup>,
    won: bool,
}

const RANGER: [&str; 12] = [
    "..1111....",
    ".111111...",
    "11111111..",
    "..2222....",
    "..2332....",
    "..2222....",
    ".444444...",
    "4.4444.4..",
    "..4444....",
    "..5..5....",
    "..5..5....",
    ".55..55...",
];
const BUNNY: [&str; 10] = [
    ".1....1...",
    ".1....1...",
    ".11..11...",
    "..1111....",
    ".121121...",
    ".111111...",
    "11111111..",
    "11111111..",
    ".111111...",
    ".1....1...",
];
const BEETLE: [&str; 12] = [
    "...2..2.....",
    "....22......",
    "..111111....",
    ".11311311...",
    "1111111111..",
    "1131111311..",
    "1111111111..",
    "1111111111..",
    ".11111111...",
    "..111111....",
    ".2..2..2....",
    "2...2...2...",
];
const WISP: [&str; 10] = [
    "...11.....",
    "..1111....",
    ".111111...",
    ".121121...",
    ".111111...",
    "11111111..",
    "11111111..",
    "1.11.11.1.",
    "1..1..1...",
    "..........",
];
const CACTUS: [&str; 16] = [
    "......11........",
    ".....1111.......",
    ".....1211.......",
    ".....1111.......",
    "..11.1111.......",
    "..11.1211..11...",
    "..1111111..11...",
    "...111111111....",
    ".....1111.......",
    ".....1211.......",
    ".....1111.......",
    ".....1111.......",
    ".....1111.......",
    "....333333......",
    "...33333333.....",
    "................",
];

impl DustRanger {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Rng::new(seed ^ 0x5a17),
            phase: Phase::Title,
            round: 1,
            lives: 3,
            coins: 0,
            score: 0,
            damage: 1,
            speed_level: 0,
            px: SCREEN_W / 2.0,
            py: TOP + ROWS as f32 * TILE / 2.0,
            facing_left: false,
            walk: 0,
            invuln: 0,
            cooldown: 0,
            timer: 0,
            spawn_in: 60,
            rapid: 0,
            spread: 0,
            boots: 0,
            rocks: Vec::new(),
            enemies: Vec::new(),
            bullets: Vec::new(),
            pickups: Vec::new(),
            won: false,
        }
    }

    fn start_round(&mut self) {
        self.timer = 0;
        self.spawn_in = 90;
        self.enemies.clear();
        self.bullets.clear();
        self.pickups.clear();
        self.px = SCREEN_W / 2.0;
        self.py = TOP + ROWS as f32 * TILE / 2.0;
        self.rocks.clear();
        let n = 5 + self.round as i32;
        let mut tries = 0;
        while (self.rocks.len() as i32) < n && tries < 200 {
            tries += 1;
            let (x, y) = (self.rng.range(1, COLS - 1), self.rng.range(1, ROWS - 1));
            let near_gate = (x - COLS / 2).abs() <= 1 || (y - ROWS / 2).abs() <= 1;
            let near_middle = (x - COLS / 2).abs() <= 2 && (y - ROWS / 2).abs() <= 2;
            if !near_gate && !near_middle && !self.rocks.contains(&(x, y)) {
                self.rocks.push((x, y));
            }
        }
        self.phase = Phase::Playing;
    }

    fn blocked(&self, x: f32, y: f32, size: f32) -> bool {
        let h = size / 2.0;
        if x - h < 2.0 || x + h > SCREEN_W - 2.0 || y - h < TOP + 2.0 || y + h > TOP + ROWS as f32 * TILE - 2.0 {
            return true;
        }
        self.rocks.iter().any(|(rx, ry)| overlaps((x - h, y - h, size, size), (*rx as f32 * TILE + 3.0, TOP + *ry as f32 * TILE + 3.0, TILE - 6.0, TILE - 6.0)))
    }

    fn spawn_wave(&mut self) {
        let gates = [(SCREEN_W / 2.0, TOP + 9.0), (SCREEN_W / 2.0, TOP + ROWS as f32 * TILE - 9.0), (9.0, TOP + ROWS as f32 * TILE / 2.0), (SCREEN_W - 9.0, TOP + ROWS as f32 * TILE / 2.0)];
        let (gx, gy) = gates[self.rng.range(0, 4) as usize];
        let top_or_bottom = gx == SCREEN_W / 2.0;
        let count = self.rng.range(1, 2 + (self.round as i32 / 3).min(3));
        for k in 0..count {
            let r = self.rng.next_f64();
            let kind = if self.round >= 4 && r < 0.2 {
                Critter::Wisp
            } else if self.round >= 2 && r < 0.45 {
                Critter::Beetle
            } else {
                Critter::Bunny
            };
            let off = k as f32 * 6.0;
            self.enemies.push(Enemy { kind, x: gx + if top_or_bottom { off } else { 0.0 }, y: gy + if top_or_bottom { 0.0 } else { off }, hp: kind.hp() + (self.round as i32 - 1) / 4, flash: 0, anim: self.rng.range(0, 60) as u32 });
        }
    }

    fn fire(&mut self, dx: f32, dy: f32) {
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let (dx, dy) = (dx / len * 4.0, dy / len * 4.0);
        self.bullets.push(Bullet { x: self.px, y: self.py, dx, dy });
        if self.spread > 0 {
            for a in [-0.25f32, 0.25] {
                let (s, c) = a.sin_cos();
                self.bullets.push(Bullet { x: self.px, y: self.py, dx: dx * c - dy * s, dy: dx * s + dy * c });
            }
        }
        self.cooldown = if self.rapid > 0 { 7 } else { 14 };
    }

    fn lose_life(&mut self) {
        self.lives -= 1;
        self.enemies.clear();
        self.bullets.clear();
        self.phase = if self.lives < 0 { Phase::GameOver } else { Phase::Down(90) };
    }

    pub fn shop_prices(&self) -> [i32; 3] {
        [8 + 4 * self.damage, 6 + 4 * self.speed_level, 15]
    }

    fn play(&mut self, input: &ArcadeInput) {
        self.timer += 1;
        self.cooldown = self.cooldown.saturating_sub(1);
        self.invuln = self.invuln.saturating_sub(1);
        self.rapid = self.rapid.saturating_sub(1);
        self.spread = self.spread.saturating_sub(1);
        self.boots = self.boots.saturating_sub(1);
        // Move.
        let mut dx = (input.right as i32 - input.left as i32) as f32;
        let mut dy = (input.down as i32 - input.up as i32) as f32;
        if dx != 0.0 && dy != 0.0 {
            dx *= std::f32::consts::FRAC_1_SQRT_2;
            dy *= std::f32::consts::FRAC_1_SQRT_2;
        }
        let speed = 1.25 + 0.15 * self.speed_level as f32 + if self.boots > 0 { 0.5 } else { 0.0 };
        if !self.blocked(self.px + dx * speed, self.py, PLAYER) {
            self.px += dx * speed;
        }
        if !self.blocked(self.px, self.py + dy * speed, PLAYER) {
            self.py += dy * speed;
        }
        if dx != 0.0 || dy != 0.0 {
            self.walk += 1;
        }
        if dx < 0.0 {
            self.facing_left = true;
        } else if dx > 0.0 {
            self.facing_left = false;
        }
        // Shoot.
        let sx = (input.shoot_right as i32 - input.shoot_left as i32) as f32;
        let sy = (input.shoot_down as i32 - input.shoot_up as i32) as f32;
        if (sx != 0.0 || sy != 0.0) && self.cooldown == 0 {
            self.fire(sx, sy);
        }
        // Spawn.
        if self.timer < ROUND_TICKS {
            if self.spawn_in == 0 {
                self.spawn_wave();
                self.spawn_in = (80 - self.round as i32 * 6).max(28) as u32 + self.rng.range(0, 30) as u32;
            } else {
                self.spawn_in -= 1;
            }
        }
        // Bullets.
        for b in self.bullets.iter_mut() {
            b.x += b.dx;
            b.y += b.dy;
        }
        let rocks = self.rocks.clone();
        self.bullets.retain(|b| {
            b.x > 0.0 && b.x < SCREEN_W && b.y > TOP && b.y < TOP + ROWS as f32 * TILE && !rocks.iter().any(|(rx, ry)| overlaps((b.x - 1.0, b.y - 1.0, 2.0, 2.0), (*rx as f32 * TILE + 3.0, TOP + *ry as f32 * TILE + 3.0, TILE - 6.0, TILE - 6.0)))
        });
        // Enemies.
        let (px, py) = (self.px, self.py);
        for i in 0..self.enemies.len() {
            let e = self.enemies[i].clone();
            let (mut ex, mut ey) = (px - e.x, py - e.y);
            let len = (ex * ex + ey * ey).sqrt().max(0.001);
            ex /= len;
            ey /= len;
            let sp = e.kind.speed() * (1.0 + self.round as f32 * 0.04);
            let (nx, ny) = (e.x + ex * sp, e.y + ey * sp);
            let size = e.kind.size();
            let flies = e.kind == Critter::Wisp;
            let mut m = e.clone();
            if flies || !self.blocked(nx, e.y, size) {
                m.x = nx;
            }
            if flies || !self.blocked(m.x, ny, size) {
                m.y = ny;
            }
            if !flies && m.x == e.x && m.y == e.y {
                // Stuck behind a rock: sidestep.
                let side = if (e.anim / 40) % 2 == 0 { 1.0 } else { -1.0 };
                if !self.blocked(e.x + ey * side * sp, e.y - ex * side * sp, size) {
                    m.x += ey * side * sp;
                    m.y -= ex * side * sp;
                }
            }
            m.anim += 1;
            m.flash = m.flash.saturating_sub(1);
            self.enemies[i] = m;
        }
        // Hits.
        let mut dead = Vec::new();
        let mut spent = Vec::new();
        for (bi, b) in self.bullets.iter().enumerate() {
            for (ei, e) in self.enemies.iter_mut().enumerate() {
                let s = e.kind.size();
                if e.hp > 0 && overlaps((b.x - 1.5, b.y - 1.5, 3.0, 3.0), (e.x - s / 2.0, e.y - s / 2.0, s, s)) {
                    e.hp -= self.damage;
                    e.flash = 6;
                    spent.push(bi);
                    if e.hp <= 0 {
                        dead.push(ei);
                    }
                    break;
                }
            }
        }
        spent.sort_unstable();
        spent.dedup();
        for i in spent.into_iter().rev() {
            self.bullets.remove(i);
        }
        dead.sort_unstable();
        dead.dedup();
        for i in dead.into_iter().rev() {
            let e = self.enemies.remove(i);
            self.score += 10;
            let r = self.rng.next_f64();
            let kind = if r < 0.25 {
                Some(Power::Coin)
            } else if r < 0.27 {
                Some(Power::Rapid)
            } else if r < 0.29 {
                Some(Power::Spread)
            } else if r < 0.31 {
                Some(Power::Boots)
            } else {
                None
            };
            if let Some(kind) = kind {
                self.pickups.push(Pickup { kind, x: e.x, y: e.y, ttl: 480 });
            }
        }
        // Pickups.
        for p in self.pickups.iter_mut() {
            p.ttl = p.ttl.saturating_sub(1);
        }
        let mut got = Vec::new();
        self.pickups.retain(|p| {
            if overlaps((px - 5.0, py - 5.0, 10.0, 10.0), (p.x - 4.0, p.y - 4.0, 8.0, 8.0)) {
                got.push(p.kind);
                false
            } else {
                p.ttl > 0
            }
        });
        for k in got {
            match k {
                Power::Coin => {
                    self.coins += 1;
                    self.score += 5;
                }
                Power::Rapid => self.rapid = POWER_TICKS,
                Power::Spread => self.spread = POWER_TICKS,
                Power::Boots => self.boots = POWER_TICKS,
            }
        }
        // Touching a critter costs a life.
        if self.invuln == 0 && self.enemies.iter().any(|e| {
            let s = e.kind.size() - 2.0;
            overlaps((px - 4.0, py - 4.0, 8.0, 8.0), (e.x - s / 2.0, e.y - s / 2.0, s, s))
        }) {
            self.lose_life();
            return;
        }
        // Round over.
        if self.timer >= ROUND_TICKS && self.enemies.is_empty() {
            self.score += 100;
            if self.round >= ROUNDS {
                self.won = true;
                self.phase = Phase::Victory;
            } else {
                self.phase = Phase::Shop;
            }
        }
    }

    /// Round progress 0..=1.
    pub fn progress(&self) -> f32 {
        (self.timer as f32 / ROUND_TICKS as f32).min(1.0)
    }
}

impl ArcadeGame for DustRanger {
    fn id(&self) -> &'static str {
        "ranger"
    }

    fn title(&self) -> &'static str {
        "Dust Ranger"
    }

    fn update(&mut self, input: &ArcadeInput) -> ArcadeStatus {
        if input.quit_pressed {
            return ArcadeStatus::Exited;
        }
        match self.phase {
            Phase::Title => {
                if input.jump_pressed || input.shoot_up || input.shoot_down || input.shoot_left || input.shoot_right {
                    self.start_round();
                }
            }
            Phase::Playing => self.play(input),
            Phase::Down(t) => {
                if t == 0 {
                    self.invuln = 120;
                    self.px = SCREEN_W / 2.0;
                    self.py = TOP + ROWS as f32 * TILE / 2.0;
                    self.spawn_in = 90;
                    self.phase = Phase::Playing;
                } else {
                    self.phase = Phase::Down(t - 1);
                }
            }
            Phase::Shop => {
                let prices = self.shop_prices();
                if let Some(k) = input.pick {
                    let i = (k - 1) as usize;
                    if self.coins >= prices[i] {
                        self.coins -= prices[i];
                        match i {
                            0 => self.damage += 1,
                            1 => self.speed_level += 1,
                            _ => self.lives += 1,
                        }
                    }
                }
                if input.jump_pressed {
                    self.round += 1;
                    self.start_round();
                }
            }
            Phase::Victory | Phase::GameOver => {
                if input.jump_pressed {
                    return ArcadeStatus::Exited;
                }
            }
        }
        ArcadeStatus::Playing
    }

    fn draw(&self, c: &mut Canvas) {
        let sand = Color::rgb(214, 180, 120);
        c.rect(0.0, 0.0, SCREEN_W, crate::SCREEN_H, Color::rgb(40, 28, 24));
        // Arena floor with speckles, fence and gates.
        c.rect(0.0, TOP, SCREEN_W, ROWS as f32 * TILE, sand);
        for y in 0..ROWS {
            for x in 0..COLS {
                let h = (x * 7919 + y * 104_729 + self.round as i32 * 31) % 11;
                if h < 3 {
                    c.rect(x as f32 * TILE + (h * 4) as f32, TOP + y as f32 * TILE + (h * 3 + 2) as f32, 2.0, 1.0, Color::rgb(190, 150, 96));
                }
            }
        }
        let fence = Color::rgb(120, 80, 50);
        let (gate_x, gate_y) = (SCREEN_W / 2.0 - 12.0, TOP + ROWS as f32 * TILE / 2.0 - 12.0);
        c.rect(0.0, TOP, gate_x, 2.0, fence);
        c.rect(gate_x + 24.0, TOP, SCREEN_W - gate_x - 24.0, 2.0, fence);
        let bottom = TOP + ROWS as f32 * TILE - 2.0;
        c.rect(0.0, bottom, gate_x, 2.0, fence);
        c.rect(gate_x + 24.0, bottom, SCREEN_W - gate_x - 24.0, 2.0, fence);
        c.rect(0.0, TOP, 2.0, gate_y - TOP, fence);
        c.rect(0.0, gate_y + 24.0, 2.0, bottom - gate_y - 24.0, fence);
        c.rect(SCREEN_W - 2.0, TOP, 2.0, gate_y - TOP, fence);
        c.rect(SCREEN_W - 2.0, gate_y + 24.0, 2.0, bottom - gate_y - 24.0, fence);
        for (rx, ry) in &self.rocks {
            c.sprite(*rx as f32 * TILE, TOP + *ry as f32 * TILE, &CACTUS, &[Color::rgb(70, 140, 70), Color::rgb(140, 200, 120), Color::rgb(150, 110, 80)], false);
        }
        // Pickups.
        for p in &self.pickups {
            if p.ttl < 120 && (p.ttl / 8) % 2 == 0 {
                continue;
            }
            let col = match p.kind {
                Power::Coin => Color::rgb(250, 210, 60),
                Power::Rapid => Color::rgb(240, 90, 70),
                Power::Spread => Color::rgb(90, 170, 250),
                Power::Boots => Color::rgb(150, 100, 60),
            };
            c.rect(p.x - 3.0, p.y - 3.0, 6.0, 6.0, col);
            c.rect(p.x - 1.0, p.y - 2.0, 2.0, 2.0, Color::rgb(255, 250, 220));
        }
        // Critters.
        for e in &self.enemies {
            let s = e.kind.size();
            let (rows, pal): (&[&str], [Color; 3]) = match e.kind {
                Critter::Bunny => (&BUNNY, [Color::rgb(200, 170, 140), Color::rgb(40, 30, 30), Color::WHITE]),
                Critter::Beetle => (&BEETLE, [Color::rgb(70, 60, 110), Color::rgb(40, 30, 40), Color::rgb(160, 150, 220)]),
                Critter::Wisp => (&WISP, [Color::rgb(200, 230, 250), Color::rgb(60, 80, 120), Color::WHITE]),
            };
            let pal = if e.flash > 0 { [Color::WHITE; 3] } else { pal };
            let bob = if (e.anim / 10) % 2 == 0 { 0.0 } else { 1.0 };
            c.sprite(e.x - s / 2.0, e.y - s / 2.0 - bob, rows, &pal, e.x > self.px);
        }
        // Ranger.
        if !matches!(self.phase, Phase::Down(_) | Phase::GameOver) && (self.invuln == 0 || (self.invuln / 4) % 2 == 0) {
            let step = if (self.walk / 8) % 2 == 0 { 0.0 } else { 1.0 };
            c.sprite(self.px - 5.0, self.py - 7.0 - step, &RANGER, &[Color::rgb(120, 80, 40), Color::rgb(240, 200, 160), Color::rgb(40, 30, 30), Color::rgb(60, 120, 170), Color::rgb(80, 60, 40)], self.facing_left);
        }
        for b in &self.bullets {
            c.rect(b.x - 1.0, b.y - 1.0, 2.0, 2.0, Color::rgb(255, 245, 200));
        }
        // HUD.
        c.rect(0.0, 0.0, SCREEN_W, TOP, Color::rgb(30, 22, 20));
        let hud = Color::rgb(240, 230, 210);
        c.text(&format!("R{}", self.round), 4.0, 4.0, 8.0, hud);
        c.text(&format!("x{}", self.lives.max(0)), 36.0, 4.0, 8.0, Color::rgb(240, 120, 110));
        c.text(&format!("${}", self.coins), 72.0, 4.0, 8.0, Color::rgb(250, 210, 60));
        c.text(&format!("{:06}", self.score), 200.0, 4.0, 8.0, hud);
        c.rect(112.0, 6.0, 80.0, 4.0, Color::rgb(70, 50, 40));
        c.rect(112.0, 6.0, 80.0 * self.progress(), 4.0, Color::rgb(120, 200, 110));
        let mid = SCREEN_W / 2.0;
        match self.phase {
            Phase::Title => {
                c.rect(28.0, 60.0, 200.0, 100.0, Color::rgba(30, 20, 20, 220));
                c.text_centered("DUST RANGER", mid, 72.0, 12.0, Color::rgb(250, 210, 120));
                c.text_centered("MOVE: W A S D", mid, 100.0, 7.0, hud);
                c.text_centered("SHOOT: ARROW KEYS", mid, 112.0, 7.0, hud);
                c.text_centered("SURVIVE 8 ROUNDS", mid, 124.0, 7.0, hud);
                c.text_centered("PRESS SPACE", mid, 144.0, 8.0, Color::rgb(250, 240, 160));
            }
            Phase::Shop => {
                let p = self.shop_prices();
                c.rect(24.0, 50.0, 208.0, 124.0, Color::rgba(30, 20, 20, 230));
                c.text_centered("TRADING POST", mid, 60.0, 10.0, Color::rgb(250, 210, 120));
                c.text(&format!("1 SHARPER SHOT  ${}", p[0]), 36.0, 86.0, 7.0, hud);
                c.text(&format!("2 QUICK BOOTS   ${}", p[1]), 36.0, 100.0, 7.0, hud);
                c.text(&format!("3 EXTRA LIFE    ${}", p[2]), 36.0, 114.0, 7.0, hud);
                c.text_centered(&format!("COINS ${}", self.coins), mid, 136.0, 8.0, Color::rgb(250, 210, 60));
                c.text_centered("SPACE: NEXT ROUND", mid, 156.0, 7.0, Color::rgb(250, 240, 160));
            }
            Phase::Down(_) => c.text_centered("OUCH!", mid, 100.0, 12.0, Color::rgb(250, 120, 100)),
            Phase::Victory => {
                c.rect(28.0, 70.0, 200.0, 80.0, Color::rgba(30, 20, 20, 230));
                c.text_centered("THE RANGE IS SAFE!", mid, 84.0, 9.0, Color::rgb(250, 210, 120));
                c.text_centered(&format!("SCORE {}", self.score), mid, 108.0, 8.0, hud);
                c.text_centered("PRESS SPACE", mid, 132.0, 7.0, Color::rgb(250, 240, 160));
            }
            Phase::GameOver => {
                c.rect(28.0, 70.0, 200.0, 80.0, Color::rgba(30, 20, 20, 230));
                c.text_centered("GAME OVER", mid, 84.0, 12.0, Color::rgb(250, 120, 100));
                c.text_centered(&format!("SCORE {}", self.score), mid, 108.0, 8.0, hud);
                c.text_centered("PRESS SPACE", mid, 132.0, 7.0, Color::rgb(250, 240, 160));
            }
            Phase::Playing => {}
        }
    }

    fn result(&self) -> ArcadeResult {
        ArcadeResult { score: self.score, won: self.won }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(g: &mut DustRanger, input: ArcadeInput, n: u32) {
        for _ in 0..n {
            g.update(&input);
        }
    }

    #[test]
    fn rounds_end_and_the_shop_sells_upgrades() {
        let mut g = DustRanger::new(3);
        tick(&mut g, ArcadeInput { jump_pressed: true, ..Default::default() }, 1);
        assert_eq!(g.phase, Phase::Playing);
        // Keep shooting in a circle until the round is over (cheat lives).
        let dirs = [(true, false, false, false), (false, true, false, false), (false, false, true, false), (false, false, false, true)];
        for t in 0..ROUND_TICKS + 3000 {
            g.lives = 3;
            let d = dirs[(t / 20 % 4) as usize];
            g.update(&ArcadeInput { shoot_up: d.0, shoot_right: d.1, shoot_down: d.2, shoot_left: d.3, ..Default::default() });
            if g.phase == Phase::Shop {
                break;
            }
            if let Phase::Down(_) = g.phase {
                tick(&mut g, ArcadeInput::default(), 100);
            }
        }
        assert_eq!(g.phase, Phase::Shop, "round ends");
        assert!(g.score >= 100);
        g.coins = 50;
        let before = g.damage;
        g.update(&ArcadeInput { pick: Some(1), ..Default::default() });
        assert_eq!(g.damage, before + 1);
        g.update(&ArcadeInput { jump_pressed: true, ..Default::default() });
        assert_eq!((g.phase, g.round), (Phase::Playing, 2));
    }

    #[test]
    fn touching_critters_costs_lives_until_game_over() {
        let mut g = DustRanger::new(5);
        g.update(&ArcadeInput { jump_pressed: true, ..Default::default() });
        let mut t = 0;
        while g.phase != Phase::GameOver && t < 60 * 600 {
            g.update(&ArcadeInput::default());
            t += 1;
        }
        assert_eq!(g.phase, Phase::GameOver);
        assert_eq!(g.update(&ArcadeInput { jump_pressed: true, ..Default::default() }), ArcadeStatus::Exited);
    }
}
