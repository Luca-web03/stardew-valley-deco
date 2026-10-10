//! Tiny procedural pixel painter for the placeholder pack.

use engine::{Color, Image};

pub fn hash(x: i32, y: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ salt.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^ (h >> 15)
}

pub fn shade(c: Color, d: i32) -> Color {
    let f = |v: u8| (v as i32 + d).clamp(0, 255) as u8;
    Color::rgba(f(c.r), f(c.g), f(c.b), c.a)
}

/// Paint helper bound to one tile cell of a sheet.
pub struct Cell<'a> {
    pub img: &'a mut Image,
    pub ox: i32,
    pub oy: i32,
    pub w: i32,
    pub h: i32,
}

impl<'a> Cell<'a> {
    pub fn at(img: &'a mut Image, index: i32, cols: i32, w: i32, h: i32) -> Self {
        Self { ox: (index % cols) * w, oy: (index / cols) * h, w, h, img }
    }
    pub fn px(&mut self, x: i32, y: i32, c: Color) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.img.set(self.ox + x, self.oy + y, c);
        }
    }
    pub fn blend(&mut self, x: i32, y: i32, c: Color) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.img.blend(self.ox + x, self.oy + y, c);
        }
    }
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.px(xx, yy, c);
            }
        }
    }
    /// Fill with base color plus hashed speckle noise.
    pub fn noise(&mut self, base: Color, amp: i32, salt: u32) {
        for y in 0..self.h {
            for x in 0..self.w {
                let n = (hash(x, y, salt) % (amp as u32 * 2 + 1)) as i32 - amp;
                self.px(x, y, shade(base, n));
            }
        }
    }
    pub fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Color) {
        for y in 0..self.h {
            for x in 0..self.w {
                let dx = (x as f32 + 0.5 - cx) / rx;
                let dy = (y as f32 + 0.5 - cy) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    self.px(x, y, c);
                }
            }
        }
    }
    pub fn outline_rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: Color) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h - 1, w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w - 1, y, 1, h, c);
    }
    /// Darken pixels adjacent to transparent ones (cheap outline).
    pub fn auto_outline(&mut self, c: Color) {
        let mut marks = Vec::new();
        for y in 0..self.h {
            for x in 0..self.w {
                if self.img.get((self.ox + x) as u32, (self.oy + y) as u32).a != 0 {
                    continue;
                }
                let n = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                    let (nx, ny) = (x + dx, y + dy);
                    nx >= 0 && ny >= 0 && nx < self.w && ny < self.h && {
                        let p = self.img.get((self.ox + nx) as u32, (self.oy + ny) as u32);
                        p.a != 0 && p != c
                    }
                });
                if n {
                    marks.push((x, y));
                }
            }
        }
        for (x, y) in marks {
            self.px(x, y, c);
        }
    }
}

pub const OUTLINE: Color = Color::rgb(40, 28, 24);
