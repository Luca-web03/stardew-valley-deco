//! Composing a farmer sheet from configured layers (see
//! [`crate::layouts::FarmerLayer`]). Works on any sheets; nothing is
//! assumed about the original game's layout beyond what the user configures.

use engine::{Color, Image};

use crate::layouts::FarmerLayer;

/// Output frame size and grid.
pub const FRAME_W: i32 = 16;
pub const FRAME_H: i32 = 32;

/// Appearance colours used for tinting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tints {
    pub skin: [u8; 3],
    pub hair: [u8; 3],
    pub shirt: [u8; 3],
    pub pants: [u8; 3],
}

fn tint_for(t: &Tints, name: Option<&str>) -> Option<[u8; 3]> {
    match name? {
        "skin" => Some(t.skin),
        "hair" => Some(t.hair),
        "shirt" => Some(t.shirt),
        "pants" => Some(t.pants),
        _ => None,
    }
}

/// Alpha-blend `src` over `dst`.
fn blend(dst: Color, src: Color) -> Color {
    if src.a == 255 || dst.a == 0 {
        return src;
    }
    if src.a == 0 {
        return dst;
    }
    let a = src.a as u32;
    let mix = |d: u8, s: u8| ((s as u32 * a + d as u32 * (255 - a)) / 255) as u8;
    Color::rgba(mix(dst.r, src.r), mix(dst.g, src.g), mix(dst.b, src.b), dst.a.max(src.a))
}

/// Build the standard 4x4 farmer sheet from `layers` (bottom first).
pub fn compose(layers: &[(&Image, &FarmerLayer)], tints: &Tints) -> Image {
    let mut out = Image::new((FRAME_W * 4) as u32, (FRAME_H * 4) as u32);
    for (img, layer) in layers {
        let tint = tint_for(tints, layer.tint.as_deref());
        for facing in 0..4 {
            for walk in 0..4 {
                let sx0 = layer.origin[0] + layer.cols[walk] * layer.frame_w;
                let sy0 = layer.origin[1] + layer.rows[facing] * layer.frame_h;
                let flip = layer.flip_left && facing == 3;
                for y in 0..layer.frame_h {
                    for x in 0..layer.frame_w {
                        let sx = if flip { sx0 + layer.frame_w - 1 - x } else { sx0 + x };
                        let sy = sy0 + y;
                        if sx < 0 || sy < 0 || sx >= img.width as i32 || sy >= img.height as i32 {
                            continue;
                        }
                        let mut c = img.get(sx as u32, sy as u32);
                        if c.a == 0 {
                            continue;
                        }
                        if let Some(t) = tint {
                            c = Color::rgba((c.r as u32 * t[0] as u32 / 255) as u8, (c.g as u32 * t[1] as u32 / 255) as u8, (c.b as u32 * t[2] as u32 / 255) as u8, c.a);
                        }
                        let ox = layer.offset[0] + x;
                        let oy = layer.offset[1] + y;
                        if ox < 0 || oy < 0 || ox >= FRAME_W || oy >= FRAME_H {
                            continue;
                        }
                        let (dx, dy) = ((walk as i32 * FRAME_W + ox) as u32, (facing as i32 * FRAME_H + oy) as u32);
                        let d = out.get(dx, dy);
                        out.set(dx as i32, dy as i32, blend(d, c));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_stack_tint_and_place() {
        // A 2-cell body sheet (white) and a 4x4 "shirt" patch (white).
        let mut body = Image::new(32, 128);
        for y in 0..128 {
            for x in 0..32 {
                body.set(x, y, Color::WHITE);
            }
        }
        let mut patch = Image::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                patch.set(x, y, Color::WHITE);
            }
        }
        let base = FarmerLayer { texture: "body".into(), cols: [0, 1, 0, 1], ..Default::default() };
        let shirt = FarmerLayer { texture: "shirt".into(), frame_w: 4, frame_h: 4, rows: [0; 4], cols: [0; 4], offset: [6, 16], tint: Some("shirt".into()), ..Default::default() };
        let tints = Tints { skin: [200, 150, 120], hair: [0, 0, 0], shirt: [10, 200, 30], pants: [0, 0, 0] };
        let out = compose(&[(&body, &base), (&patch, &shirt)], &tints);
        assert_eq!((out.width, out.height), (64, 128));
        // Body pixel untinted, shirt patch tinted and placed at (6,16) in each frame.
        assert_eq!(out.get(0, 0), Color::WHITE);
        assert_eq!(out.get(6, 16), Color::rgba(10, 200, 30, 255));
        assert_eq!(out.get(16 * 3 + 9, 32 * 2 + 19), Color::rgba(10, 200, 30, 255));
        assert_eq!(out.get(5, 16), Color::WHITE);
    }
}
