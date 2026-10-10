//! TMX (Tiled XML) reader and writer, matching the layout produced by
//! StardewXNBHack / TMXTile when unpacking xTile maps.

use std::collections::HashMap;
use std::io::Read;

use crate::map::{AnimatedTile, Layer, Map, Tile, TileSheet};
use crate::props::{PropValue, Props};

const FLIP_MASK: u32 = 0x1FFF_FFFF;

fn parse_props(node: roxmltree::Node) -> Props {
    let mut p = Props::new();
    for props in node.children().filter(|c| c.has_tag_name("properties")) {
        for prop in props.children().filter(|c| c.has_tag_name("property")) {
            let Some(name) = prop.attribute("name") else { continue };
            let raw = prop.attribute("value").map(|s| s.to_string()).unwrap_or_else(|| prop.text().unwrap_or("").to_string());
            let v = match prop.attribute("type") {
                Some("bool") => PropValue::Bool(raw == "true" || raw == "T"),
                Some("int") => raw.parse().map(PropValue::Int).unwrap_or(PropValue::Str(raw)),
                Some("float") => raw.parse().map(PropValue::Float).unwrap_or(PropValue::Str(raw)),
                _ => PropValue::Str(raw),
            };
            p.insert(name.to_string(), v);
        }
    }
    p
}

fn attr_i32(n: roxmltree::Node, a: &str, d: i32) -> i32 {
    n.attribute(a).and_then(|v| v.parse().ok()).unwrap_or(d)
}

/// Strip directory and extension from an image path: "../Maps/foo.png" -> "foo".
pub fn image_asset_name(src: &str) -> String {
    let s = src.replace('\\', "/");
    let file = s.rsplit('/').next().unwrap_or(&s);
    match file.rfind('.') {
        Some(i) => file[..i].to_string(),
        None => file.to_string(),
    }
}

fn decode_data(data: roxmltree::Node, count: usize) -> Result<Vec<u32>, String> {
    let enc = data.attribute("encoding");
    let comp = data.attribute("compression");
    let text = data.text().unwrap_or("").trim();
    match enc {
        Some("csv") => text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u32>().map_err(|e| e.to_string()))
            .collect(),
        Some("base64") => {
            let raw = base64_decode(text)?;
            let bytes = match comp {
                Some("zlib") => {
                    let mut out = Vec::new();
                    flate2::read::ZlibDecoder::new(&raw[..]).read_to_end(&mut out).map_err(|e| e.to_string())?;
                    out
                }
                Some("gzip") => {
                    let mut out = Vec::new();
                    flate2::read::GzDecoder::new(&raw[..]).read_to_end(&mut out).map_err(|e| e.to_string())?;
                    out
                }
                None => raw,
                Some(c) => return Err(format!("unsupported TMX compression {c}")),
            };
            Ok(bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
        }
        None => {
            // Plain XML <tile gid=".."/> children.
            let v: Vec<u32> = data
                .children()
                .filter(|c| c.has_tag_name("tile"))
                .map(|t| t.attribute("gid").and_then(|g| g.parse().ok()).unwrap_or(0))
                .collect();
            if v.len() < count {
                let mut v = v;
                v.resize(count, 0);
                Ok(v)
            } else {
                Ok(v)
            }
        }
        Some(e) => Err(format!("unsupported TMX encoding {e}")),
    }
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    }
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        let Some(v) = val(c) else { continue };
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

