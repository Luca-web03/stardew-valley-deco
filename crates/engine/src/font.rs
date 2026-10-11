//! Built-in 8x8 bitmap font (public-domain font8x8 glyphs), rasterized into a
//! texture atlas so text can be drawn through the normal sprite path.

use crate::draw::DrawList;
use crate::math::{Color, IRect, Vec2};
use crate::texture::{Image, TexId, TextureStore};

pub const GLYPH: i32 = 8;
const COLS: u32 = 16;

pub struct BuiltinFont {
    pub tex: TexId,
}

impl BuiltinFont {
    pub fn install(store: &mut TextureStore) -> Self {
        let mut img = Image::new(COLS * 8, 8 * 8);
        for (i, glyph) in font8x8::legacy::BASIC_LEGACY.iter().enumerate().take(128) {
            let gx = (i as u32 % COLS) * 8;
            let gy = (i as u32 / COLS) * 8;
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8 {
                    if bits & (1 << col) != 0 {
                        img.set((gx + col) as i32, (gy + row as u32) as i32, Color::WHITE);
                    }
                }
            }
        }
        Self { tex: store.insert("__engine/font8x8", img) }
    }

    fn src(c: char) -> IRect {
        let i = if (c as u32) < 128 { c as u32 } else { '?' as u32 };
        IRect::new(((i % COLS) * 8) as i32, ((i / COLS) * 8) as i32, 8, 8)
    }

    pub fn measure(text: &str, scale: f32) -> Vec2 {
        let mut w = 0usize;
        let mut lines = 1;
        let mut cur = 0usize;
        for c in text.chars() {
            if c == '\n' {
                lines += 1;
                w = w.max(cur);
                cur = 0;
            } else {
                cur += 1;
            }
        }
        w = w.max(cur);
        Vec2::new(w as f32 * GLYPH as f32 * scale, lines as f32 * (GLYPH as f32 + 2.0) * scale)
    }

    pub fn draw(&self, out: &mut DrawList, text: &str, pos: Vec2, scale: f32, color: Color, depth: f32) {
        let mut x = pos.x;
        let mut y = pos.y;
        for c in text.chars() {
            if c == '\n' {
                x = pos.x;
                y += (GLYPH as f32 + 2.0) * scale;
                continue;
            }
            if c != ' ' {
                out.draw_tinted(self.tex, Self::src(c), Vec2::new(x, y), scale, color, depth);
            }
            x += GLYPH as f32 * scale;
        }
    }
}

/// A bitmap font as the game ships them (XNA `SpriteFont`): glyph rects in a
/// texture, per-glyph cropping offsets and kerning (left bearing, width,
/// right bearing), line spacing and extra spacing between glyphs.
#[derive(Clone, Debug, Default)]
pub struct SpriteFontData {
    pub image: Image,
    pub glyphs: Vec<IRect>,
    pub cropping: Vec<IRect>,
    pub chars: Vec<char>,
    pub line_spacing: i32,
    pub spacing: f32,
    /// (left bearing, width, right bearing) per glyph.
    pub kerning: Vec<[f32; 3]>,
    pub default_char: Option<char>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    pub src: IRect,
    pub crop: IRect,
    pub lsb: f32,
    pub width: f32,
    pub rsb: f32,
}

#[derive(Clone, Debug)]
pub struct SpriteFont {
    pub tex: TexId,
    pub glyphs: std::collections::HashMap<char, Glyph>,
    pub line_spacing: f32,
    pub spacing: f32,
    pub default_char: Option<char>,
}

impl SpriteFont {
    pub fn install(store: &mut TextureStore, name: &str, data: SpriteFontData) -> Self {
        let mut glyphs = std::collections::HashMap::new();
        for (i, c) in data.chars.iter().enumerate() {
            let src = data.glyphs.get(i).copied().unwrap_or(IRect::new(0, 0, 0, 0));
            let crop = data.cropping.get(i).copied().unwrap_or(IRect::new(0, 0, src.w, src.h));
            let k = data.kerning.get(i).copied().unwrap_or([0.0, src.w as f32, 0.0]);
            glyphs.insert(*c, Glyph { src, crop, lsb: k[0], width: k[1], rsb: k[2] });
        }
        let tex = store.insert(name, data.image);
        Self { tex, glyphs, line_spacing: data.line_spacing as f32, spacing: data.spacing, default_char: data.default_char }
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(&c).or_else(|| self.default_char.and_then(|d| self.glyphs.get(&d))).or_else(|| self.glyphs.get(&'?'))
    }

    /// Size of `text` (XNA `MeasureString`).
    pub fn measure(&self, text: &str, scale: f32) -> Vec2 {
        let (mut width, mut x, mut y) = (0.0f32, 0.0f32, 0.0f32);
        let mut line_h = self.line_spacing;
        let mut first = true;
        for c in text.chars() {
            match c {
                '\r' => continue,
                '\n' => {
                    line_h = self.line_spacing;
                    x = 0.0;
                    y += self.line_spacing;
                    first = true;
                    continue;
                }
                _ => {}
            }
            let Some(g) = self.glyph(c) else { continue };
            if first {
                x = g.lsb.max(0.0);
                first = false;
            } else {
                x += self.spacing + g.lsb;
            }
            x += g.width;
            width = width.max(x + g.rsb.max(0.0));
            x += g.rsb;
            line_h = line_h.max(g.crop.h as f32);
        }
        Vec2::new(width * scale, (y + line_h) * scale)
    }

