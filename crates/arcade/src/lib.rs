//! Arcade cabinets: two original minigames drawn on a small virtual screen.
//!
//! * [`ranger::DustRanger`]: a top-down arena shooter (move with WASD,
//!   shoot with the arrow keys).
//! * [`rails::RailRunner`]: a side-scrolling cart runner (jump with Space,
//!   W, Up or C; hold for a higher jump).
//!
//! Games implement [`ArcadeGame`] and only need an [`ArcadeInput`] per tick
//! and a [`Canvas`] to draw on, so they can run inside another game's menu
//! or as their own engine module ([`ArcadeModule`]).

pub mod rails;
pub mod ranger;

use engine::font::BuiltinFont;
use engine::{Color, DrawList, EngineCtx, FRect, GameModule, InputState, Key, TexId, Vec2};

/// Virtual screen size in arcade pixels.
pub const SCREEN_W: f32 = 256.0;
pub const SCREEN_H: f32 = 224.0;

/// Buttons for one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArcadeInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub shoot_up: bool,
    pub shoot_down: bool,
    pub shoot_left: bool,
    pub shoot_right: bool,
    /// Jump / confirm held.
    pub jump: bool,
    /// Jump / confirm pressed this tick.
    pub jump_pressed: bool,
    pub quit_pressed: bool,
    /// Number keys 1..=3 pressed this tick (shop choices).
    pub pick: Option<u8>,
}

impl ArcadeInput {
    /// Fixed cabinet mapping: WASD moves, arrows shoot, Space/C/Enter jump.
    pub fn from_input(i: &InputState) -> Self {
        let pick = [Key::Num1, Key::Num2, Key::Num3].iter().position(|k| i.pressed(*k)).map(|p| p as u8 + 1);
        Self {
            up: i.held(Key::W),
            down: i.held(Key::S),
            left: i.held(Key::A),
            right: i.held(Key::D),
            shoot_up: i.held(Key::Up),
            shoot_down: i.held(Key::Down),
            shoot_left: i.held(Key::Left),
            shoot_right: i.held(Key::Right),
            jump: i.held(Key::Space) || i.held(Key::C) || i.held(Key::Enter) || i.held(Key::W) || i.held(Key::Up),
            jump_pressed: i.pressed(Key::Space) || i.pressed(Key::C) || i.pressed(Key::Enter) || i.pressed(Key::W) || i.pressed(Key::Up),
            quit_pressed: i.pressed(Key::Escape),
            pick,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArcadeStatus {
    Playing,
    /// The player left (Escape) or the game ended and was dismissed.
    Exited,
}

/// Results reported when a game is closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArcadeResult {
    pub score: i32,
    pub won: bool,
}

pub trait ArcadeGame {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    /// One 1/60 s tick.
    fn update(&mut self, input: &ArcadeInput) -> ArcadeStatus;
    fn draw(&self, c: &mut Canvas);
    fn result(&self) -> ArcadeResult;
}

/// Create a game by id ("ranger" or "rails").
pub fn create(id: &str, seed: u64) -> Option<Box<dyn ArcadeGame>> {
    match id {
        "ranger" | "Arcade_Prairie" => Some(Box::new(ranger::DustRanger::new(seed))),
        "rails" | "Arcade_Minecart" => Some(Box::new(rails::RailRunner::new(seed))),
        _ => None,
    }
}

/// Drawing surface: arcade pixels mapped into a screen rectangle.
pub struct Canvas<'a> {
    pub out: &'a mut DrawList,
    pub origin: Vec2,
    pub scale: f32,
    pub depth: f32,
    pub font: Option<TexId>,
}

impl<'a> Canvas<'a> {
    /// Fit the virtual screen into `area`, centred, integer scale if possible.
    pub fn fit(out: &'a mut DrawList, area: FRect, depth: f32, font: Option<TexId>) -> Self {
        let s = (area.w / SCREEN_W).min(area.h / SCREEN_H);
        let scale = if s >= 1.0 { s.floor() } else { s };
        let origin = Vec2::new(area.x + (area.w - SCREEN_W * scale) / 2.0, area.y + (area.h - SCREEN_H * scale) / 2.0);
        Self { out, origin, scale, depth, font }
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        // Clip to the virtual screen.
        let (x0, y0) = (x.max(0.0), y.max(0.0));
        let (x1, y1) = ((x + w).min(SCREEN_W), (y + h).min(SCREEN_H));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (x, y, w, h) = (x0, y0, x1 - x0, y1 - y0);
        let r = FRect::new(self.origin.x + x * self.scale, self.origin.y + y * self.scale, w * self.scale, h * self.scale);
        self.out.fill(r, c, self.depth);
        self.depth += 1e-7;
    }

