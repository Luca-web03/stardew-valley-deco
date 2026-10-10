//! Reader for the xTile binary map format ("tBIN10"), as documented by the
//! xTile/tIDE map editor ecosystem.

use crate::map::{AnimatedTile, Layer, Map, Tile, TileSheet};
use crate::props::{PropValue, Props};

struct R<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> R<'a> {
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.p + n > self.b.len() {
            return Err(format!("tbin: unexpected end at {}", self.p));
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }
    fn i32(&mut self) -> Result<i32, String> {
        let b = self.bytes(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn f32(&mut self) -> Result<f32, String> {
        let b = self.bytes(4)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn string(&mut self) -> Result<String, String> {
        let n = self.i32()?;
        if n < 0 {
            return Err("tbin: negative string length".into());
        }
        Ok(String::from_utf8_lossy(self.bytes(n as usize)?).into_owned())
    }
    fn size(&mut self) -> Result<(i32, i32), String> {
        Ok((self.i32()?, self.i32()?))
    }
    fn props(&mut self) -> Result<Props, String> {
        let n = self.i32()?;
        let mut p = Props::new();
        for _ in 0..n {
            let k = self.string()?;
            let v = match self.u8()? {
                0 => PropValue::Bool(self.u8()? != 0),
                1 => PropValue::Int(self.i32()?),
                2 => PropValue::Float(self.f32()?),
                3 => PropValue::Str(self.string()?),
                t => return Err(format!("tbin: bad property type {t}")),
            };
            p.insert(k, v);
        }
        Ok(p)
    }
}

pub fn parse(bytes: &[u8], id: &str) -> Result<Map, String> {
    let mut r = R { b: bytes, p: 0 };
    if r.bytes(6)? != b"tBIN10" {
        return Err("not a tBIN10 map".into());
    }
    let mut map = Map { id: id.to_string(), ..Default::default() };
    let _map_id = r.string()?;
    let _desc = r.string()?;
    map.props = r.props()?;

    let nsheets = r.i32()?;
    for _ in 0..nsheets {
        let sid = r.string()?;
        let _d = r.string()?;
        let image = r.string()?;
        let (sw, sh) = r.size()?;
        let (tw, th) = r.size()?;
        let margin = r.size()?;
        let spacing = r.size()?;
        let props = r.props()?;
        map.sheets.push(TileSheet {
            id: sid,
            image: crate::tmx::image_asset_name(&image),
            sheet_w: sw,
            sheet_h: sh,
            tile_w: tw,
            tile_h: th,
            margin,
            spacing,
            props,
        });
    }

    let nlayers = r.i32()?;
    for _ in 0..nlayers {
        let lid = r.string()?;
        let visible = r.u8()? != 0;
        let _d = r.string()?;
        let (lw, lh) = r.size()?;
        let (tw, th) = r.size()?;
        if map.tile_w == 0 {
            map.tile_w = tw;
            map.tile_h = th;
        }
        map.width = map.width.max(lw);
        map.height = map.height.max(lh);
        let mut layer = Layer::new(&lid, lw, lh);
        layer.visible = visible;
        layer.props = r.props()?;
        let mut cur_sheet: Option<u16> = None;
        for y in 0..lh {
            let mut x = 0;
            while x < lw {
                match r.u8()? {
                    b'T' => {
                        let s = r.string()?;
                        cur_sheet = map.sheet_index(&s);
                    }
                    b'N' => x += r.i32()?,
                    b'S' => {
                        let t = read_static(&mut r, cur_sheet)?;
                        layer.set(x, y, Some(t));
                        x += 1;
                    }
                    b'A' => {
                        let interval = r.i32()?;
                        let count = r.i32()?;
                        let mut frames = Vec::new();
                        let mut fs = cur_sheet;
                        let mut first: Option<Tile> = None;
                        while (frames.len() as i32) < count {
                            match r.u8()? {
                                b'T' => {
                                    let s = r.string()?;
                                    fs = map.sheet_index(&s);
                                }
                                b'S' => {
                                    let t = read_static(&mut r, fs)?;
                                    frames.push((t.sheet, t.index));
                                    first.get_or_insert(t);
                                }
                                c => return Err(format!("tbin: bad anim frame tag {c}")),
                            }
                        }
                        let anim_props = r.props()?;
                        if let Some(mut t) = first {
                            t.props.extend(anim_props);
                            t.anim = Some(AnimatedTile { frames, interval_ms: interval });
                            layer.set(x, y, Some(t));
                        }
                        x += 1;
                    }
                    c => return Err(format!("tbin: bad tile tag {c} at {}", r.p)),
                }
            }
        }
        map.layers.push(layer);
    }
    Ok(map)
}

fn read_static(r: &mut R, sheet: Option<u16>) -> Result<Tile, String> {
    let index = r.i32()?;
    let _blend = r.u8()?;
    let props = r.props()?;
    let mut t = Tile::new(sheet.unwrap_or(0), index);
    t.props = props;
    Ok(t)
}

/// Serialize to tBIN10 (used to test the reader).
pub fn write(map: &Map) -> Vec<u8> {
    fn s(o: &mut Vec<u8>, v: &str) {
        o.extend((v.len() as i32).to_le_bytes());
        o.extend(v.as_bytes());
    }
    fn i(o: &mut Vec<u8>, v: i32) {
        o.extend(v.to_le_bytes());
    }
    fn props(o: &mut Vec<u8>, p: &Props) {
        i(o, p.len() as i32);
        for (k, v) in p {
            s(o, k);
            match v {
                PropValue::Bool(b) => {
                    o.push(0);
                    o.push(*b as u8)
                }
                PropValue::Int(n) => {
                    o.push(1);
                    i(o, *n)
                }
                PropValue::Float(f) => {
                    o.push(2);
                    o.extend(f.to_le_bytes())
                }
                PropValue::Str(t) => {
                    o.push(3);
                    s(o, t)
                }
            }
        }
    }
    let mut o = b"tBIN10".to_vec();
    s(&mut o, &map.id);
    s(&mut o, "");
    props(&mut o, &map.props);
    i(&mut o, map.sheets.len() as i32);
    for sh in &map.sheets {
        s(&mut o, &sh.id);
        s(&mut o, "");
        s(&mut o, &sh.image);
        i(&mut o, sh.sheet_w);
        i(&mut o, sh.sheet_h);
        i(&mut o, sh.tile_w);
        i(&mut o, sh.tile_h);
        i(&mut o, sh.margin.0);
        i(&mut o, sh.margin.1);
        i(&mut o, sh.spacing.0);
        i(&mut o, sh.spacing.1);
        props(&mut o, &sh.props);
    }
    i(&mut o, map.layers.len() as i32);
    for l in &map.layers {
        s(&mut o, &l.id);
        o.push(l.visible as u8);
        s(&mut o, "");
        i(&mut o, l.width);
        i(&mut o, l.height);
        i(&mut o, map.tile_w);
        i(&mut o, map.tile_h);
        props(&mut o, &l.props);
        let mut cur: Option<u16> = None;
        for y in 0..l.height {
            let mut nulls = 0;
            for x in 0..l.width {
                match l.get(x, y) {
                    None => nulls += 1,
                    Some(t) => {
                        if nulls > 0 {
                            o.push(b'N');
                            i(&mut o, nulls);
                            nulls = 0;
                        }
                        let emit = |o: &mut Vec<u8>, sheet: u16, idx: i32, p: &Props, cur: &mut Option<u16>| {
                            if *cur != Some(sheet) {
                                o.push(b'T');
                                s(o, &map.sheets[sheet as usize].id);
                                *cur = Some(sheet);
                            }
                            o.push(b'S');
                            i(o, idx);
                            o.push(0);
                            props(o, p);
                        };
                        match &t.anim {
                            None => emit(&mut o, t.sheet, t.index, &t.props, &mut cur),
                            Some(a) => {
                                o.push(b'A');
                                i(&mut o, a.interval_ms);
                                i(&mut o, a.frames.len() as i32);
                                for (fs, fi) in &a.frames {
                                    emit(&mut o, *fs, *fi, &Props::new(), &mut cur);
                                }
                                props(&mut o, &t.props);
                            }
                        }
                    }
                }
            }
            if nulls > 0 {
                o.push(b'N');
                i(&mut o, nulls);
            }
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut map = Map { id: "M".into(), width: 4, height: 2, tile_w: 16, tile_h: 16, ..Default::default() };
        map.sheets.push(TileSheet { id: "a".into(), image: "x".into(), sheet_w: 4, sheet_h: 4, tile_w: 16, tile_h: 16, ..Default::default() });
        map.sheets.push(TileSheet { id: "b".into(), image: "y".into(), sheet_w: 4, sheet_h: 4, tile_w: 16, tile_h: 16, ..Default::default() });
        let mut l = Layer::new("Back", 4, 2);
        l.set(1, 0, Some(Tile::new(1, 3)));
        let mut t = Tile::new(0, 2);
        t.anim = Some(AnimatedTile { frames: vec![(0, 2), (0, 3)], interval_ms: 250 });
        t.props.insert("Water".into(), PropValue::Str("T".into()));
        l.set(3, 1, Some(t));
        map.layers.push(l);
        let back = parse(&write(&map), "M").unwrap();
        assert_eq!(back.tile("Back", 1, 0).unwrap().sheet, 1);
        let a = back.tile("Back", 3, 1).unwrap();
        assert_eq!(a.anim.as_ref().unwrap().frames.len(), 2);
        assert_eq!(a.frame_at(260), (0, 3));
        assert_eq!(back.tile_prop("Back", 3, 1, "Water").as_deref(), Some("T"));
        assert!(back.tile("Back", 0, 0).is_none());
    }
}
