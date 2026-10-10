//! Placeholder maps, built in code and serialized to TMX.

use super::art::*;
use crate::map::{AnimatedTile, Layer, Map, Tile, TileSheet};
use crate::props::PropValue;

const OUT: u16 = 0;

fn s(v: &str) -> PropValue {
    PropValue::Str(v.to_string())
}

fn outdoor_sheet() -> TileSheet {
    let mut sh = TileSheet {
        id: "outdoors".into(),
        image: "spring_dev_outdoors".into(),
        sheet_w: OUT_COLS,
        sheet_h: 5,
        tile_w: 16,
        tile_h: 16,
        ..Default::default()
    };
    sh.set_tile_prop(DIRT, "Diggable", s("T"));
    sh.set_tile_prop(DIRT, "Type", s("Dirt"));
    sh.set_tile_prop(GRASS, "Diggable", s("T"));
    sh.set_tile_prop(GRASS, "Type", s("Grass"));
    sh.set_tile_prop(GRASS2, "Diggable", s("T"));
    sh.set_tile_prop(GRASS2, "Type", s("Grass"));
    sh.set_tile_prop(PATH, "Type", s("Stone"));
    sh.set_tile_prop(BRIDGE, "Type", s("Wood"));
    sh.set_tile_prop(SAND, "Diggable", s("T"));
    sh.set_tile_prop(SAND, "Type", s("Sand"));
    for f in 0..3 {
        sh.set_tile_prop(WATER + f, "Water", s("T"));
    }
    sh
}

fn indoor_sheet() -> TileSheet {
    let mut sh = TileSheet {
        id: "indoors".into(),
        image: "dev_indoors".into(),
        sheet_w: IN_COLS,
        sheet_h: 2,
        tile_w: 16,
        tile_h: 16,
        ..Default::default()
    };
    sh.set_tile_prop(FLOOR, "Type", s("Wood"));
    sh
}

struct B {
    map: Map,
}

impl B {
    fn new(id: &str, w: i32, h: i32, sheet: TileSheet) -> Self {
        let mut map = Map { id: id.into(), width: w, height: h, tile_w: 16, tile_h: 16, ..Default::default() };
        map.sheets.push(sheet);
        for l in ["Back", "Buildings", "Paths", "Front", "AlwaysFront"] {
            map.layers.push(Layer::new(l, w, h));
        }
        Self { map }
    }
    fn set(&mut self, layer: &str, x: i32, y: i32, idx: i32) {
        self.map.layer_mut(layer).unwrap().set(x, y, Some(Tile::new(0, idx)));
    }
    fn set_props(&mut self, layer: &str, x: i32, y: i32, idx: i32, props: &[(&str, &str)]) {
        let mut t = Tile::new(0, idx);
        for (k, v) in props {
            t.props.insert(k.to_string(), s(v));
        }
        self.map.layer_mut(layer).unwrap().set(x, y, Some(t));
    }
    fn fill(&mut self, layer: &str, x: i32, y: i32, w: i32, h: i32, idx: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.set(layer, xx, yy, idx);
            }
        }
    }
    fn water(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                let mut t = Tile::new(OUT, WATER);
                t.anim = Some(AnimatedTile { frames: vec![(OUT, WATER), (OUT, WATER + 1), (OUT, WATER + 2)], interval_ms: 300 });
                self.map.layer_mut("Back").unwrap().set(xx, yy, Some(t));
            }
        }
    }
    fn prop(&mut self, k: &str, v: &str) {
        self.map.props.insert(k.into(), s(v));
    }
    /// A tree from map tiles: trunk on Buildings, canopy on Front (drawn over the player).
    fn map_tree(&mut self, x: i32, y: i32) {
        self.set("Buildings", x, y, TRUNK);
        self.set("Front", x, y - 1, TREE_TOP);
        self.set("Front", x - 1, y - 1, TREE_TOP);
        self.set("Front", x + 1, y - 1, TREE_TOP);
        self.set("Front", x, y - 2, TREE_TOP);
    }
    /// A small house facade with a door tile carrying `action`.
    fn house(&mut self, x: i32, y: i32, w: i32, action: &str) {
        self.fill("Front", x, y, w, 2, ROOF);
        self.fill("Buildings", x, y + 2, w, 1, WALL_TOP);
        self.fill("Buildings", x, y + 3, w, 2, WALL);
        self.set("Buildings", x + 1, y + 3, WINDOW);
        self.set("Buildings", x + w - 2, y + 3, WINDOW);
        let dx = x + w / 2;
        self.set_props("Buildings", dx, y + 4, DOOR, &[("Action", action)]);
    }
}

