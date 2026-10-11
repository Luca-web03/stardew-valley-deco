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
    b.set_props("Buildings", 10, 8, SHRINE, &[("Action", "Port.Shrine")]);
    b.set("Buildings", 27, 9, BIN);
    for x in 30..34 {
        b.set("Back", x, 9, FLOWERS);
    }
    // Greenhouse shed by the pond.
    b.fill("Front", 30, 26, 4, 1, ROOF);
    b.fill("Buildings", 30, 27, 4, 2, WALL);
    b.set_props("Buildings", 31, 28, DOOR, &[("Action", "Warp 4 7 Greenhouse")]);
    b.prop("Warp", &format!("{} 13 Town 1 15 {} 14 Town 1 16", w, w));
    b.prop("ShippingBinTile", "27 9");
    b.prop("FarmHouseEntry", "21 9");
    b.set("Back", 17, 10, PET_BOWL);
    b.set_props("Buildings", 40, 11, MINECART, &[("Action", "MinecartTransport Default Farm")]);
    b.prop("PetBowlTile", "17 10");
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
    b.set_props("Buildings", 2, 2, TV, &[("Action", "TV")]);

    b.set_props("Buildings", 3, 4, TABLE, &[]);
    b.fill("Back", 3, 6, 3, 2, RUG);
    b.prop("Warp", "5 10 Farm 21 10");
    b.map
}

