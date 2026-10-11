//! Minimal XNB container reader.
//!
//! Supports uncompressed and LZ4-compressed XNB files holding a `Texture2D`
//! (SurfaceFormat Color, Bgr565, Bgra5551, Bgra4444, Dxt1, Dxt3, Dxt5), a
//! `SpriteFont` or an xTile map (raw tBIN payload). LZX-compressed files are
//! reported as unsupported; unpack those with StardewXNBHack.

pub enum XnbAsset {
    Texture(engine::Image),
    /// Raw tBIN bytes from an xTile map asset.
    Tbin(Vec<u8>),
    Font(engine::font::SpriteFontData),
}

fn read_7bit(b: &[u8], p: &mut usize) -> Result<u32, String> {
    let mut result = 0u32;
    let mut shift = 0;
    loop {
        let byte = *b.get(*p).ok_or("xnb: eof in 7-bit int")?;
        *p += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift > 28 {
            return Err("xnb: bad 7-bit int".into());
        }
    }
}

fn read_str(b: &[u8], p: &mut usize) -> Result<String, String> {
    let n = read_7bit(b, p)? as usize;
    let s = b.get(*p..*p + n).ok_or("xnb: eof in string")?;
    *p += n;
    Ok(String::from_utf8_lossy(s).into_owned())
}

fn u32le(b: &[u8], p: &mut usize) -> Result<u32, String> {
    let s = b.get(*p..*p + 4).ok_or("xnb: eof")?;
    *p += 4;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn read(bytes: &[u8]) -> Result<XnbAsset, String> {
    if bytes.len() < 10 || &bytes[0..3] != b"XNB" {
        return Err("not an XNB file".into());
    }
    let flags = bytes[5];
    let total = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
    let body: Vec<u8> = if flags & 0x40 != 0 {
        let dsize = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
        let end = total.min(bytes.len());
        lz4_flex::block::decompress(&bytes[14..end], dsize).map_err(|e| format!("xnb lz4: {e}"))?
    } else if flags & 0x80 != 0 {
        return Err("xnb: LZX compression not supported; unpack with StardewXNBHack".into());
    } else {
        bytes[10..total.min(bytes.len())].to_vec()
    };
    let b = &body[..];
    let mut p = 0;
    let nreaders = read_7bit(b, &mut p)?;
    let mut readers = Vec::new();
    for _ in 0..nreaders {
        readers.push(read_str(b, &mut p)?);
        p += 4; // reader version
    }
    let _shared = read_7bit(b, &mut p)?;
    let ti = read_7bit(b, &mut p)?;
    if ti == 0 {
        return Err("xnb: null root asset".into());
    }
    let reader = readers.get(ti as usize - 1).cloned().unwrap_or_default();
    if reader.contains("Texture2DReader") {
        return Ok(XnbAsset::Texture(read_texture(b, &mut p)?));
    }
    if reader.contains("SpriteFontReader") {
        return Ok(XnbAsset::Font(read_sprite_font(b, &mut p)?));
    }
    if reader.contains("TideReader") || reader.contains("xTile") {
        let n = u32le(b, &mut p)? as usize;
        let data = b.get(p..p + n).ok_or("xnb: eof in map")?;
        return Ok(XnbAsset::Tbin(data.to_vec()));
    }
    Err(format!("xnb: unsupported asset reader {reader}"))
}

fn i32le(b: &[u8], p: &mut usize) -> Result<i32, String> {
    Ok(u32le(b, p)? as i32)
}

fn f32le(b: &[u8], p: &mut usize) -> Result<f32, String> {
    Ok(f32::from_bits(u32le(b, p)?))
}

/// A .NET `BinaryReader.ReadChar` (UTF-8).
fn read_char(b: &[u8], p: &mut usize) -> Result<char, String> {
    let first = *b.get(*p).ok_or("xnb: eof in char")?;
    let n = match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    };
    let bytes = b.get(*p..*p + n).ok_or("xnb: eof in char")?;
    *p += n;
    std::str::from_utf8(bytes).ok().and_then(|s| s.chars().next()).ok_or_else(|| "xnb: bad char".into())
}