pub fn farm() -> Map {
    let (w, h) = (48, 36);
    let mut b = B::new("Farm", w, h, outdoor_sheet());
    b.fill("Back", 0, 0, w, h, GRASS);
    for i in 0..60 {
        let x = (super::paint::hash(i, 0, 3) % w as u32) as i32;
        let y = (super::paint::hash(i, 1, 3) % h as u32) as i32;
        b.set("Back", x, y, GRASS2);
    }
    // Big tillable field.
    b.fill("Back", 8, 16, 26, 14, DIRT);
    // Path from the house to the east exit.
    b.fill("Back", 20, 10, 2, 6, PATH);
    b.fill("Back", 20, 13, w - 20, 2, PATH);
    // Pond.
    b.water(38, 22, 7, 6);
    b.fill("Back", 37, 21, 9, 1, SAND);
    // Border: cliffs/fence with the exit gap on the east edge at y 13..15.
    for x in 0..w {
        b.set("Buildings", x, 0, CLIFF);
        b.set("Buildings", x, h - 1, FENCE);
    }
    for y in 0..h {
        b.set("Buildings", 0, y, CLIFF);
        if !(13..15).contains(&y) {
            b.set("Buildings", w - 1, y, FENCE);
        }
    }
    for (x, y) in [(4, 6), (8, 4), (14, 3), (30, 4), (36, 6), (42, 3), (3, 20), (4, 30), (44, 32)] {
        b.map_tree(x, y);
    }
    // Farmhouse.
    b.house(18, 4, 6, "Warp 5 8 FarmHouse");
    b.set_props("Buildings", 25, 9, MAILBOX, &[("Action", "Mailbox")]);
    b.set("Buildings", 27, 9, BIN);
    for x in 30..34 {
        b.set("Back", x, 9, FLOWERS);
    }
    b.prop("Warp", &format!("{} 13 Town 1 15 {} 14 Town 1 16", w, w));
    b.prop("ShippingBinTile", "27 9");
    b.prop("FarmHouseEntry", "21 9");
    b.prop("SpawnDebris", "T");
    b.map
}

pub fn farmhouse() -> Map {
    let (w, h) = (12, 11);
    let mut b = B::new("FarmHouse", w, h, indoor_sheet());
    b.fill("Back", 0, 0, w, h, VOID);
    b.fill("Back", 1, 3, w - 2, h - 4, FLOOR);
    b.fill("Buildings", 1, 1, w - 2, 1, IWALL_TOP);
    b.fill("Buildings", 1, 2, w - 2, 1, IWALL);
    for y in 1..h - 1 {
        b.set("Buildings", 0, y, IWALL_TOP);
        b.set("Buildings", w - 1, y, IWALL_TOP);
    }
    for x in 0..w {
        b.set("Buildings", x, 0, IWALL_TOP);
        if x != 5 {
            b.set("Buildings", x, h - 1, IWALL_TOP);
        }
    }
    b.set("Back", 5, h - 2, MAT);
    b.set("Back", 5, h - 1, MAT);
    b.set_props("Back", 9, 4, BED_TOP, &[("Bed", "T")]);
    b.set_props("Back", 9, 5, BED_BOTTOM, &[("Bed", "T")]);
    b.set("Buildings", 2, 2, TV);
    b.set_props("Buildings", 3, 4, TABLE, &[]);
    b.fill("Back", 3, 6, 3, 2, RUG);
    b.prop("Warp", "5 10 Farm 21 10");
    b.map
}

