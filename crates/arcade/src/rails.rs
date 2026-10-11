//! "Rail Runner": an original side-scrolling cart game. The cart rolls on
//! by itself; jump over gaps and up ledges (hold to jump higher), grab gems,
//! and reach the end of five tracks through different caverns.

use engine::{Color, Rng};

use crate::{approach, overlaps, ArcadeGame, ArcadeInput, ArcadeResult, ArcadeStatus, Canvas, SCREEN_H, SCREEN_W};

const COL: f32 = 16.0;
const CART_X: f32 = 56.0;
const CART_W: f32 = 16.0;
const CART_H: f32 = 12.0;
pub const TRACKS: u32 = 5;
const GRAVITY: f32 = 0.32;
const HOLD_GRAVITY: f32 = 0.16;
const JUMP: f32 = -5.0;
const HOLD_TICKS: u32 = 14;
const CHECKPOINT: usize = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Title,
    Riding,
    /// Crashed; respawn at the last checkpoint.
    Crashed(u32),
    /// Track finished; short pause.
    Cleared(u32),
    Victory,
    GameOver,
}

struct Theme {
    name: &'static str,
    sky: Color,
    far: Color,
    rail: Color,
    rock: Color,
}

const THEMES: [Theme; 5] = [
    Theme { name: "OLD MINE", sky: Color::rgb(30, 22, 18), far: Color::rgb(60, 44, 34), rail: Color::rgb(150, 110, 70), rock: Color::rgb(100, 72, 52) },
    Theme { name: "CRYSTAL HALL", sky: Color::rgb(16, 22, 44), far: Color::rgb(40, 60, 110), rail: Color::rgb(140, 170, 220), rock: Color::rgb(60, 80, 130) },
    Theme { name: "EMBER DEEP", sky: Color::rgb(40, 12, 10), far: Color::rgb(90, 30, 20), rail: Color::rgb(200, 110, 60), rock: Color::rgb(110, 40, 30) },
    Theme { name: "MOSS TUNNEL", sky: Color::rgb(14, 30, 20), far: Color::rgb(30, 70, 44), rail: Color::rgb(120, 170, 100), rock: Color::rgb(50, 90, 60) },
    Theme { name: "SKY TRESTLE", sky: Color::rgb(120, 170, 220), far: Color::rgb(200, 220, 240), rail: Color::rgb(120, 90, 60), rock: Color::rgb(140, 110, 80) },
];

pub struct RailRunner {
    rng: Rng,
    pub phase: Phase,
    pub track: u32,
    pub lives: i32,
    pub gems: i32,
    pub score: i32,
    /// Track top (arcade y) per column; None = gap.
    cols: Vec<Option<f32>>,
    gem_list: Vec<(f32, f32, bool)>,
    /// Scroll position (world x of the screen's left edge).
    pub scroll: f32,
    speed: f32,
    y: f32,
    vy: f32,
    grounded: bool,
    hold: u32,
    wheel: u32,
    won: bool,
}

impl RailRunner {
    pub fn new(seed: u64) -> Self {
        let mut g = Self {
            rng: Rng::new(seed ^ 0x7a11),
            phase: Phase::Title,
            track: 1,
            lives: 3,
            gems: 0,
            score: 0,
            cols: Vec::new(),
            gem_list: Vec::new(),
            scroll: 0.0,
            speed: 2.0,
            y: 0.0,
            vy: 0.0,
            grounded: true,
            hold: 0,
            wheel: 0,
            won: false,
        };
        g.build_track();
        g
    }

    /// Lay out the current track: flat runs, ledges and gaps.
    fn build_track(&mut self) {
        let len = 150 + self.track as usize * 30;
        let mut cols = Vec::with_capacity(len + 20);
        let mut gems = Vec::new();
        let mut h = 160.0;
        // A safe start.
        for _ in 0..12 {
            cols.push(Some(h));
        }
        while cols.len() < len {
            let r = self.rng.next_f64();
            let level = self.track as f64;
            if r < 0.3 + level * 0.03 {
                // Gap with gems arcing over it.
                let w = self.rng.range(2, 3 + (self.track as i32 / 2).min(2)) as usize;
                let start = cols.len();
                for _ in 0..w {
                    cols.push(None);
                }
                for k in 0..w + 1 {
                    let x = (start + k) as f32 * COL;
                    let t = k as f32 / w as f32;
                    gems.push((x, h - 26.0 - (t * std::f32::consts::PI).sin() * 22.0, false));
                }
            } else if r < 0.55 {
                // Ledge up or down.
                let dh = if self.rng.chance(0.5) { -16.0 } else { 16.0 } * if self.rng.chance(0.3) { 2.0 } else { 1.0 };
                h = (h + dh).clamp(112.0, 192.0);
            }
            let run = self.rng.range(3, 9 - (self.track as i32).min(4)) as usize;
            for k in 0..run {
                cols.push(Some(h));
                if k == run / 2 && self.rng.chance(0.4) {
                    gems.push((cols.len() as f32 * COL - 8.0, h - 18.0, false));
                }
            }
        }
        // Station at the end.
        for _ in 0..20 {
            cols.push(Some(h));
        }
        self.cols = cols;
        self.gem_list = gems;
        self.speed = 2.0 + self.track as f32 * 0.3;
        self.respawn(0);
    }