/// First farmhouse upgrade: a wider house with a kitchen.
pub fn farmhouse1() -> Map {
    let (w, h) = (16, 12);
    let mut b = B::new("FarmHouse1", w, h, indoor_sheet());
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
    b.set_props("Buildings", 2, 2, TV, &[("Action", "TV")]);
    for x in 11..14 {
        b.set_props("Buildings", x, 2, COUNTER, &[("Action", "kitchen")]);
    }
    b.set_props("Buildings", 14, 2, FRIDGE, &[("Action", "Fridge")]);
    b.fill("Back", 3, 6, 3, 2, RUG);
    b.prop("Warp", "5 11 Farm 21 10");
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
    // Movie theater with a box office sign.
    b.house(30, 4, 6, "Theater");
    b.set_props("Buildings", 29, 9, SIGN, &[("Action", "OpenShop Dev_BoxOffice")]);
    b.set_props("Buildings", 18, 11, SIGN, &[("Action", "Message \"Welcome to town!\"")]);
    b.set_props("Buildings", 20, 11, SIGN, &[("Action", "Billboard")]);
    b.set_props("Buildings", 30, 14, SIGN, &[("Action", "AnimalShop")]);
    b.set_props("Buildings", 32, 14, SIGN, &[("Action", "Carpenter")]);
    b.set_props("Buildings", 34, 14, SIGN, &[("Action", "Blacksmith")]);
    b.set_props("Buildings", 36, 14, SIGN, &[("Action", "OpenShop AdventureShop")]);
    b.set_props("Buildings", 37, 17, SIGN, &[("Action", "Port.Travel Dev_Dunes 6 16 300 0900-1700")]);
    b.set_props("Buildings", 16, 11, SIGN, &[("Action", "Bundles")]);
    b.set_props("Buildings", 14, 11, SIGN, &[("Action", "Museum")]);
    b.set_props("Buildings", 12, 11, SIGN, &[("Action", "OpenShop Furniture")]);
    b.set_props("Buildings", 22, 11, SIGN, &[("Action", "SpecialOrders")]);
    b.set_props("Buildings", 10, 11, SIGN, &[("Action", "OpenShop Dev_Tailor")]);
    b.set_props("Buildings", 7, 3, MINECART, &[("Action", "MinecartTransport Default Town")]);
    b.set_props("Buildings", 6, 11, SIGN, &[("Action", "OpenShop Dev_Bookstall")]);
    b.set_props("Buildings", 27, 11, SIGN, &[("Action", "Port.Divorce")]);
    b.set_props("Buildings", 24, 11, SIGN, &[("Action", "Port.JojaMembership")]);
    b.set_props("Buildings", 25, 11, SIGN, &[("Action", "Port.JojaForm")]);
    b.set_props("Buildings", 18, 25, SIGN, &[("Action", "Port.Travel Dev_Isle 4 14 500 0600-1800")]);
    b.set_props("Buildings", 4, 11, LAMP, &[("Action", "Port.Mastery")]);
    for (x, y) in [(4, 5), (33, 12), (6, 20), (35, 20)] {
        b.map_tree(x, y);
    }
    // Cave mouth to the mines in the north-west corner.
    b.fill("Back", 2, 1, 3, 3, PATH);
    b.set("Buildings", 1, 1, CLIFF);
    b.set("Buildings", 5, 1, CLIFF);
    b.set("Buildings", 20, 14, LAMP);
    b.set("Buildings", 28, 14, LAMP);
    b.set_props("Buildings", 19, 9, GARBAGE, &[("Action", "Garbage Dev_Store")]);
    b.set_props("Buildings", 8, 8, GARBAGE, &[("Action", "Garbage Dev_House")]);
    b.prop("Warp", "-1 15 Farm 46 13 -1 16 Farm 46 14 3 0 Mine 5 8");
    b.prop("TravelingCartTile", "28 18");
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

/// Animal house interior with a trough row along the top.
pub fn animal_house(id: &str, w: i32, h: i32) -> Map {
    let mut b = B::new(id, w, h, indoor_sheet());
    b.fill("Back", 0, 0, w, h, VOID);
    b.fill("Back", 1, 2, w - 2, h - 3, MAT);
    for x in 0..w {
        b.set("Buildings", x, 0, IWALL_TOP);
        b.set("Buildings", x, 1, IWALL);
        if x != w / 2 {
            b.set("Buildings", x, h - 1, IWALL_TOP);
        }
    }
    for y in 0..h {
        b.set("Buildings", 0, y, IWALL_TOP);
        b.set("Buildings", w - 1, y, IWALL_TOP);
    }
    for x in 2..w - 2 {
        b.set_props("Back", x, 2, RUG, &[("Trough", "T")]);
    }
    b.set("Back", w / 2, h - 1, MAT);
    b.set_props("Buildings", w - 2, 2, SHELF, &[("Action", "HayHopper")]);
    // Exit warp; the engine retargets it to the building door at runtime.
    b.prop("Warp", &format!("{} {} Farm 0 0", w / 2, h - 1));
    b.map
}

/// Slime hutch interior: an open floor with four troughs on the right.
pub fn slime_hutch() -> Map {
    let (w, h) = (12, 9);
    let mut b = B::new("Dev_SlimeHutch", w, h, indoor_sheet());
    b.fill("Back", 0, 0, w, h, VOID);
    b.fill("Back", 1, 2, w - 2, h - 3, FLOOR);
    for x in 0..w {
        b.set("Buildings", x, 0, IWALL_TOP);
        b.set("Buildings", x, 1, IWALL);
        if x != w / 2 {
            b.set("Buildings", x, h - 1, IWALL_TOP);
        }
    }
    for y in 0..h {
        b.set("Buildings", 0, y, IWALL_TOP);
        b.set("Buildings", w - 1, y, IWALL_TOP);
    }
    for y in 2..6 {
        b.set_props("Buildings", w - 2, y, COUNTER, &[("Port.Trough", "T")]);
    }
    b.set("Back", w / 2, h - 1, MAT);
    b.prop("Warp", &format!("{} {} Farm 0 0", w / 2, h - 1));
    b.map
}

/// A one-room cabin with a bed.
pub fn cabin() -> Map {
    let (w, h) = (8, 8);
    let mut b = B::new("Dev_Cabin", w, h, indoor_sheet());
    b.fill("Back", 0, 0, w, h, VOID);
    b.fill("Back", 1, 2, w - 2, h - 3, FLOOR);
    for x in 0..w {
        b.set("Buildings", x, 0, IWALL_TOP);
        b.set("Buildings", x, 1, IWALL);
        if x != 3 {
            b.set("Buildings", x, h - 1, IWALL_TOP);
        }
    }
    for y in 0..h {
        b.set("Buildings", 0, y, IWALL_TOP);
        b.set("Buildings", w - 1, y, IWALL_TOP);
    }
    b.set_props("Back", 5, 2, BED_TOP, &[("Bed", "T")]);
    b.set_props("Back", 5, 3, BED_BOTTOM, &[("Bed", "T")]);
    b.set("Back", 2, 4, RUG);
    b.set("Back", 3, h - 1, MAT);
    b.prop("Warp", &format!("3 {} Farm 0 0", h - 1));
    b.map
}

pub fn greenhouse() -> Map {
    let (w, h) = (10, 9);
    let mut b = B::new("Greenhouse", w, h, outdoor_sheet());
    b.fill("Back", 0, 0, w, h, PATH);
    b.fill("Back", 1, 1, w - 2, h - 3, DIRT);
    for x in 0..w {
        b.set("Buildings", x, 0, WALL);
        if x != 4 {
            b.set("Buildings", x, h - 1, WALL);
        }
    }
    for y in 0..h {
        b.set("Buildings", 0, y, WALL);
        b.set("Buildings", w - 1, y, WALL);
    }
    b.prop("Outdoors", "F");
    b.prop("Warp", "4 8 Farm 31 29");
    b.map
}

/// Placeholder desert reached by paid travel (original layout): sand,
/// boulders, a small oasis and the mouth of the deep cavern.
pub fn dunes() -> Map {
    let (w, h) = (30, 22);
    let mut b = B::new("Dev_Dunes", w, h, outdoor_sheet());
    b.fill("Back", 0, 0, w, h, SAND);
    b.fill("Back", 4, 16, 22, 2, PATH);
    b.water(19, 6, 5, 3);
    for x in 0..w {
        b.set("Buildings", x, 0, BOULDER);
        b.set("Buildings", x, h - 1, BOULDER);
    }
    for y in 0..h {
        b.set("Buildings", 0, y, BOULDER);
        b.set("Buildings", w - 1, y, BOULDER);
    }
    for (x, y) in [(6, 5), (9, 11), (14, 4), (26, 13), (3, 12), (22, 19), (17, 12)] {
        b.set("Buildings", x, y, BOULDER);
    }
    // Games parlor door.
    b.fill("Buildings", 20, 1, 5, 3, SANDSTONE);
    b.set_props("Buildings", 22, 3, DOOR, &[("Action", "Warp 6 8 Dev_Parlor")]);
    // Cave mouth into the deep cavern.
    b.fill("Buildings", 6, 1, 5, 3, SANDSTONE);
    b.set_props("Buildings", 8, 3, 29, &[("Action", "Mine 121")]);
    // Ride back to town.
    b.set_props("Buildings", 5, 15, SIGN, &[("Action", "Port.Travel Town 36 18 0")]);
    b.set_props("Buildings", 14, 15, MINECART, &[("Action", "MinecartTransport Default Dunes")]);
    b.set_props("Buildings", 20, 15, SIGN, &[("Action", "OpenShop Dev_DuneTrader")]);
    b.map
}

/// A small island (original placeholder): two sandy halves split by a
/// channel, a broken rope bridge repaired with sun shells, and the volcano
/// mouth on the far side.
pub fn isle() -> Map {
    let (w, h) = (26, 18);
    let mut b = B::new("Dev_Isle", w, h, outdoor_sheet());
    b.water(0, 0, w, h);
    b.fill("Back", 2, 2, 10, 14, SAND);
    b.fill("Back", 14, 2, 10, 14, SAND);
    for (x, y) in [(3, 4), (9, 11), (16, 12), (22, 5)] {
        b.set("Buildings", x, y, BUSH);
    }
    // The bridge deck is wood; the broken rope blocks it until repaired.
    for x in 12..14 {
        b.set("Back", x, 8, BRIDGE);
        b.set_props("Buildings", x, 8, FENCE, &[("Port.ClearedBy", "port_isle_bridge")]);
    }
    b.set_props("Buildings", 11, 7, SIGN, &[("Action", "Port.Unlock SunShell 5 port_isle_bridge Repair the rope bridge.")]);
    // Volcano mouth.
    b.fill("Buildings", 18, 2, 5, 2, SANDSTONE);
    b.set_props("Buildings", 20, 3, 29, &[("Action", "Port.Volcano")]);
    // Boat home.
    b.set_props("Buildings", 3, 15, SIGN, &[("Action", "Port.Travel Town 17 24 0")]);
    b.map
}

/// A small games parlor off the dunes (original placeholder).
pub fn parlor() -> Map {
    let (w, h) = (12, 10);
    let mut b = B::new("Dev_Parlor", w, h, indoor_sheet());
    b.fill("Back", 0, 0, w, h, VOID);
    b.fill("Back", 1, 3, w - 2, h - 4, RUG);
    b.fill("Buildings", 1, 1, w - 2, 1, IWALL_TOP);
    b.fill("Buildings", 1, 2, w - 2, 1, IWALL);
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
    for (x, action) in [(2, "Port.Reels"), (3, "Port.Reels"), (5, "Port.HighLow"), (8, "Port.CoinCounter"), (9, "OpenShop Dev_PrizeCounter")] {
        b.set_props("Buildings", x, 4, if action.starts_with("OpenShop") || action == "Port.CoinCounter" { COUNTER } else { TABLE }, &[("Action", action)]);
    }
    b.set("Back", 6, h - 1, MAT);
    b.prop("Warp", "6 9 Dev_Dunes 22 4");
    b.map
}

/// The dunes dressed up for the (invented) Lantern Market festival.
pub fn dunes_market() -> Map {
    let mut map = dunes();
    map.id = "Dev_Dunes_Market".into();
    let mut b = B { map };
    for x in [8, 12, 16, 22, 25] {
        b.set("Buildings", x, 13, LAMP);
    }
    b.set_props("Buildings", 18, 13, BIN, &[("Action", "OpenShop Dev_MarketStall")]);
    b.set_props("Buildings", 19, 13, SIGN, &[("Action", "Message \"Lanterns, snacks and souvenirs!\"")]);
    b.map
}

pub fn all() -> Vec<Map> {
    vec![farm(), farmhouse(), farmhouse1(), town(), seed_shop(), mine_entrance(), animal_house("Coop", 10, 8), animal_house("Barn", 14, 10), slime_hutch(), cabin(), greenhouse(), dunes(), dunes_market(), parlor(), isle()]
}
