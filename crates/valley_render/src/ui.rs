//! UI drawing primitives in an original style: framed wooden boxes, text,
//! item slots, bars. Menus in the game crate are built from these.

use engine::font::BuiltinFont;
use engine::{Color, DrawList, FRect, IRect, Sprite, TexId, Vec2};

pub const PANEL: Color = Color::rgb(244, 214, 160);
pub const PANEL_DARK: Color = Color::rgb(214, 170, 110);
pub const FRAME: Color = Color::rgb(120, 60, 30);
pub const FRAME_LIGHT: Color = Color::rgb(190, 110, 50);
pub const TEXT: Color = Color::rgb(70, 36, 20);
pub const TEXT_LIGHT: Color = Color::rgb(255, 250, 235);
pub const HIGHLIGHT: Color = Color::rgb(255, 80, 60);

pub struct Ui<'a> {
    pub out: &'a mut DrawList,
    pub font: &'a BuiltinFont,
    pub white: TexId,
    pub depth: f32,
}

impl<'a> Ui<'a> {
    pub fn fill(&mut self, r: FRect, c: Color) {
        self.out.fill(r, c, self.depth);
    }

    /// Framed panel.
    pub fn panel(&mut self, r: FRect) {
        let d = self.depth;
        self.out.fill(FRect::new(r.x + 6.0, r.y + 6.0, r.w, r.h), Color::rgba(0, 0, 0, 70), d);
        self.out.fill(r, FRAME, d + 0.00001);
        self.out.fill(FRect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0), FRAME_LIGHT, d + 0.00002);
        self.out.fill(FRect::new(r.x + 8.0, r.y + 8.0, r.w - 16.0, r.h - 16.0), PANEL, d + 0.00003);
    }

    /// Inset slot (inventory cell).
    pub fn slot(&mut self, r: FRect, selected: bool) {
        let d = self.depth + 0.0001;
        self.out.fill(r, PANEL_DARK, d);
        self.out.fill(FRect::new(r.x + 2.0, r.y + 2.0, r.w - 4.0, r.h - 4.0), Color::rgb(232, 196, 140), d + 0.00001);
        if selected {
            self.out.outline(r, 4.0, HIGHLIGHT, d + 0.0002);
        }
    }

    pub fn text(&mut self, s: &str, pos: Vec2, scale: f32, color: Color) {
        self.font.draw(self.out, s, pos, scale, color, self.depth + 0.0005);
    }

    pub fn text_shadow(&mut self, s: &str, pos: Vec2, scale: f32, color: Color) {
        self.font.draw(self.out, s, pos + Vec2::new(scale, scale), scale, Color::rgba(0, 0, 0, 120), self.depth + 0.0004);
        self.font.draw(self.out, s, pos, scale, color, self.depth + 0.0005);
    }

    pub fn text_centered(&mut self, s: &str, center_x: f32, y: f32, scale: f32, color: Color) {
        let w = BuiltinFont::measure(s, scale).x;
        self.text(s, Vec2::new(center_x - w / 2.0, y), scale, color);
    }

    pub fn measure(&self, s: &str, scale: f32) -> Vec2 {
        BuiltinFont::measure(s, scale)
    }

    pub fn icon(&mut self, tex: TexId, src: IRect, r: FRect) {
        self.out.sprite(Sprite::new(tex, src, r, self.depth + 0.0003));
    }

    pub fn icon_tinted(&mut self, tex: TexId, src: IRect, r: FRect, tint: Color) {
        let mut s = Sprite::new(tex, src, r, self.depth + 0.0003);
        s.tint = tint;
        self.out.sprite(s);
    }

    /// Stack count in the bottom-right of a slot.
    pub fn stack_number(&mut self, n: i32, slot: FRect) {
        if n <= 1 {
            return;
        }
        let s = n.to_string();
        let w = BuiltinFont::measure(&s, 2.0).x;
        let p = Vec2::new(slot.x + slot.w - w - 4.0, slot.y + slot.h - 18.0);
        self.font.draw(self.out, &s, p + Vec2::new(2.0, 2.0), 2.0, Color::rgba(0, 0, 0, 160), self.depth + 0.0006);
        self.font.draw(self.out, &s, p, 2.0, TEXT_LIGHT, self.depth + 0.0007);
    }

    /// Wrap text to a pixel width at a given scale.
    pub fn wrap(&self, s: &str, width: f32, scale: f32) -> Vec<String> {
        let max_chars = ((width / (8.0 * scale)).floor() as usize).max(1);
        let mut lines = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split_whitespace() {
                if !line.is_empty() && line.len() + 1 + word.len() > max_chars {
                    lines.push(std::mem::take(&mut line));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
            lines.push(line);
        }
        lines
    }

    /// Vertical bar filled bottom-up (energy/health).
    pub fn vbar(&mut self, r: FRect, frac: f32, color: Color) {
        let d = self.depth;
        self.out.fill(r, FRAME, d);
        let inner = FRect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, r.h - 8.0);
        self.out.fill(inner, Color::rgb(60, 40, 30), d + 0.00001);
        let h = inner.h * frac.clamp(0.0, 1.0);
        self.out.fill(FRect::new(inner.x, inner.y + inner.h - h, inner.w, h), color, d + 0.00002);
    }

    /// Button; returns its rect for hit testing.
    pub fn button(&mut self, r: FRect, label: &str, hover: bool) -> FRect {
        let d = self.depth;
        self.out.fill(r, FRAME, d);
        self.out.fill(FRect::new(r.x + 3.0, r.y + 3.0, r.w - 6.0, r.h - 6.0), if hover { Color::rgb(255, 236, 190) } else { PANEL_DARK }, d + 0.00001);
        let scale = 2.0;
        let m = BuiltinFont::measure(label, scale);
        self.text(label, Vec2::new(r.x + (r.w - m.x) / 2.0, r.y + (r.h - 8.0 * scale) / 2.0), scale, TEXT);
        r
    }
}