fn read_rect(b: &[u8], p: &mut usize) -> Result<engine::IRect, String> {
    Ok(engine::IRect::new(i32le(b, p)?, i32le(b, p)?, i32le(b, p)?, i32le(b, p)?))
}

/// `ReadObject<List<T>>` of a value type: type id, count, raw elements.
fn read_list<T>(b: &[u8], p: &mut usize, mut item: impl FnMut(&[u8], &mut usize) -> Result<T, String>) -> Result<Vec<T>, String> {
    if read_7bit(b, p)? == 0 {
        return Ok(Vec::new());
    }
    let n = u32le(b, p)? as usize;
    let mut v = Vec::with_capacity(n.min(1 << 16));
    for _ in 0..n {
        v.push(item(b, p)?);
    }
    Ok(v)
}

fn read_sprite_font(b: &[u8], p: &mut usize) -> Result<engine::font::SpriteFontData, String> {
    if read_7bit(b, p)? == 0 {
        return Err("xnb: font without a texture".into());
    }
    let image = read_texture(b, p)?;
    let glyphs = read_list(b, p, read_rect)?;
    let cropping = read_list(b, p, read_rect)?;
    let chars = read_list(b, p, read_char)?;
    let line_spacing = i32le(b, p)?;
    let spacing = f32le(b, p)?;
    let kerning = read_list(b, p, |b, p| Ok([f32le(b, p)?, f32le(b, p)?, f32le(b, p)?]))?;
    let has_default = *b.get(*p).ok_or("xnb: eof in font")? != 0;
    *p += 1;
    let default_char = if has_default { Some(read_char(b, p)?) } else { None };
    Ok(engine::font::SpriteFontData { image, glyphs, cropping, chars, line_spacing, spacing, kerning, default_char })
}

/// A `Texture2D` body: format, size, mip levels (only the first is kept).
fn read_texture(b: &[u8], p: &mut usize) -> Result<engine::Image, String> {
    let format = u32le(b, p)?;
    let w = u32le(b, p)?;
    let h = u32le(b, p)?;
    let mips = u32le(b, p)?;
    let size = u32le(b, p)? as usize;
    let data = b.get(*p..*p + size).ok_or("xnb: eof in texture")?;
    *p += size;
    // Skip the remaining mip levels.
    for _ in 1..mips {
        let n = u32le(b, p)? as usize;
        *p += n;
    }
    let mut rgba = decode_surface(format, w, h, data)?;
    // XNA stores premultiplied alpha; convert back to straight alpha.
    for px in rgba.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 && a < 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Ok(engine::Image { width: w, height: h, rgba })
}

fn expand(v: u32, bits: u32) -> u8 {
    ((v * 255 + ((1 << bits) - 1) / 2) / ((1 << bits) - 1)) as u8
}

fn rgb565(c: u16) -> [u8; 3] {
    [expand((c >> 11) as u32 & 31, 5), expand((c >> 5) as u32 & 63, 6), expand(c as u32 & 31, 5)]
}