    /// Draw `text` (XNA `DrawString` layout).
    pub fn draw(&self, out: &mut DrawList, text: &str, pos: Vec2, scale: f32, color: Color, depth: f32) {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        let mut first = true;
        for c in text.chars() {
            match c {
                '\r' => continue,
                '\n' => {
                    x = 0.0;
                    y += self.line_spacing;
                    first = true;
                    continue;
                }
                _ => {}
            }
            let Some(g) = self.glyph(c).copied() else { continue };
            if first {
                x = g.lsb.max(0.0);
                first = false;
            } else {
                x += self.spacing + g.lsb;
            }
            if g.src.w > 0 && g.src.h > 0 {
                let at = Vec2::new(pos.x + (x + g.crop.x as f32) * scale, pos.y + (y + g.crop.y as f32) * scale);
                out.draw_tinted(self.tex, g.src, at, scale, color, depth);
            }
            x += g.width + g.rsb;
        }
    }
}

/// The font text is drawn with: the game's own sprite font when available
/// (scaled so a line advances as far as the built-in font's), else the
/// built-in 8x8 font.
pub enum Font {
    Builtin(BuiltinFont),
    /// `builtin` stays available for fixed-cell text (arcade screens).
    Sprite { font: SpriteFont, unit: f32, builtin: BuiltinFont },
}

impl Font {
    pub fn sprite(font: SpriteFont, builtin: BuiltinFont) -> Self {
        let unit = (GLYPH as f32 + 2.0) / font.line_spacing.max(1.0);
        Font::Sprite { font, unit, builtin }
    }

    /// The built-in 8x8 font atlas.
    pub fn builtin_tex(&self) -> TexId {
        match self {
            Font::Builtin(b) | Font::Sprite { builtin: b, .. } => b.tex,
        }
    }

    pub fn measure(&self, text: &str, scale: f32) -> Vec2 {
        match self {
            Font::Builtin(_) => BuiltinFont::measure(text, scale),
            Font::Sprite { font, unit, .. } => font.measure(text, scale * unit),
        }
    }

    pub fn draw(&self, out: &mut DrawList, text: &str, pos: Vec2, scale: f32, color: Color, depth: f32) {
        match self {
            Font::Builtin(f) => f.draw(out, text, pos, scale, color, depth),
            Font::Sprite { font, unit, .. } => font.draw(out, text, pos, scale * unit, color, depth),
        }
    }

    /// Height of one line of glyphs (for vertical centering).
    pub fn line_height(&self, scale: f32) -> f32 {
        match self {
            Font::Builtin(_) => GLYPH as f32 * scale,
            Font::Sprite { font, unit, .. } => font.line_spacing * unit * scale * 0.8,
        }
    }

    /// Greedy word wrap to a pixel width.
    pub fn wrap(&self, s: &str, width: f32, scale: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split_whitespace() {
                let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if !line.is_empty() && self.measure(&candidate, scale).x > width {
                    lines.push(std::mem::replace(&mut line, word.to_string()));
                } else {
                    line = candidate;
                }
            }
            lines.push(line);
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SpriteFont {
        let mut store = TextureStore::default();
        let data = SpriteFontData {
            image: Image::new(32, 16),
            glyphs: vec![IRect::new(0, 0, 6, 10), IRect::new(8, 0, 4, 10), IRect::new(0, 0, 0, 0)],
            cropping: vec![IRect::new(0, 2, 6, 12), IRect::new(1, 2, 4, 12), IRect::new(0, 0, 0, 12)],
            chars: vec!['A', 'i', ' '],
            line_spacing: 16,
            spacing: 1.0,
            kerning: vec![[1.0, 6.0, 1.0], [0.0, 4.0, 1.0], [0.0, 4.0, 0.0]],
            default_char: Some('A'),
        };
        SpriteFont::install(&mut store, "test", data)
    }

    #[test]
    fn measures_like_xna() {
        let f = sample();
        // "Ai": A: x=1 (first, lsb), +6 => 7, width=max(7+1)=8, x=8; i: x += 1+0 => 9, +4 => 13, width 14, x=14.
        assert_eq!(f.measure("Ai", 1.0), Vec2::new(14.0, 16.0));
        assert_eq!(f.measure("A\nA", 2.0), Vec2::new(16.0, 64.0));
        // Unknown characters use the default.
        assert_eq!(f.measure("?", 1.0), f.measure("A", 1.0));
    }

    #[test]
    fn wraps_by_measured_width() {
        let f = Font::sprite(sample(), BuiltinFont { tex: TexId(0) });
        let lines = f.wrap("Ai Ai Ai", f.measure("Ai Ai", 1.0).x + 0.5, 1.0);
        assert_eq!(lines, vec!["Ai Ai".to_string(), "Ai".to_string()]);
    }
}