    fn respawn(&mut self, col: usize) {
        // Back up to solid ground.
        let mut c = col;
        while c > 0 && (self.cols.get(c).copied().flatten().is_none() || self.cols.get(c + 1).copied().flatten().is_none()) {
            c -= 1;
        }
        self.scroll = c as f32 * COL - CART_X;
        let top = self.cols.get(c).copied().flatten().unwrap_or(160.0);
        self.y = top - CART_H;
        self.vy = 0.0;
        self.grounded = true;
    }

    fn column_top(&self, world_x: f32) -> Option<f32> {
        if world_x < 0.0 {
            return self.cols.first().copied().flatten();
        }
        self.cols.get((world_x / COL) as usize).copied().flatten()
    }

    /// Highest track top under the cart's span (None over a gap).
    fn floor_under(&self, wx: f32) -> Option<f32> {
        let a = self.column_top(wx + 2.0);
        let b = self.column_top(wx + CART_W - 2.0);
        match (a, b) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    fn crash(&mut self) {
        self.lives -= 1;
        self.phase = if self.lives < 0 { Phase::GameOver } else { Phase::Crashed(60) };
    }

    fn ride(&mut self, input: &ArcadeInput) {
        self.wheel += 1;
        let wx = self.scroll + CART_X;
        // Jump.
        if self.grounded && input.jump_pressed {
            self.vy = JUMP;
            self.grounded = false;
            self.hold = 0;
        }
        // Advance; a ledge higher than the cart is a wall.
        let next_wx = wx + self.speed;
        let ahead = self.column_top(next_wx + CART_W - 2.0);
        if let Some(top) = ahead {
            if self.y + CART_H > top + 4.0 && self.column_top(wx + CART_W - 2.0).map_or(true, |cur| top < cur) {
                self.crash();
                return;
            }
        }
        self.scroll += self.speed;
        let wx = next_wx;
        // Vertical motion.
        if !self.grounded {
            let g = if input.jump && self.vy < 0.0 && self.hold < HOLD_TICKS { HOLD_GRAVITY } else { GRAVITY };
            self.hold += 1;
            self.vy = approach(self.vy + g, 7.0, 0.0).min(7.0);
            let prev_bottom = self.y + CART_H;
            self.y += self.vy;
            if let Some(top) = self.floor_under(wx) {
                if self.vy >= 0.0 && prev_bottom <= top + 1.0 && self.y + CART_H >= top {
                    self.y = top - CART_H;
                    self.vy = 0.0;
                    self.grounded = true;
                }
            }
        } else {
            match self.floor_under(wx) {
                Some(top) => {
                    // Roll down small drops.
                    if top > self.y + CART_H + 0.5 {
                        self.grounded = false;
                        self.vy = 0.0;
                    } else {
                        self.y = top - CART_H;
                    }
                }
                None => {
                    self.grounded = false;
                    self.vy = 0.0;
                }
            }
        }
        if self.y > SCREEN_H + 16.0 {
            self.crash();
            return;
        }
        // Gems.
        let cart = (wx, self.y, CART_W, CART_H);
        let mut got = 0;
        for g in self.gem_list.iter_mut() {
            if !g.2 && overlaps(cart, (g.0 - 3.0, g.1 - 3.0, 6.0, 6.0)) {
                g.2 = true;
                got += 1;
            }
        }
        self.gems += got;
        self.score += got * 10;
        // End of the line.
        if wx >= (self.cols.len() - 12) as f32 * COL {
            self.score += 200 + (self.scroll / COL) as i32;
            self.phase = Phase::Cleared(120);
        }
    }

    pub fn progress(&self) -> f32 {
        ((self.scroll + CART_X) / ((self.cols.len() - 12) as f32 * COL)).clamp(0.0, 1.0)
    }
}

impl ArcadeGame for RailRunner {
    fn id(&self) -> &'static str {
        "rails"
    }