/// Pixels of an XNA surface as RGBA8.
pub fn decode_surface(format: u32, w: u32, h: u32, data: &[u8]) -> Result<Vec<u8>, String> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; w * h * 4];
    match format {
        0 => {
            let n = w * h * 4;
            out.copy_from_slice(data.get(..n).ok_or("xnb: short texture")?);
        }
        1..=3 => {
            for i in 0..w * h {
                let c = u16::from_le_bytes([*data.get(i * 2).ok_or("xnb: short texture")?, data[i * 2 + 1]]);
                let px = match format {
                    1 => {
                        let [r, g, b] = rgb565(c);
                        [r, g, b, 255]
                    }
                    2 => [expand((c >> 10) as u32 & 31, 5), expand((c >> 5) as u32 & 31, 5), expand(c as u32 & 31, 5), if c & 0x8000 != 0 { 255 } else { 0 }],
                    _ => [expand((c >> 8) as u32 & 15, 4), expand((c >> 4) as u32 & 15, 4), expand(c as u32 & 15, 4), expand((c >> 12) as u32 & 15, 4)],
                };
                out[i * 4..i * 4 + 4].copy_from_slice(&px);
            }
        }
        4..=6 => {
            let block = if format == 4 { 8 } else { 16 };
            let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
            for by in 0..bh {
                for bx in 0..bw {
                    let off = (by * bw + bx) * block;
                    let blk = data.get(off..off + block).ok_or("xnb: short DXT texture")?;
                    let (alpha, color) = if format == 4 { (None, blk) } else { (Some(&blk[..8]), &blk[8..]) };
                    let c0 = u16::from_le_bytes([color[0], color[1]]);
                    let c1 = u16::from_le_bytes([color[2], color[3]]);
                    let (a, b) = (rgb565(c0), rgb565(c1));
                    let mix = |x: u8, y: u8, wx: u32, wy: u32| ((x as u32 * wx + y as u32 * wy) / (wx + wy)) as u8;
                    let four = format != 4 || c0 > c1;
                    let palette: [[u8; 4]; 4] = if four {
                        [
                            [a[0], a[1], a[2], 255],
                            [b[0], b[1], b[2], 255],
                            [mix(a[0], b[0], 2, 1), mix(a[1], b[1], 2, 1), mix(a[2], b[2], 2, 1), 255],
                            [mix(a[0], b[0], 1, 2), mix(a[1], b[1], 1, 2), mix(a[2], b[2], 1, 2), 255],
                        ]
                    } else {
                        [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], [mix(a[0], b[0], 1, 1), mix(a[1], b[1], 1, 1), mix(a[2], b[2], 1, 1), 255], [0, 0, 0, 0]]
                    };
                    let bits = u32::from_le_bytes([color[4], color[5], color[6], color[7]]);
                    // DXT5 alpha palette.
                    let alpha5: Option<([u8; 8], u64)> = (format == 6).then(|| {
                        let al = alpha.unwrap();
                        let (a0, a1) = (al[0] as u32, al[1] as u32);
                        let mut pal = [0u8; 8];
                        pal[0] = a0 as u8;
                        pal[1] = a1 as u8;
                        if a0 > a1 {
                            for k in 1..7u32 {
                                pal[k as usize + 1] = (((7 - k) * a0 + k * a1) / 7) as u8;
                            }
                        } else {
                            for k in 1..5u32 {
                                pal[k as usize + 1] = (((5 - k) * a0 + k * a1) / 5) as u8;
                            }
                            pal[6] = 0;
                            pal[7] = 255;
                        }
                        let mut idx = 0u64;
                        for (i, v) in al[2..8].iter().enumerate() {
                            idx |= (*v as u64) << (8 * i);
                        }
                        (pal, idx)
                    });
                    for py in 0..4 {
                        for px in 0..4 {
                            let (x, y) = (bx * 4 + px, by * 4 + py);
                            if x >= w || y >= h {
                                continue;
                            }
                            let i = py * 4 + px;
                            let mut c = palette[((bits >> (2 * i)) & 3) as usize];
                            if format == 5 {
                                let al = alpha.unwrap();
                                let nib = (u64::from_le_bytes(al.try_into().unwrap()) >> (4 * i)) & 15;
                                c[3] = expand(nib as u32, 4);
                            } else if let Some((pal, idx)) = alpha5 {
                                c[3] = pal[((idx >> (3 * i)) & 7) as usize];
                            }
                            let o = (y * w + x) * 4;
                            out[o..o + 4].copy_from_slice(&c);
                        }
                    }
                }
            }
        }
        other => return Err(format!("xnb: unsupported surface format {other}")),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w7(v: &mut Vec<u8>, mut n: u32) {
        loop {
            let b = (n & 0x7F) as u8;
            n >>= 7;
            if n == 0 {
                v.push(b);
                break;
            }
            v.push(b | 0x80);
        }
    }

    fn wstr(v: &mut Vec<u8>, s: &str) {
        w7(v, s.len() as u32);
        v.extend_from_slice(s.as_bytes());
    }

    fn container(readers: &[&str], body: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        w7(&mut b, readers.len() as u32);
        for r in readers {
            wstr(&mut b, r);
            b.extend_from_slice(&0u32.to_le_bytes());
        }
        w7(&mut b, 0);
        b.extend_from_slice(body);
        let mut f = b"XNBw\x05\x00".to_vec();
        f.extend_from_slice(&((10 + b.len()) as u32).to_le_bytes());
        f.extend_from_slice(&b);
        f
    }

    #[test]
    fn reads_a_sprite_font() {
        let readers = [
            "Microsoft.Xna.Framework.Content.SpriteFontReader",
            "Microsoft.Xna.Framework.Content.Texture2DReader",
            "Microsoft.Xna.Framework.Content.ListReader`1[[Microsoft.Xna.Framework.Rectangle]]",
            "Microsoft.Xna.Framework.Content.ListReader`1[[System.Char]]",
            "Microsoft.Xna.Framework.Content.ListReader`1[[Microsoft.Xna.Framework.Vector3]]",
        ];
        let mut body = Vec::new();
        w7(&mut body, 1); // SpriteFont
        w7(&mut body, 2); // Texture2D: Color 2x1, one mip
        for v in [0u32, 2, 1, 1, 8] {
            body.extend_from_slice(&v.to_le_bytes());
        }
        body.extend_from_slice(&[255, 0, 0, 255, 0, 0, 0, 0]);
        for _ in 0..2 {
            w7(&mut body, 3); // List<Rectangle>
            body.extend_from_slice(&2u32.to_le_bytes());
            for r in [[0i32, 0, 1, 1], [1, 0, 1, 1]] {
                for v in r {
                    body.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        w7(&mut body, 4); // List<char>
        body.extend_from_slice(&2u32.to_le_bytes());
        body.extend_from_slice("aé".as_bytes());
        body.extend_from_slice(&12i32.to_le_bytes());
        body.extend_from_slice(&1.5f32.to_le_bytes());
        w7(&mut body, 5); // List<Vector3>
        body.extend_from_slice(&2u32.to_le_bytes());
        for v in [0.0f32, 1.0, 1.0, 0.5, 1.0, 0.0] {
            body.extend_from_slice(&v.to_le_bytes());
        }
        body.push(1);
        body.push(b'a');
        let Ok(XnbAsset::Font(f)) = read(&container(&readers, &body)) else { panic!("not a font") };
        assert_eq!(f.chars, vec!['a', 'é']);
        assert_eq!(f.glyphs[1], engine::IRect::new(1, 0, 1, 1));
        assert_eq!((f.line_spacing, f.spacing), (12, 1.5));
        assert_eq!(f.kerning[1], [0.5, 1.0, 0.0]);
        assert_eq!(f.default_char, Some('a'));
        assert_eq!(&f.image.rgba[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn decodes_dxt1_blocks() {
        // c0 = pure red, c1 = pure blue, all pixels index 0 except the last (1).
        let red: u16 = 31 << 11;
        let blue: u16 = 31;
        let mut blk = Vec::new();
        blk.extend_from_slice(&red.to_le_bytes());
        blk.extend_from_slice(&blue.to_le_bytes());
        blk.extend_from_slice(&(1u32 << 30).to_le_bytes());
        let px = decode_surface(4, 4, 4, &blk).unwrap();
        assert_eq!(&px[..4], &[255, 0, 0, 255]);
        assert_eq!(&px[60..64], &[0, 0, 255, 255]);
    }
}