pub fn parse(xml: &str, id: &str) -> Result<Map, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    if !root.has_tag_name("map") {
        return Err("not a TMX map".into());
    }
    let mut map = Map {
        id: id.to_string(),
        width: attr_i32(root, "width", 0),
        height: attr_i32(root, "height", 0),
        tile_w: attr_i32(root, "tilewidth", 16),
        tile_h: attr_i32(root, "tileheight", 16),
        props: parse_props(root),
        ..Default::default()
    };

    // gid -> (sheet index, local index); keep firstgids sorted.
    let mut firstgids: Vec<(u32, u16)> = Vec::new();
    // Per (sheet, local id) animation frames in local ids + duration.
    let mut anims: HashMap<(u16, i32), (Vec<i32>, i32)> = HashMap::new();

    for ts in root.children().filter(|c| c.has_tag_name("tileset")) {
        let sheet_idx = map.sheets.len() as u16;
        let firstgid = attr_i32(ts, "firstgid", 1) as u32;
        let tw = attr_i32(ts, "tilewidth", map.tile_w);
        let th = attr_i32(ts, "tileheight", map.tile_h);
        let margin = attr_i32(ts, "margin", 0);
        let spacing = attr_i32(ts, "spacing", 0);
        let mut sheet = TileSheet {
            id: ts.attribute("name").unwrap_or("").to_string(),
            tile_w: tw,
            tile_h: th,
            margin: (margin, margin),
            spacing: (spacing, spacing),
            props: parse_props(ts),
            ..Default::default()
        };
        let columns = attr_i32(ts, "columns", 0);
        if let Some(img) = ts.children().find(|c| c.has_tag_name("image")) {
            sheet.image = image_asset_name(img.attribute("source").unwrap_or(""));
            let iw = attr_i32(img, "width", 0);
            let ih = attr_i32(img, "height", 0);
            sheet.sheet_w = if columns > 0 { columns } else { (iw - margin * 2 + spacing) / (tw + spacing).max(1) };
            sheet.sheet_h = (ih - margin * 2 + spacing) / (th + spacing).max(1);
        } else {
            sheet.sheet_w = columns.max(1);
            let count = attr_i32(ts, "tilecount", columns);
            sheet.sheet_h = (count + sheet.sheet_w - 1) / sheet.sheet_w;
        }
        for tile in ts.children().filter(|c| c.has_tag_name("tile")) {
            let tid = attr_i32(tile, "id", 0);
            for (k, v) in parse_props(tile) {
                sheet.set_tile_prop(tid, &k, v);
            }
            if let Some(anim) = tile.children().find(|c| c.has_tag_name("animation")) {
                let mut frames = Vec::new();
                let mut dur = 0;
                for f in anim.children().filter(|c| c.has_tag_name("frame")) {
                    frames.push(attr_i32(f, "tileid", 0));
                    dur = attr_i32(f, "duration", dur);
                }
                if !frames.is_empty() {
                    anims.insert((sheet_idx, tid), (frames, dur.max(1)));
                }
            }
        }
        firstgids.push((firstgid, sheet_idx));
        map.sheets.push(sheet);
    }
    firstgids.sort();

    let resolve = |gid: u32| -> Option<(u16, i32)> {
        let gid = gid & FLIP_MASK;
        if gid == 0 {
            return None;
        }
        let (fg, s) = firstgids.iter().rev().find(|(fg, _)| *fg <= gid)?;
        Some((*s, (gid - fg) as i32))
    };

    for node in root.children() {
        if node.has_tag_name("layer") {
            let w = attr_i32(node, "width", map.width);
            let h = attr_i32(node, "height", map.height);
            let mut layer = Layer::new(node.attribute("name").unwrap_or(""), w, h);
            layer.visible = node.attribute("visible") != Some("0");
            layer.props = parse_props(node);
            if let Some(data) = node.children().find(|c| c.has_tag_name("data")) {
                let gids = decode_data(data, (w * h) as usize)?;
                for (i, gid) in gids.into_iter().enumerate().take((w * h) as usize) {
                    if let Some((s, idx)) = resolve(gid) {
                        let mut t = Tile::new(s, idx);
                        if let Some((frames, dur)) = anims.get(&(s, idx)) {
                            t.anim = Some(AnimatedTile { frames: frames.iter().map(|f| (s, *f)).collect(), interval_ms: *dur });
                        }
                        layer.tiles[i] = Some(t);
                    }
                }
            }
            map.layers.push(layer);
        }
    }

    // Tile-instance properties live in object groups named after the layer.
    for og in root.children().filter(|c| c.has_tag_name("objectgroup")) {
        let lname = og.attribute("name").unwrap_or("").to_string();
        let (tw, th) = (map.tile_w.max(1), map.tile_h.max(1));
        let Some(layer) = map.layer_mut(&lname) else { continue };
        for obj in og.children().filter(|c| c.has_tag_name("object")) {
            let x = obj.attribute("x").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0) as i32 / tw;
            let y = obj.attribute("y").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0) as i32 / th;
            let props = parse_props(obj);
            if let Some(t) = layer.get_mut(x, y) {
                t.props.extend(props);
            }
        }
    }
    Ok(map)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

fn write_props(out: &mut String, props: &Props, indent: &str) {
    if props.is_empty() {
        return;
    }
    out.push_str(&format!("{indent}<properties>\n"));
    for (k, v) in props {
        let (ty, val) = match v {
            PropValue::Bool(b) => (" type=\"bool\"", b.to_string()),
            PropValue::Int(i) => (" type=\"int\"", i.to_string()),
            PropValue::Float(f) => (" type=\"float\"", f.to_string()),
            PropValue::Str(s) => ("", s.clone()),
        };
        out.push_str(&format!("{indent} <property name=\"{}\"{} value=\"{}\"/>\n", esc(k), ty, esc(&val)));
    }
    out.push_str(&format!("{indent}</properties>\n"));
}