pub fn town() -> Map {
    let (w, h) = (40, 30);
    let mut b = B::new("Town", w, h, outdoor_sheet());
    b.fill("Back", 0, 0, w, h, GRASS);
    b.fill("Back", 0, 15, w, 2, PATH);
    b.fill("Back", 24, 10, 2, 16, PATH);
    b.fill("Back", 8, 22, 30, 2, PATH);
    b.water(0, 26, w, 4);
    b.fill("Back", 14, 26, 3, 4, BRIDGE);
    for x in 0..w {
        b.set("Buildings", x, 0, CLIFF);
    }
    for y in 0..26 {
        if !(15..17).contains(&y) {
            b.set("Buildings", 0, y, FENCE);
        }
        b.set("Buildings", w - 1, y, FENCE);
    }
    // General store.
    b.house(21, 4, 8, "LockedDoorWarp 6 9 SeedShop 900 2100");
    // Mara's house.
    b.house(9, 2, 6, "Warp 0 0 Town");
    b.set_props("Buildings", 18, 11, SIGN, &[("Action", "Message \"Welcome to town!\"")]);
    for (x, y) in [(4, 5), (33, 12), (6, 20), (35, 20)] {
        b.map_tree(x, y);
    }
    // Cave mouth to the mines in the north-west corner.
    b.fill("Back", 2, 1, 3, 3, PATH);
    b.set("Buildings", 1, 1, CLIFF);
    b.set("Buildings", 5, 1, CLIFF);
    b.set("Buildings", 20, 14, LAMP);
    b.set("Buildings", 28, 14, LAMP);
    b.prop("Warp", "-1 15 Farm 46 13 -1 16 Farm 46 14 3 0 Mine 5 8");
    b.map
}

pub fn mine_entrance() -> Map {
    let (w, h) = (11, 10);
    let mut sheet = TileSheet { id: "mine".into(), image: "dev_mine".into(), sheet_w: 8, sheet_h: 1, tile_w: 16, tile_h: 16, ..Default::default() };
    sheet.set_tile_prop(0, "Type", s("Stone"));
    let mut b = B::new("Mine", w, h, sheet);
    b.fill("Back", 0, 0, w, h, 0);
    for x in 0..w {
        b.set("Buildings", x, 0, 2);
        b.set("Buildings", x, 1, 3);
        if x != 5 {
            b.set("Buildings", x, h - 1, 2);
        }
    }
    for y in 0..h {
        b.set("Buildings", 0, y, 2);
        b.set("Buildings", w - 1, y, 2);
    }
    // Descent shaft and elevator.
    b.set_props("Buildings", 5, 3, 4, &[("Action", "Mine")]);
    b.set_props("Buildings", 8, 2, 3, &[("Action", "MineElevator")]);
    b.prop("Warp", "5 9 Town 3 2");
    b.map
}

pub fn seed_shop() -> Map {
    let (w, h) = (12, 11);
    let mut b = B::new("SeedShop", w, h, indoor_sheet());
    b.fill("Back", 0, 0, w, h, VOID);
    b.fill("Back", 1, 3, w - 2, h - 4, FLOOR);
    b.fill("Buildings", 1, 1, w - 2, 1, IWALL_TOP);
    b.fill("Buildings", 1, 2, w - 2, 1, SHELF);
    for y in 1..h - 1 {
        b.set("Buildings", 0, y, IWALL_TOP);
        b.set("Buildings", w - 1, y, IWALL_TOP);
    }
    for x in 0..w {
        b.set("Buildings", x, 0, IWALL_TOP);
        if x != 6 {
            b.set("Buildings", x, h - 1, IWALL_TOP);
        }
    }
    for x in 3..10 {
        b.set_props("Buildings", x, 5, COUNTER, &[("Action", "OpenShop SeedShop")]);
    }
    b.set("Back", 6, h - 1, MAT);
    b.prop("Warp", "6 10 Town 25 9");
    b.map
}

pub fn all() -> Vec<Map> {
    vec![farm(), farmhouse(), town(), seed_shop(), mine_entrance()]
}