    /// A small sprite from text rows: each char indexes `palette` by
    /// position in "123456789"; '.' is transparent.
    pub fn sprite(&mut self, x: f32, y: f32, rows: &[&str], palette: &[Color], flip: bool) {
        for (ry, row) in rows.iter().enumerate() {
            let n = row.len();
            // Merge horizontal runs of one colour into one rect.
            let bytes = row.as_bytes();
            let mut i = 0;
            while i < n {
                let ch = bytes[i];
                let mut j = i + 1;
                while j < n && bytes[j] == ch {
                    j += 1;
                }
                if ch != b'.' {
                    let idx = (ch - b'1') as usize;
                    if let Some(col) = palette.get(idx) {
                        let x0 = if flip { (n - j) as f32 } else { i as f32 };
                        self.rect(x + x0, y + ry as f32, (j - i) as f32, 1.0, *col);
                    }
                }
                i = j;
            }
        }
    }

    pub fn text(&mut self, s: &str, x: f32, y: f32, size: f32, c: Color) {
        let Some(font) = self.font else { return };
        let f = BuiltinFont { tex: font };
        f.draw(self.out, s, Vec2::new(self.origin.x + x * self.scale, self.origin.y + y * self.scale), size * self.scale / 8.0, c, self.depth + 1e-4);
    }

    pub fn text_centered(&mut self, s: &str, cx: f32, y: f32, size: f32, c: Color) {
        let w = s.chars().count() as f32 * size;
        self.text(s, cx - w / 2.0, y, size, c);
    }
}

/// Runs one arcade game as a standalone engine module (restarts after
/// Escape; Ctrl+Tab still switches modules).
pub struct ArcadeModule {
    id: String,
    game_id: &'static str,
    game: Box<dyn ArcadeGame>,
    font: Option<TexId>,
    seed: u64,
    pub best: i32,
}

impl ArcadeModule {
    pub fn new(game_id: &'static str) -> Self {
        let game = create(game_id, 1).expect("known arcade game");
        Self { id: format!("arcade_{game_id}"), game_id, game, font: None, seed: 1, best: 0 }
    }
}

impl GameModule for ArcadeModule {
    fn id(&self) -> &str {
        &self.id
    }

    fn init(&mut self, ctx: &mut EngineCtx) {
        self.font = ctx.textures.id("__engine/font8x8").or_else(|| Some(BuiltinFont::install(&mut ctx.textures).tex));
    }

    fn update(&mut self, ctx: &mut EngineCtx) {
        let input = ArcadeInput::from_input(&ctx.input);
        if self.game.update(&input) == ArcadeStatus::Exited {
            self.best = self.best.max(self.game.result().score);
            self.seed += 1;
            self.game = create(self.game_id, self.seed).expect("known arcade game");
        }
    }

    fn draw(&mut self, ctx: &mut EngineCtx, out: &mut DrawList) {
        out.fill(FRect::new(0.0, 0.0, ctx.screen_w as f32, ctx.screen_h as f32), Color::rgb(10, 10, 14), 0.0);
        let mut c = Canvas::fit(out, FRect::new(0.0, 0.0, ctx.screen_w as f32, ctx.screen_h as f32), 0.1, self.font);
        self.game.draw(&mut c);
        if self.best > 0 {
            let best = format!("BEST {}", self.best);
            c.text(&best, 4.0, SCREEN_H - 10.0, 6.0, Color::rgb(160, 160, 170));
        }
    }

    fn command(&mut self, _ctx: &mut EngineCtx, cmd: &str) -> String {
        match cmd.trim() {
            "restart" => {
                self.seed += 1;
                self.game = create(self.game_id, self.seed).expect("known arcade game");
                "restarted".into()
            }
            _ => "commands: restart".into(),
        }
    }
}

/// Tiny helpers shared by the games.
pub(crate) fn approach(v: f32, target: f32, step: f32) -> f32 {
    if v < target {
        (v + step).min(target)
    } else {
        (v - step).max(target)
    }
}

pub(crate) fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}
