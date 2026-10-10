//! Reference software renderer: rasterizes a [`DrawList`] into an [`Image`].
//!
//! Used for headless runs, golden-image tests and screenshots. Sampling is
//! nearest-neighbour (pixel art), blending is straight-alpha "over".

use crate::draw::{DrawCmd, DrawList, Sprite};
use crate::math::Color;
use crate::texture::{Image, TextureStore};

pub fn render(list: &DrawList, textures: &TextureStore) -> Image {
    let mut fb = Image::filled(list.width.max(1), list.height.max(1), list.clear);
    for cmd in list.sorted() {
        match cmd {
            DrawCmd::Fill { rect, color, .. } => {
                let x0 = rect.x.round() as i32;
                let y0 = rect.y.round() as i32;
                let x1 = (rect.x + rect.w).round() as i32;
                let y1 = (rect.y + rect.h).round() as i32;
                for y in y0.max(0)..y1.min(fb.height as i32) {
                    for x in x0.max(0)..x1.min(fb.width as i32) {
                        fb.blend(x, y, *color);
                    }
                }
            }
            DrawCmd::Sprite(s) => draw_sprite(&mut fb, s, textures.get(s.tex)),
        }
    }
    fb
}

fn draw_sprite(fb: &mut Image, s: &Sprite, tex: &Image) {
    if s.dst.w <= 0.0 || s.dst.h <= 0.0 || s.src.w <= 0 || s.src.h <= 0 {
        return;
    }
    let (sin, cos) = s.rotation.sin_cos();
    // Pivot in screen space.
    let px = s.dst.x + s.origin.x;
    let py = s.dst.y + s.origin.y;
    // Bounding box of the transformed quad.
    let corners = [(0.0, 0.0), (s.dst.w, 0.0), (0.0, s.dst.h), (s.dst.w, s.dst.h)];
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (cx, cy) in corners {
        let lx = cx - s.origin.x;
        let ly = cy - s.origin.y;
        let x = px + lx * cos - ly * sin;
        let y = py + lx * sin + ly * cos;
        minx = minx.min(x);
        miny = miny.min(y);
        maxx = maxx.max(x);
        maxy = maxy.max(y);
    }
    let x0 = (minx.floor() as i32).max(0);
    let y0 = (miny.floor() as i32).max(0);
    let x1 = (maxx.ceil() as i32).min(fb.width as i32);
    let y1 = (maxy.ceil() as i32).min(fb.height as i32);
    let sx = s.src.w as f32 / s.dst.w;
    let sy = s.src.h as f32 / s.dst.h;
    for y in y0..y1 {
        for x in x0..x1 {
            // Inverse transform the pixel center into local quad space.
            let dx = x as f32 + 0.5 - px;
            let dy = y as f32 + 0.5 - py;
            let lx = dx * cos + dy * sin + s.origin.x;
            let ly = -dx * sin + dy * cos + s.origin.y;
            if lx < 0.0 || ly < 0.0 || lx >= s.dst.w || ly >= s.dst.h {
                continue;
            }
            let mut u = (lx * sx) as i32;
            let mut v = (ly * sy) as i32;
            if s.flip_x {
                u = s.src.w - 1 - u;
            }
            if s.flip_y {
                v = s.src.h - 1 - v;
            }
            let tx = s.src.x + u;
            let ty = s.src.y + v;
            if tx < 0 || ty < 0 {
                continue;
            }
            let c = tex.get(tx as u32, ty as u32);
            if c.a == 0 {
                continue;
            }
            let c = if s.tint == Color::WHITE { c } else { c.mul(s.tint) };
            fb.blend(x, y, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{FRect, IRect, Vec2};

    #[test]
    fn sprite_scales_flips_and_sorts() {
        let mut store = TextureStore::new();
        let mut img = Image::new(2, 1);
        img.set(0, 0, Color::RED);
        img.set(1, 0, Color::GREEN);
        let t = store.insert("t", img);
        let mut list = DrawList::new(8, 4);
        list.clear(Color::BLACK);
        list.draw(t, IRect::new(0, 0, 2, 1), Vec2::new(0.0, 0.0), 2.0, 0.5);
        let mut s = Sprite::new(t, IRect::new(0, 0, 2, 1), FRect::new(4.0, 0.0, 4.0, 2.0), 0.5);
        s.flip_x = true;
        list.sprite(s);
        // Lower depth submitted later must still end up underneath.
        list.fill(FRect::new(0.0, 0.0, 8.0, 4.0), Color::BLACK.with_alpha(1.0), 0.1);
        let fb = render(&list, &store);
        assert_eq!(fb.get(0, 0), Color::RED);
        assert_eq!(fb.get(1, 1), Color::RED);
        assert_eq!(fb.get(2, 0), Color::GREEN);
        assert_eq!(fb.get(4, 0), Color::GREEN);
        assert_eq!(fb.get(7, 1), Color::RED);
        assert_eq!(fb.get(0, 3), Color::BLACK);
    }

    #[test]
    fn png_roundtrip() {
        let mut img = Image::new(3, 2);
        img.set(2, 1, Color::rgba(1, 2, 3, 4));
        let back = Image::decode_png(&img.encode_png()).unwrap();
        assert_eq!(back, img);
    }
}
