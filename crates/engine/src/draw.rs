use crate::math::{Color, FRect, IRect, Vec2};
use crate::texture::TexId;

/// A textured quad. `dst` is in screen pixels. Rotation (radians) is applied
/// around `origin`, expressed in destination pixels relative to `dst`'s top-left.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub tex: TexId,
    pub src: IRect,
    pub dst: FRect,
    pub tint: Color,
    pub flip_x: bool,
    pub flip_y: bool,
    pub rotation: f32,
    pub origin: Vec2,
    pub depth: f32,
}

impl Sprite {
    pub fn new(tex: TexId, src: IRect, dst: FRect, depth: f32) -> Self {
        Self {
            tex,
            src,
            dst,
            tint: Color::WHITE,
            flip_x: false,
            flip_y: false,
            rotation: 0.0,
            origin: Vec2::ZERO,
            depth,
        }
    }
}

#[derive(Clone, Debug)]
pub enum DrawCmd {
    Sprite(Sprite),
    Fill { rect: FRect, color: Color, depth: f32 },
}

impl DrawCmd {
    pub fn depth(&self) -> f32 {
        match self {
            DrawCmd::Sprite(s) => s.depth,
            DrawCmd::Fill { depth, .. } => *depth,
        }
    }
}

/// Commands for one frame. Higher depth draws on top; equal depths keep
/// submission order (stable sort), matching sprite-batch semantics.
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    pub clear: Color,
    pub width: u32,
    pub height: u32,
    pub cmds: Vec<DrawCmd>,
}

impl DrawList {
    pub fn new(width: u32, height: u32) -> Self {
        Self { clear: Color::BLACK, width, height, cmds: Vec::new() }
    }

    pub fn clear(&mut self, c: Color) {
        self.clear = c;
        self.cmds.clear();
    }

    pub fn sprite(&mut self, s: Sprite) {
        self.cmds.push(DrawCmd::Sprite(s));
    }

    /// Draw `src` from `tex` at `pos` scaled uniformly by `scale`.
    pub fn draw(&mut self, tex: TexId, src: IRect, pos: Vec2, scale: f32, depth: f32) {
        self.sprite(Sprite::new(
            tex,
            src,
            FRect::new(pos.x, pos.y, src.w as f32 * scale, src.h as f32 * scale),
            depth,
        ));
    }

    pub fn draw_tinted(&mut self, tex: TexId, src: IRect, pos: Vec2, scale: f32, tint: Color, depth: f32) {
        let mut s = Sprite::new(
            tex,
            src,
            FRect::new(pos.x, pos.y, src.w as f32 * scale, src.h as f32 * scale),
            depth,
        );
        s.tint = tint;
        self.sprite(s);
    }

    pub fn fill(&mut self, rect: FRect, color: Color, depth: f32) {
        self.cmds.push(DrawCmd::Fill { rect, color, depth });
    }

    /// Outline of `t` pixels drawn inside `rect`.
    pub fn outline(&mut self, rect: FRect, t: f32, color: Color, depth: f32) {
        self.fill(FRect::new(rect.x, rect.y, rect.w, t), color, depth);
        self.fill(FRect::new(rect.x, rect.y + rect.h - t, rect.w, t), color, depth);
        self.fill(FRect::new(rect.x, rect.y, t, rect.h), color, depth);
        self.fill(FRect::new(rect.x + rect.w - t, rect.y, t, rect.h), color, depth);
    }

    /// Commands sorted for submission (stable by depth).
    pub fn sorted(&self) -> Vec<&DrawCmd> {
        let mut v: Vec<&DrawCmd> = self.cmds.iter().collect();
        v.sort_by(|a, b| a.depth().partial_cmp(&b.depth()).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}