    fn title(&self) -> &'static str {
        "Rail Runner"
    }

    fn update(&mut self, input: &ArcadeInput) -> ArcadeStatus {
        if input.quit_pressed {
            return ArcadeStatus::Exited;
        }
        match self.phase {
            Phase::Title => {
                if input.jump_pressed {
                    self.phase = Phase::Riding;
                }
            }
            Phase::Riding => self.ride(input),
            Phase::Crashed(t) => {
                if t == 0 {
                    let col = ((self.scroll + CART_X) / COL) as usize;
                    self.respawn(col / CHECKPOINT * CHECKPOINT);
                    self.phase = Phase::Riding;
                } else {
                    self.phase = Phase::Crashed(t - 1);
                }
            }
            Phase::Cleared(t) => {
                if t == 0 {
                    if self.track >= TRACKS {
                        self.won = true;
                        self.phase = Phase::Victory;
                    } else {
                        self.track += 1;
                        self.build_track();
                        self.phase = Phase::Riding;
                    }
                } else {
                    self.phase = Phase::Cleared(t - 1);
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
        let th = &THEMES[((self.track - 1) as usize).min(THEMES.len() - 1)];
        c.rect(0.0, 0.0, SCREEN_W, SCREEN_H, th.sky);
        // Parallax pillars.
        let far = self.scroll * 0.3;
        for k in -1..8 {
            let x = k as f32 * 48.0 - far % 48.0;
            let hgt = 40.0 + ((k + (far / 48.0) as i32) * 37).rem_euclid(50) as f32;
            c.rect(x, SCREEN_H - 40.0 - hgt, 28.0, hgt + 40.0, th.far);
        }
        // Track columns.
        let first = (self.scroll / COL).floor() as i32 - 1;
        for i in first..first + (SCREEN_W / COL) as i32 + 3 {
            if i < 0 {
                continue;
            }
            let Some(Some(top)) = self.cols.get(i as usize) else { continue };
            let x = i as f32 * COL - self.scroll;
            c.rect(x, *top + 4.0, COL, SCREEN_H - top, th.rock);
            c.rect(x, *top, COL, 2.0, th.rail);
            c.rect(x + 3.0, *top + 2.0, 3.0, 3.0, Color::rgb(90, 60, 40));
            c.rect(x + 10.0, *top + 2.0, 3.0, 3.0, Color::rgb(90, 60, 40));
        }
        // Checkpoint flags.
        for i in (CHECKPOINT..self.cols.len()).step_by(CHECKPOINT) {
            let x = i as f32 * COL - self.scroll;
            if (-8.0..SCREEN_W).contains(&x) {
                if let Some(Some(top)) = self.cols.get(i) {
                    c.rect(x, top - 20.0, 1.0, 20.0, Color::rgb(220, 220, 220));
                    c.rect(x + 1.0, top - 20.0, 6.0, 4.0, Color::rgb(230, 80, 70));
                }
            }
        }
        // Gems.
        for g in &self.gem_list {
            let x = g.0 - self.scroll;
            if g.2 || !(-8.0..SCREEN_W + 8.0).contains(&x) {
                continue;
            }
            let tw = ((self.wheel / 6) % 3) as f32;
            c.rect(x - 3.0, g.1 - 3.0, 6.0, 6.0, Color::rgb(110, 230, 200));
            c.rect(x - 1.0, g.1 - 2.0 + tw * 0.5, 2.0, 2.0, Color::WHITE);
        }
        // Cart.
        if !matches!(self.phase, Phase::Crashed(t) if (t / 4) % 2 == 0) && self.phase != Phase::GameOver {
            let y = self.y;
            c.rect(CART_X, y + 2.0, CART_W, 7.0, Color::rgb(150, 150, 160));
            c.rect(CART_X, y + 2.0, CART_W, 2.0, Color::rgb(200, 200, 210));
            c.rect(CART_X + 5.0, y - 6.0, 6.0, 8.0, Color::rgb(70, 120, 170));
            c.rect(CART_X + 5.0, y - 10.0, 6.0, 4.0, Color::rgb(240, 200, 160));
            c.rect(CART_X + 4.0, y - 11.0, 8.0, 2.0, Color::rgb(160, 60, 50));
            let spin = (self.wheel / 4) % 2 == 0;
            for wx in [CART_X + 2.0, CART_X + 10.0] {
                c.rect(wx, y + 9.0, 4.0, 3.0, Color::rgb(50, 50, 55));
                if spin {
                    c.rect(wx + 1.0, y + 10.0, 2.0, 1.0, Color::rgb(160, 160, 170));
                }
            }
        }
        // HUD.
        let hud = Color::rgb(240, 235, 220);
        c.rect(0.0, 0.0, SCREEN_W, 14.0, Color::rgba(0, 0, 0, 150));
        c.text(&format!("T{}", self.track), 4.0, 3.0, 8.0, hud);
        c.text(&format!("x{}", self.lives.max(0)), 32.0, 3.0, 8.0, Color::rgb(240, 120, 110));
        c.text(&format!("*{}", self.gems), 64.0, 3.0, 8.0, Color::rgb(110, 230, 200));
        c.text(&format!("{:06}", self.score), 200.0, 3.0, 8.0, hud);
        c.rect(108.0, 5.0, 84.0, 4.0, Color::rgb(60, 60, 60));
        c.rect(108.0, 5.0, 84.0 * self.progress(), 4.0, Color::rgb(240, 200, 90));
        let mid = SCREEN_W / 2.0;
        match self.phase {
            Phase::Title => {
                c.rect(28.0, 60.0, 200.0, 96.0, Color::rgba(0, 0, 0, 200));
                c.text_centered("RAIL RUNNER", mid, 72.0, 12.0, Color::rgb(240, 200, 90));
                c.text_centered("JUMP: SPACE (HOLD = HIGHER)", mid, 100.0, 6.0, hud);
                c.text_centered("REACH THE END OF 5 TRACKS", mid, 112.0, 6.0, hud);
                c.text_centered("PRESS SPACE", mid, 136.0, 8.0, Color::rgb(250, 240, 160));
            }
            Phase::Cleared(_) => {
                let label = if self.track >= TRACKS { "LAST STOP!".to_string() } else { format!("{} CLEAR!", th.name) };
                c.text_centered(&label, mid, 90.0, 9.0, Color::rgb(250, 240, 160));
            }
            Phase::Crashed(_) => c.text_centered("CRASH!", mid, 90.0, 12.0, Color::rgb(250, 120, 100)),
            Phase::Victory => {
                c.rect(28.0, 70.0, 200.0, 80.0, Color::rgba(0, 0, 0, 210));
                c.text_centered("END OF THE LINE!", mid, 84.0, 9.0, Color::rgb(240, 200, 90));
                c.text_centered(&format!("SCORE {}", self.score), mid, 108.0, 8.0, hud);
                c.text_centered("PRESS SPACE", mid, 132.0, 7.0, Color::rgb(250, 240, 160));
            }
            Phase::GameOver => {
                c.rect(28.0, 70.0, 200.0, 80.0, Color::rgba(0, 0, 0, 210));
                c.text_centered("GAME OVER", mid, 84.0, 12.0, Color::rgb(250, 120, 100));
                c.text_centered(&format!("SCORE {}", self.score), mid, 108.0, 8.0, hud);
                c.text_centered("PRESS SPACE", mid, 132.0, 7.0, Color::rgb(250, 240, 160));
            }
            Phase::Riding => {
                if self.progress() < 0.05 {
                    c.text_centered(th.name, mid, 40.0, 9.0, hud);
                }
            }
        }
    }

    fn result(&self) -> ArcadeResult {
        ArcadeResult { score: self.score, won: self.won }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple bot: jump when the column ahead is a gap or a higher ledge.
    fn bot(g: &RailRunner) -> ArcadeInput {
        let wx = g.scroll + CART_X;
        let mut jump = false;
        for k in 1..4 {
            let ahead = g.column_top(wx + CART_W + k as f32 * COL * 0.75);
            match ahead {
                None => jump = true,
                Some(t) if t < g.y + CART_H - 1.0 => jump = true,
                _ => {}
            }
        }
        ArcadeInput { jump_pressed: jump, jump, ..Default::default() }
    }

    #[test]
    fn falling_into_a_gap_costs_a_life() {
        let mut g = RailRunner::new(9);
        g.update(&ArcadeInput { jump_pressed: true, ..Default::default() });
        let lives = g.lives;
        for _ in 0..3000 {
            g.update(&ArcadeInput::default());
            if g.lives < lives {
                break;
            }
        }
        assert!(g.lives < lives, "never jumping crashes eventually");
    }

    #[test]
    fn a_careful_rider_finishes_tracks() {
        let mut g = RailRunner::new(4);
        g.update(&ArcadeInput { jump_pressed: true, ..Default::default() });
        let mut t = 0;
        while g.track < 2 && t < 60 * 120 {
            g.lives = 3;
            let i = bot(&g);
            g.update(&i);
            t += 1;
        }
        assert!(g.track >= 2, "reached track 2 (track {}, progress {})", g.track, g.progress());
        assert!(g.score > 200);
    }
}