/// Serialize a map as TMX (CSV encoding). Inverse of [`parse`].
pub fn write(map: &Map) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<map version=\"1.0\" orientation=\"orthogonal\" renderorder=\"right-down\" width=\"{}\" height=\"{}\" tilewidth=\"{}\" tileheight=\"{}\">\n",
        map.width, map.height, map.tile_w, map.tile_h
    ));
    write_props(&mut out, &map.props, " ");
    let mut firstgid = 1;
    let mut firsts = Vec::new();
    for s in &map.sheets {
        firsts.push(firstgid);
        let count = s.sheet_w * s.sheet_h;
        out.push_str(&format!(
            " <tileset firstgid=\"{}\" name=\"{}\" tilewidth=\"{}\" tileheight=\"{}\" tilecount=\"{}\" columns=\"{}\">\n",
            firstgid, esc(&s.id), s.tile_w, s.tile_h, count, s.sheet_w
        ));
        let sheet_props: Props = s.props.iter().filter(|(k, _)| !k.starts_with("@TileIndex@")).map(|(k, v)| (k.clone(), v.clone())).collect();
        write_props(&mut out, &sheet_props, "  ");
        out.push_str(&format!(
            "  <image source=\"{}.png\" width=\"{}\" height=\"{}\"/>\n",
            esc(&s.image), s.sheet_w * s.tile_w, s.sheet_h * s.tile_h
        ));
        let mut per_tile: std::collections::BTreeMap<i32, Props> = Default::default();
        for (k, v) in &s.props {
            if let Some(rest) = k.strip_prefix("@TileIndex@") {
                let mut it = rest.splitn(2, '@');
                if let (Some(i), Some(n)) = (it.next().and_then(|i| i.parse().ok()), it.next()) {
                    per_tile.entry(i).or_default().insert(n.to_string(), v.clone());
                }
            }
        }
        // Animations are stored per tileset tile in TMX.
        let sheet_idx = firsts.len() as u16 - 1;
        let mut anims: std::collections::BTreeMap<i32, &crate::map::AnimatedTile> = Default::default();
        for l in &map.layers {
            for t in l.tiles.iter().flatten() {
                if let Some(a) = &t.anim {
                    if t.sheet == sheet_idx {
                        anims.insert(t.index, a);
                    }
                }
            }
        }
        let ids: std::collections::BTreeSet<i32> = per_tile.keys().chain(anims.keys()).copied().collect();
        for i in ids {
            out.push_str(&format!("  <tile id=\"{i}\">\n"));
            if let Some(p) = per_tile.get(&i) {
                write_props(&mut out, p, "   ");
            }
            if let Some(a) = anims.get(&i) {
                out.push_str("   <animation>\n");
                for (_, f) in &a.frames {
                    out.push_str(&format!("    <frame tileid=\"{f}\" duration=\"{}\"/>\n", a.interval_ms));
                }
                out.push_str("   </animation>\n");
            }
            out.push_str("  </tile>\n");
        }
        out.push_str(" </tileset>\n");
        firstgid += count.max(1) as u32;
    }
    let mut objects = String::new();
    for l in &map.layers {
        out.push_str(&format!(" <layer name=\"{}\" width=\"{}\" height=\"{}\"{}>\n", esc(&l.id), l.width, l.height, if l.visible { "" } else { " visible=\"0\"" }));
        write_props(&mut out, &l.props, "  ");
        out.push_str("  <data encoding=\"csv\">\n");
        let mut tile_objs = String::new();
        for y in 0..l.height {
            let row: Vec<String> = (0..l.width)
                .map(|x| match l.get(x, y) {
                    Some(t) => {
                        if !t.props.is_empty() {
                            tile_objs.push_str(&format!(
                                "  <object name=\"TileData\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\">\n",
                                x * map.tile_w, y * map.tile_h, map.tile_w, map.tile_h
                            ));
                            write_props(&mut tile_objs, &t.props, "   ");
                            tile_objs.push_str("  </object>\n");
                        }
                        (firsts[t.sheet as usize] + t.index as u32).to_string()
                    }
                    None => "0".into(),
                })
                .collect();
            out.push_str(&row.join(","));
            if y + 1 < l.height {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  </data>\n </layer>\n");
        if !tile_objs.is_empty() {
            objects.push_str(&format!(" <objectgroup name=\"{}\">\n{} </objectgroup>\n", esc(&l.id), tile_objs));
        }
    }
    out.push_str(&objects);
    out.push_str("</map>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut map = Map { id: "T".into(), width: 3, height: 2, tile_w: 16, tile_h: 16, ..Default::default() };
        map.props.insert("Warp".into(), PropValue::Str("0 0 Town 1 1".into()));
        let mut sheet = TileSheet { id: "a".into(), image: "sheet".into(), sheet_w: 4, sheet_h: 2, tile_w: 16, tile_h: 16, ..Default::default() };
        sheet.set_tile_prop(5, "Diggable", PropValue::Str("T".into()));
        map.sheets.push(sheet);
        let mut l = Layer::new("Back", 3, 2);
        l.set(0, 0, Some(Tile::new(0, 5)));
        let mut t = Tile::new(0, 7);
        t.props.insert("Action".into(), PropValue::Str("Mailbox".into()));
        l.set(2, 1, Some(t));
        map.layers.push(l);
        let xml = write(&map);
        let back = parse(&xml, "T").unwrap();
        assert_eq!(back.width, 3);
        assert_eq!(back.prop("Warp").as_deref(), Some("0 0 Town 1 1"));
        assert_eq!(back.tile("Back", 0, 0).unwrap().index, 5);
        assert_eq!(back.tile_prop("Back", 0, 0, "Diggable").as_deref(), Some("T"));
        assert_eq!(back.tile_prop("Back", 2, 1, "Action").as_deref(), Some("Mailbox"));
        assert!(back.tile("Back", 1, 0).is_none());
        assert_eq!(back.sheets[0].image, "sheet");
    }
}
