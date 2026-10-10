//! Minimal XNB container reader.
//!
//! Supports uncompressed and LZ4-compressed XNB files holding a `Texture2D`
//! (SurfaceFormat.Color) or an xTile map (raw tBIN payload). LZX-compressed
//! files are reported as unsupported; unpack those with StardewXNBHack.

pub enum XnbAsset {
    Texture(engine::Image),
    /// Raw tBIN bytes from an xTile map asset.
    Tbin(Vec<u8>),
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
        let format = u32le(b, &mut p)?;
        let w = u32le(b, &mut p)?;
        let h = u32le(b, &mut p)?;
        let _mips = u32le(b, &mut p)?;
        let size = u32le(b, &mut p)? as usize;
        if format != 0 {
            return Err(format!("xnb: unsupported surface format {format}"));
        }
        let data = b.get(p..p + size).ok_or("xnb: eof in texture")?;
        // XNA stores premultiplied alpha; convert back to straight alpha.
        let mut rgba = data.to_vec();
        for px in rgba.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a > 0 && a < 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        return Ok(XnbAsset::Texture(engine::Image { width: w, height: h, rgba }));
    }
    if reader.contains("TideReader") || reader.contains("xTile") {
        let n = u32le(b, &mut p)? as usize;
        let data = b.get(p..p + n).ok_or("xnb: eof in map")?;
        return Ok(XnbAsset::Tbin(data.to_vec()));
    }
    Err(format!("xnb: unsupported asset reader {reader}"))
}
