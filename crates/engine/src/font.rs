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
