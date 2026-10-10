//! Original placeholder textures. Layouts follow the conventions the
//! renderer expects (16px tiles, 16x32 character frames, 64x64 portraits).

use engine::{Color, Image};

use super::paint::{hash, shade, Cell, OUTLINE};

pub const OUT_COLS: i32 = 8;

// Outdoor sheet tile indices.
pub const GRASS: i32 = 0;
pub const GRASS2: i32 = 1;
pub const DIRT: i32 = 2;
pub const PATH: i32 = 3;
pub const WATER: i32 = 4; // 4..=6 animation frames
pub const SAND: i32 = 7;
pub const WALL_TOP: i32 = 8;
pub const WALL: i32 = 9;
pub const ROOF: i32 = 10;
pub const DOOR: i32 = 11;
pub const WINDOW: i32 = 12;
pub const FENCE: i32 = 13;
pub const CLIFF: i32 = 14;
pub const TRUNK: i32 = 15;
pub const FLOWERS: i32 = 24;
pub const BUSH: i32 = 25;
pub const SIGN: i32 = 26;
pub const MAILBOX: i32 = 27;
pub const LAMP: i32 = 28;
pub const BRIDGE: i32 = 30;
pub const TREE_TOP: i32 = 31;
pub const BIN: i32 = 32;
pub const GARBAGE: i32 = 33;
pub const PET_BOWL: i32 = 34;

// Indoor sheet tile indices.
pub const FLOOR: i32 = 0;
pub const IWALL: i32 = 1;
pub const IWALL_TOP: i32 = 2;
pub const BED_TOP: i32 = 3;
pub const BED_BOTTOM: i32 = 4;
pub const COUNTER: i32 = 5;
pub const RUG: i32 = 6;
pub const TABLE: i32 = 7;
pub const SHELF: i32 = 8;
pub const MAT: i32 = 9;
pub const VOID: i32 = 10;
pub const TV: i32 = 11;

fn grass_color(season: &str) -> (Color, Color) {
    match season {
        "summer" => (Color::rgb(70, 150, 50), Color::rgb(50, 120, 40)),
        "fall" => (Color::rgb(170, 130, 50), Color::rgb(140, 95, 40)),
        "winter" => (Color::rgb(225, 232, 240), Color::rgb(190, 205, 222)),
        _ => (Color::rgb(96, 176, 64), Color::rgb(70, 140, 52)),
    }
}

pub fn outdoors(season: &str) -> Image {
    let mut img = Image::new(16 * OUT_COLS as u32, 16 * 5);
    let (g, gd) = grass_color(season);
    let dirt = Color::rgb(150, 106, 64);
    let water = if season == "winter" { Color::rgb(120, 170, 220) } else { Color::rgb(56, 120, 200) };
    let wood = Color::rgb(140, 92, 52);
    let cell = |img: &mut Image, i: i32| -> Image {
        let _ = (img, i);
        Image::new(0, 0)
    };
    let _ = cell;
    {
        let mut c = Cell::at(&mut img, GRASS, OUT_COLS, 16, 16);
        c.noise(g, 6, 1);
        for i in 0..6 {
            let x = (hash(i, 0, 7) % 15) as i32;
            let y = (hash(i, 1, 7) % 14) as i32;
            c.px(x, y, gd);
            c.px(x, y + 1, gd);
        }
    }
    {
        let mut c = Cell::at(&mut img, GRASS2, OUT_COLS, 16, 16);
        c.noise(g, 6, 2);
        for i in 0..10 {
            let x = (hash(i, 3, 9) % 15) as i32;
            let y = (hash(i, 4, 9) % 14) as i32;
            c.px(x, y, shade(gd, -10));
            c.px(x + 1, y + 1, gd);
        }
    }
    Cell::at(&mut img, DIRT, OUT_COLS, 16, 16).noise(dirt, 8, 3);
    Cell::at(&mut img, PATH, OUT_COLS, 16, 16).noise(Color::rgb(196, 170, 120), 6, 4);
    for f in 0..3 {
        let mut c = Cell::at(&mut img, WATER + f, OUT_COLS, 16, 16);
        c.noise(water, 4, 5 + f as u32);
        for k in 0..3 {
            let y = (k * 5 + f * 2) % 16;
            let x = (k * 7 + f * 3) % 12;
            c.rect(x, y, 4, 1, shade(water, 40));
        }
    }
    Cell::at(&mut img, SAND, OUT_COLS, 16, 16).noise(Color::rgb(222, 200, 140), 6, 8);
    {
        let mut c = Cell::at(&mut img, WALL_TOP, OUT_COLS, 16, 16);
        c.noise(Color::rgb(170, 60, 50), 6, 10);
        for y in (0..16).step_by(4) {
            c.rect(0, y, 16, 1, Color::rgb(120, 40, 36));
        }
    }
    {
        let mut c = Cell::at(&mut img, WALL, OUT_COLS, 16, 16);
        c.noise(Color::rgb(214, 190, 150), 4, 11);
        for y in (3..16).step_by(4) {
            c.rect(0, y, 16, 1, Color::rgb(180, 150, 110));
        }
    }
    {
        let mut c = Cell::at(&mut img, ROOF, OUT_COLS, 16, 16);
        c.noise(Color::rgb(150, 50, 44), 6, 12);
        for y in (0..16).step_by(3) {
            c.rect(0, y, 16, 1, Color::rgb(110, 34, 30));
        }
    }
    {
        let mut c = Cell::at(&mut img, DOOR, OUT_COLS, 16, 16);
        c.noise(Color::rgb(214, 190, 150), 4, 11);
        c.rect(3, 1, 10, 15, wood);
        c.outline_rect(3, 1, 10, 15, OUTLINE);
        c.px(10, 9, Color::YELLOW);
    }
    {
        let mut c = Cell::at(&mut img, WINDOW, OUT_COLS, 16, 16);
        c.noise(Color::rgb(214, 190, 150), 4, 11);
        c.rect(3, 3, 10, 9, Color::rgb(150, 200, 230));
        c.outline_rect(3, 3, 10, 9, OUTLINE);
        c.rect(8, 3, 1, 9, OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, FENCE, OUT_COLS, 16, 16);
        c.rect(2, 4, 3, 12, wood);
        c.rect(11, 4, 3, 12, wood);
        c.rect(0, 7, 16, 2, shade(wood, 20));
        c.rect(0, 12, 16, 2, shade(wood, 20));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, CLIFF, OUT_COLS, 16, 16);
        c.noise(Color::rgb(120, 110, 100), 12, 14);
        c.rect(0, 0, 16, 2, g);
    }
    {
        let mut c = Cell::at(&mut img, TRUNK, OUT_COLS, 16, 16);
        c.rect(5, 0, 6, 14, Color::rgb(110, 72, 40));
        c.rect(3, 13, 10, 3, Color::rgb(90, 60, 34));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, FLOWERS, OUT_COLS, 16, 16);
        c.noise(g, 6, 1);
        for (i, col) in [Color::rgb(240, 90, 120), Color::rgb(250, 230, 90), Color::rgb(130, 140, 250)].iter().enumerate() {
            let x = 2 + i as i32 * 5;
            let y = 3 + (i as i32 * 4) % 9;
            c.rect(x, y, 2, 2, *col);
        }
    }
    {
        let mut c = Cell::at(&mut img, BUSH, OUT_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 7.5, 6.5, shade(gd, -10));
        c.ellipse(7.0, 8.0, 5.0, 4.0, gd);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, SIGN, OUT_COLS, 16, 16);
        c.rect(7, 8, 2, 8, wood);
        c.rect(2, 2, 12, 7, shade(wood, 30));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, MAILBOX, OUT_COLS, 16, 16);
        c.rect(7, 8, 2, 8, wood);
        c.rect(3, 2, 10, 7, Color::rgb(70, 110, 180));
        c.rect(12, 1, 1, 4, Color::RED);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, LAMP, OUT_COLS, 16, 16);
        c.rect(7, 4, 2, 12, Color::rgb(60, 60, 70));
        c.rect(5, 0, 6, 5, Color::rgb(250, 220, 120));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 29, OUT_COLS, 16, 16);
        c.rect(0, 0, 16, 16, Color::BLACK);
    }
    {
        let mut c = Cell::at(&mut img, BRIDGE, OUT_COLS, 16, 16);
        c.noise(shade(wood, 20), 6, 30);
        for x in (0..16).step_by(4) {
            c.rect(x, 0, 1, 16, shade(wood, -30));
        }
    }
    {
        let mut c = Cell::at(&mut img, TREE_TOP, OUT_COLS, 16, 16);
        c.ellipse(8.0, 8.0, 8.0, 8.0, shade(gd, -15));
        c.ellipse(7.0, 7.0, 5.5, 5.5, gd);
    }
    {
        let mut c = Cell::at(&mut img, BIN, OUT_COLS, 16, 16);
        c.rect(1, 4, 14, 12, shade(wood, 10));
        c.rect(0, 2, 16, 4, Color::rgb(150, 60, 50));
        for x in (3..14).step_by(4) {
            c.rect(x, 7, 1, 9, shade(wood, -30));
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, GARBAGE, OUT_COLS, 16, 16);
        let metal = Color::rgb(120, 135, 140);
        c.rect(3, 5, 10, 11, metal);
        for x in (5..12).step_by(3) {
            c.rect(x, 6, 1, 9, shade(metal, -25));
        }
        c.rect(2, 3, 12, 2, shade(metal, 20));
        c.rect(7, 1, 2, 2, shade(metal, -20));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, PET_BOWL, OUT_COLS, 16, 16);
        c.noise(g, 6, 1);
        c.ellipse(8.0, 11.0, 7.0, 4.0, OUTLINE);
        c.ellipse(8.0, 11.0, 6.0, 3.0, Color::rgb(70, 110, 190));
        c.ellipse(8.0, 10.0, 4.0, 1.5, Color::rgb(40, 60, 110));
    }
    img
}

pub const IN_COLS: i32 = 8;

pub fn indoors() -> Image {
    let mut img = Image::new(16 * IN_COLS as u32, 32);
    let wood = Color::rgb(176, 120, 70);
    {
        let mut c = Cell::at(&mut img, FLOOR, IN_COLS, 16, 16);
        c.noise(wood, 5, 40);
        for y in (0..16).step_by(4) {
            c.rect(0, y, 16, 1, shade(wood, -35));
        }
        c.rect(7, 0, 1, 4, shade(wood, -35));
        c.rect(3, 4, 1, 4, shade(wood, -35));
        c.rect(11, 8, 1, 4, shade(wood, -35));
    }
    {
        let mut c = Cell::at(&mut img, IWALL, IN_COLS, 16, 16);
        c.noise(Color::rgb(220, 200, 160), 3, 41);
        for x in (0..16).step_by(4) {
            c.rect(x, 0, 1, 16, Color::rgb(200, 176, 136));
        }
        c.rect(0, 14, 16, 2, Color::rgb(120, 80, 50));
    }
    Cell::at(&mut img, IWALL_TOP, IN_COLS, 16, 16).noise(Color::rgb(100, 70, 50), 4, 42);
    {
        let mut c = Cell::at(&mut img, BED_TOP, IN_COLS, 16, 16);
        c.rect(1, 2, 14, 14, Color::rgb(120, 80, 50));
        c.rect(2, 4, 12, 6, Color::WHITE);
        c.rect(2, 10, 12, 6, Color::rgb(80, 120, 200));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, BED_BOTTOM, IN_COLS, 16, 16);
        c.rect(1, 0, 14, 14, Color::rgb(80, 120, 200));
        c.rect(1, 12, 14, 3, Color::rgb(120, 80, 50));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, COUNTER, IN_COLS, 16, 16);
        c.rect(0, 2, 16, 14, shade(wood, -20));
        c.rect(0, 2, 16, 3, shade(wood, 20));
    }
    Cell::at(&mut img, RUG, IN_COLS, 16, 16).noise(Color::rgb(170, 60, 80), 8, 43);
    {
        let mut c = Cell::at(&mut img, TABLE, IN_COLS, 16, 16);
        c.noise(wood, 5, 40);
        c.rect(1, 3, 14, 8, shade(wood, 30));
        c.rect(2, 11, 2, 5, shade(wood, -30));
        c.rect(12, 11, 2, 5, shade(wood, -30));
    }
    {
        let mut c = Cell::at(&mut img, SHELF, IN_COLS, 16, 16);
        c.rect(0, 0, 16, 16, shade(wood, -30));
        for (i, y) in [3, 8, 13].iter().enumerate() {
            c.rect(1, *y, 14, 1, shade(wood, 20));
            for k in 0..4 {
                let col = [Color::RED, Color::GREEN, Color::YELLOW, Color::rgb(120, 120, 240)][(k + i) % 4];
                c.rect(2 + k as i32 * 3, y - 3, 2, 3, col);
            }
        }
    }
    Cell::at(&mut img, MAT, IN_COLS, 16, 16).noise(Color::rgb(140, 160, 90), 6, 44);
    Cell::at(&mut img, VOID, IN_COLS, 16, 16).rect(0, 0, 16, 16, Color::BLACK);
    {
        let mut c = Cell::at(&mut img, TV, IN_COLS, 16, 16);
        c.rect(1, 3, 14, 11, Color::rgb(50, 50, 60));
        c.rect(3, 5, 10, 7, Color::rgb(100, 160, 200));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Item icon sheet, 24 columns of 16x16 cells (same grid as the game's object sheet).
pub const OBJ_COLS: i32 = 24;
pub const ICON_TURNIP_SEEDS: i32 = 0;
pub const ICON_TURNIP: i32 = 1;
pub const ICON_BEAN_SEEDS: i32 = 2;
pub const ICON_BEAN: i32 = 3;
pub const ICON_PUMPKIN_SEEDS: i32 = 4;
pub const ICON_PUMPKIN: i32 = 5;
pub const ICON_WOOD: i32 = 6;
pub const ICON_STONE: i32 = 7;
pub const ICON_FIBER: i32 = 8;
pub const ICON_SAP: i32 = 9;
pub const ICON_BREAD: i32 = 10;
pub const ICON_WEEDS: i32 = 11;
pub const ICON_TWIG: i32 = 12;
pub const ICON_ROCK: i32 = 13;
pub const ICON_BERRY: i32 = 14;
pub const ICON_CHEST: i32 = 15;
pub const ICON_JUICE: i32 = 16;
pub const ICON_SPRINKLER: i32 = 17;
pub const ICON_FERTILIZER: i32 = 18;
pub const ICON_SPEEDGRO: i32 = 19;
pub const ICON_PERCH: i32 = 20;
pub const ICON_DARTER: i32 = 21;
pub const ICON_DRIFTWOOD: i32 = 22;
pub const ICON_COPPER_ORE: i32 = 23;
pub const ICON_IRON_ORE: i32 = 24;
pub const ICON_GOLD_ORE: i32 = 25;
pub const ICON_COAL: i32 = 26;
pub const ICON_MINE_ROCK: i32 = 27;
pub const ICON_COPPER_NODE: i32 = 28;
pub const ICON_IRON_NODE: i32 = 29;
pub const ICON_GOLD_NODE: i32 = 30;
pub const ICON_GOO: i32 = 31;
pub const ICON_DUST: i32 = 32;
pub const ICON_SALAD: i32 = 33;
pub const ICON_STEW: i32 = 34;
pub const ICON_EGG: i32 = 35;
pub const ICON_MILK: i32 = 36;
pub const ICON_HAY: i32 = 37;
pub const ICON_COPPER_BAR: i32 = 38;
pub const ICON_IRON_BAR: i32 = 39;
pub const ICON_GOLD_BAR: i32 = 40;
pub const ICON_IRIDIUM_BAR: i32 = 41;
pub const ICON_GEODE: i32 = 42;
pub const ICON_QUARTZ: i32 = 43;
pub const ICON_AMBERITE: i32 = 44;
pub const ICON_BOUQUET: i32 = 45;
pub const ICON_PENDANT: i32 = 46;
pub const ICON_PLUM_SAPLING: i32 = 47;
pub const ICON_PLUM: i32 = 48;
pub const ICON_TREE_SEED: i32 = 49;
pub const ICON_SYRUP: i32 = 50;
pub const ICON_ARTIFACT_SPOT: i32 = 51;
pub const ICON_OLD_COIN: i32 = 52;
pub const ICON_GUARD_RING: i32 = 53;
pub const ICON_MAGNET_RING: i32 = 54;
pub const ICON_BOOTS: i32 = 55;
pub const ICON_BOMB: i32 = 56;
pub const ICON_CRAB_POT: i32 = 57;
pub const ICON_BAIT: i32 = 58;
pub const ICON_CRAWLER: i32 = 59;
pub const ICON_BATTERY: i32 = 60;

fn seed_packet(c: &mut Cell, col: Color) {
    c.rect(3, 2, 10, 13, Color::rgb(230, 215, 170));
    c.rect(5, 5, 6, 6, col);
    c.rect(3, 2, 10, 2, shade(col, -40));
    c.auto_outline(OUTLINE);
}

pub fn objects() -> Image {
    let mut img = Image::new(16 * OBJ_COLS as u32, 16 * 4);
    let turnip = Color::rgb(240, 230, 230);
    let bean = Color::rgb(80, 170, 70);
    let pumpkin = Color::rgb(235, 130, 40);
    seed_packet(&mut Cell::at(&mut img, ICON_TURNIP_SEEDS, OBJ_COLS, 16, 16), Color::rgb(200, 120, 200));
    {
        let mut c = Cell::at(&mut img, ICON_TURNIP, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 5.0, 4.5, turnip);
        c.ellipse(8.0, 8.0, 4.0, 2.0, Color::rgb(190, 110, 200));
        c.rect(6, 1, 1, 5, bean);
        c.rect(9, 1, 1, 5, bean);
        c.rect(8, 15, 1, 1, turnip);
        c.auto_outline(OUTLINE);
    }
    seed_packet(&mut Cell::at(&mut img, ICON_BEAN_SEEDS, OBJ_COLS, 16, 16), bean);
    {
        let mut c = Cell::at(&mut img, ICON_BEAN, OBJ_COLS, 16, 16);
        for i in 0..4 {
            c.ellipse(5.0 + i as f32 * 2.0, 12.0 - i as f32 * 2.5, 2.2, 2.2, shade(bean, i * 6));
        }
        c.auto_outline(OUTLINE);
    }
    seed_packet(&mut Cell::at(&mut img, ICON_PUMPKIN_SEEDS, OBJ_COLS, 16, 16), pumpkin);
    {
        let mut c = Cell::at(&mut img, ICON_PUMPKIN, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 7.0, 5.0, pumpkin);
        c.rect(5, 6, 1, 9, shade(pumpkin, -40));
        c.rect(10, 6, 1, 9, shade(pumpkin, -40));
        c.rect(7, 2, 2, 4, Color::rgb(90, 120, 40));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_WOOD, OBJ_COLS, 16, 16);
        c.rect(2, 6, 12, 6, Color::rgb(150, 100, 56));
        c.ellipse(13.0, 9.0, 2.0, 3.0, Color::rgb(210, 170, 110));
        c.rect(2, 8, 10, 1, Color::rgb(110, 70, 40));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_STONE, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 6.0, 4.5, Color::rgb(150, 150, 160));
        c.ellipse(6.5, 8.5, 2.5, 1.5, Color::rgb(190, 190, 200));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_FIBER, OBJ_COLS, 16, 16);
        for i in 0..5 {
            c.rect(4 + i * 2, 3 + (i % 2) * 2, 1, 11, Color::rgb(120, 170, 70));
        }
        c.rect(3, 9, 11, 2, Color::rgb(170, 130, 70));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_SAP, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 4.0, 4.0, Color::rgb(200, 140, 60));
        c.ellipse(8.0, 6.0, 2.0, 3.0, Color::rgb(200, 140, 60));
        c.px(7, 9, Color::WHITE);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BREAD, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 7.0, 4.5, Color::rgb(210, 160, 90));
        c.rect(5, 7, 1, 3, Color::rgb(240, 210, 150));
        c.rect(9, 7, 1, 3, Color::rgb(240, 210, 150));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_WEEDS, OBJ_COLS, 16, 16);
        for i in 0..6 {
            let x = 2 + i * 2;
            let h = 6 + (hash(i, 0, 99) % 6) as i32;
            c.rect(x, 15 - h, 2, h, shade(Color::rgb(70, 140, 50), (i % 3) * 12));
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_TWIG, OBJ_COLS, 16, 16);
        for i in 0..10 {
            c.rect(3 + i, 12 - i / 2, 2, 2, Color::rgb(130, 90, 50));
        }
        c.rect(8, 6, 1, 3, Color::rgb(130, 90, 50));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_ROCK, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 7.0, 5.5, Color::rgb(130, 130, 140));
        c.ellipse(6.0, 8.0, 3.0, 2.0, Color::rgb(170, 170, 180));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BERRY, OBJ_COLS, 16, 16);
        for (x, y) in [(6.0, 9.0), (10.0, 9.0), (8.0, 12.0)] {
            c.ellipse(x, y, 2.5, 2.5, Color::rgb(140, 60, 170));
        }
        c.rect(8, 4, 1, 4, Color::rgb(80, 130, 50));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_CHEST, OBJ_COLS, 16, 16);
        c.rect(2, 5, 12, 10, Color::rgb(160, 104, 56));
        c.rect(2, 5, 12, 3, Color::rgb(190, 130, 70));
        c.rect(7, 8, 2, 3, Color::YELLOW);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_JUICE, OBJ_COLS, 16, 16);
        c.rect(5, 2, 6, 3, Color::rgb(200, 200, 210));
        c.rect(4, 5, 8, 10, Color::rgb(230, 200, 120));
        c.rect(5, 8, 6, 6, Color::rgb(240, 140, 60));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_SPRINKLER, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 11.0, 6.0, 3.0, Color::rgb(150, 150, 160));
        c.rect(7, 4, 2, 7, Color::rgb(120, 120, 130));
        c.rect(5, 3, 6, 2, Color::rgb(90, 140, 220));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_FERTILIZER, OBJ_COLS, 16, 16);
        c.rect(3, 4, 10, 11, Color::rgb(180, 150, 100));
        c.rect(3, 4, 10, 2, Color::rgb(140, 110, 70));
        c.rect(6, 8, 4, 4, Color::rgb(100, 70, 40));
        c.auto_outline(OUTLINE);
    }
    for (idx, body, fin) in [(ICON_PERCH, Color::rgb(120, 170, 90), Color::rgb(220, 120, 60)), (ICON_DARTER, Color::rgb(90, 140, 210), Color::rgb(230, 230, 250))] {
        let mut c = Cell::at(&mut img, idx, OBJ_COLS, 16, 16);
        c.ellipse(7.5, 8.5, 5.5, 3.5, body);
        c.rect(12, 6, 3, 2, fin);
        c.rect(12, 9, 3, 2, fin);
        c.rect(6, 4, 3, 2, fin);
        c.px(4, 7, OUTLINE);
        c.auto_outline(OUTLINE);
    }
    let ore_colors = [
        (ICON_COPPER_ORE, ICON_COPPER_NODE, Color::rgb(210, 120, 70)),
        (ICON_IRON_ORE, ICON_IRON_NODE, Color::rgb(200, 200, 215)),
        (ICON_GOLD_ORE, ICON_GOLD_NODE, Color::rgb(245, 205, 60)),
    ];
    for (ore, node, col) in ore_colors {
        {
            let mut c = Cell::at(&mut img, ore, OBJ_COLS, 16, 16);
            c.ellipse(8.0, 10.0, 5.5, 4.5, Color::rgb(110, 100, 95));
            c.rect(5, 8, 2, 2, col);
            c.rect(9, 10, 2, 2, col);
            c.rect(7, 12, 2, 1, col);
            c.auto_outline(OUTLINE);
        }
        {
            let mut c = Cell::at(&mut img, node, OBJ_COLS, 16, 16);
            c.ellipse(8.0, 10.0, 7.0, 5.5, Color::rgb(120, 112, 108));
            for (x, y) in [(4, 7), (9, 9), (6, 12), (11, 6)] {
                c.rect(x, y, 2, 2, col);
            }
            c.auto_outline(OUTLINE);
        }
    }
    {
        let mut c = Cell::at(&mut img, ICON_COAL, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 5.0, 4.0, Color::rgb(40, 40, 46));
        c.px(6, 8, Color::rgb(90, 90, 100));
        c.auto_outline(Color::rgb(10, 10, 12));
    }
    {
        let mut c = Cell::at(&mut img, ICON_MINE_ROCK, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 7.0, 5.5, Color::rgb(105, 98, 110));
        c.ellipse(6.0, 8.0, 3.0, 2.0, Color::rgb(140, 132, 145));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_GOO, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 5.0, 4.0, Color::rgb(130, 95, 60));
        c.px(6, 8, Color::rgb(190, 160, 120));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_DUST, OBJ_COLS, 16, 16);
        for (x, y) in [(5, 6), (9, 5), (7, 9), (10, 10), (4, 11)] {
            c.rect(x, y, 2, 2, Color::rgb(200, 190, 210));
        }
        c.auto_outline(OUTLINE);
    }
    for (idx, col) in [
        (ICON_COPPER_BAR, Color::rgb(220, 130, 80)),
        (ICON_IRON_BAR, Color::rgb(200, 205, 220)),
        (ICON_GOLD_BAR, Color::rgb(250, 210, 70)),
        (ICON_IRIDIUM_BAR, Color::rgb(170, 90, 220)),
    ] {
        let mut c = Cell::at(&mut img, idx, OBJ_COLS, 16, 16);
        for y in 0..5 {
            c.rect(3 + y / 2, 6 + y, 10 - y, 1, shade(col, 20 - y * 10));
        }
        c.rect(3, 11, 10, 2, shade(col, -40));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_GEODE, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 6.0, 5.0, Color::rgb(150, 130, 110));
        c.ellipse(6.0, 7.0, 2.0, 1.5, Color::rgb(180, 160, 140));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_QUARTZ, OBJ_COLS, 16, 16);
        c.rect(6, 3, 4, 10, Color::rgb(235, 240, 250));
        c.rect(7, 2, 2, 1, Color::rgb(235, 240, 250));
        c.rect(8, 3, 1, 9, Color::rgb(200, 210, 230));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_AMBERITE, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 4.5, 5.0, Color::rgb(240, 160, 40));
        c.px(7, 7, Color::rgb(255, 230, 160));
        c.auto_outline(OUTLINE);
    }
    for (idx, gem) in [(ICON_GUARD_RING, Color::rgb(80, 160, 230)), (ICON_MAGNET_RING, Color::rgb(220, 70, 70))] {
        let mut c = Cell::at(&mut img, idx, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 5.0, 4.0, Color::rgb(200, 170, 80));
        c.ellipse(8.0, 10.0, 3.0, 2.2, Color::TRANSPARENT);
        c.ellipse(8.0, 5.0, 2.2, 2.2, gem);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BOOTS, OBJ_COLS, 16, 16);
        c.rect(4, 3, 5, 9, Color::rgb(120, 80, 50));
        c.rect(4, 11, 9, 3, Color::rgb(100, 65, 40));
        c.rect(4, 13, 9, 1, Color::rgb(60, 40, 30));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BOMB, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 4.5, 4.5, Color::rgb(45, 45, 55));
        c.px(6, 8, Color::rgb(120, 120, 140));
        c.rect(8, 3, 1, 3, Color::rgb(150, 120, 80));
        c.px(9, 2, Color::rgb(250, 200, 80));
        c.auto_outline(Color::rgb(15, 15, 20));
    }
    {
        let mut c = Cell::at(&mut img, ICON_CRAB_POT, OBJ_COLS, 16, 16);
        c.rect(3, 6, 10, 8, Color::rgb(150, 110, 70));
        for x in (4..13).step_by(3) {
            c.rect(x, 6, 1, 8, Color::rgb(100, 70, 45));
        }
        c.rect(3, 5, 10, 1, Color::rgb(90, 60, 40));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BAIT, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 5.0, 4.0, Color::rgb(170, 120, 90));
        c.px(6, 9, Color::rgb(220, 180, 150));
        c.px(10, 11, Color::rgb(220, 180, 150));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_CRAWLER, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 4.5, 3.0, Color::rgb(170, 70, 50));
        c.rect(3, 7, 2, 2, Color::rgb(200, 90, 60));
        c.rect(11, 7, 2, 2, Color::rgb(200, 90, 60));
        for x in [5, 7, 9, 11] {
            c.px(x, 13, Color::rgb(140, 60, 40));
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BATTERY, OBJ_COLS, 16, 16);
        c.rect(5, 4, 6, 10, Color::rgb(70, 80, 100));
        c.rect(6, 2, 4, 2, Color::rgb(180, 180, 190));
        c.rect(7, 6, 2, 3, Color::rgb(250, 220, 60));
        c.rect(6, 8, 2, 3, Color::rgb(250, 220, 60));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_PLUM_SAPLING, OBJ_COLS, 16, 16);
        c.rect(7, 6, 2, 9, Color::rgb(120, 80, 50));
        c.ellipse(8.0, 5.0, 4.0, 3.0, Color::rgb(90, 160, 70));
        c.rect(4, 13, 8, 2, Color::rgb(130, 100, 70));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_PLUM, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 5.0, 5.0, Color::rgb(130, 60, 140));
        c.px(6, 8, Color::rgb(200, 150, 210));
        c.rect(8, 3, 1, 3, Color::rgb(90, 60, 40));
        c.rect(9, 3, 3, 2, Color::rgb(90, 160, 70));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_TREE_SEED, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 4.0, 4.5, Color::rgb(170, 120, 70));
        c.rect(5, 5, 6, 3, Color::rgb(120, 85, 50));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_SYRUP, OBJ_COLS, 16, 16);
        c.rect(6, 2, 4, 3, Color::rgb(220, 220, 220));
        c.rect(4, 5, 8, 10, Color::rgb(170, 90, 30));
        c.rect(5, 7, 2, 4, Color::rgb(220, 150, 80));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_ARTIFACT_SPOT, OBJ_COLS, 16, 16);
        for (x, y) in [(5, 10), (8, 9), (11, 11)] {
            c.rect(x, y, 1, 4, Color::rgb(120, 80, 60));
            c.px(x, y - 1, Color::rgb(150, 100, 80));
        }
    }
    {
        let mut c = Cell::at(&mut img, ICON_OLD_COIN, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 5.0, 5.0, Color::rgb(190, 160, 80));
        c.ellipse(8.0, 9.0, 3.0, 3.0, Color::rgb(160, 130, 60));
        c.px(8, 9, Color::rgb(220, 200, 120));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BOUQUET, OBJ_COLS, 16, 16);
        c.rect(7, 9, 2, 6, Color::rgb(70, 140, 50));
        for (x, y, col) in [(5, 5, Color::rgb(240, 90, 120)), (9, 4, Color::rgb(250, 220, 90)), (7, 7, Color::rgb(130, 140, 250)), (10, 7, Color::rgb(240, 120, 60))] {
            c.ellipse(x as f32, y as f32, 2.2, 2.2, col);
        }
        c.rect(5, 10, 6, 2, Color::rgb(230, 230, 240));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_PENDANT, OBJ_COLS, 16, 16);
        for i in 0..5 {
            c.px(4 + i, 2 + i, Color::rgb(220, 200, 120));
            c.px(11 - i, 2 + i, Color::rgb(220, 200, 120));
        }
        c.ellipse(8.0, 10.0, 3.0, 3.5, Color::rgb(90, 200, 220));
        c.px(7, 9, Color::WHITE);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_EGG, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 4.0, 5.5, Color::rgb(240, 225, 200));
        c.px(6, 6, Color::WHITE);
        c.px(9, 11, Color::rgb(200, 170, 140));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_MILK, OBJ_COLS, 16, 16);
        c.rect(5, 3, 6, 2, Color::rgb(170, 120, 80));
        c.rect(4, 5, 8, 10, Color::rgb(245, 245, 245));
        c.rect(4, 9, 8, 2, Color::rgb(120, 170, 220));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_HAY, OBJ_COLS, 16, 16);
        c.rect(2, 6, 12, 8, Color::rgb(220, 190, 90));
        for x in (3..14).step_by(3) {
            c.rect(x, 6, 1, 8, Color::rgb(190, 160, 70));
        }
        c.rect(2, 9, 12, 1, Color::rgb(150, 100, 50));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_SALAD, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 11.0, 6.5, 3.5, Color::rgb(235, 235, 240));
        c.ellipse(8.0, 9.0, 5.0, 2.5, Color::rgb(90, 180, 70));
        c.px(6, 8, Color::rgb(230, 70, 60));
        c.px(10, 9, Color::rgb(240, 200, 60));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_STEW, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 11.0, 6.5, 4.0, Color::rgb(120, 90, 70));
        c.ellipse(8.0, 9.5, 5.0, 2.0, Color::rgb(200, 120, 60));
        c.px(6, 9, Color::rgb(240, 160, 80));
        c.rect(7, 3, 1, 3, Color::rgb(220, 220, 220));
        c.rect(10, 2, 1, 3, Color::rgb(220, 220, 220));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_DRIFTWOOD, OBJ_COLS, 16, 16);
        for i in 0..10 {
            c.rect(3 + i, 10 - i / 3, 2, 3, Color::rgb(170, 150, 120));
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_SPEEDGRO, OBJ_COLS, 16, 16);
        c.rect(3, 4, 10, 11, Color::rgb(170, 200, 120));
        c.rect(3, 4, 10, 2, Color::rgb(120, 160, 80));
        c.rect(6, 7, 1, 6, Color::rgb(60, 120, 40));
        c.rect(9, 7, 1, 6, Color::rgb(60, 120, 40));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Big craftables: 16x32 cells, 8 columns. 0 = storage box, 1 = juicer, 2 = furnace.
pub fn craftables() -> Image {
    let mut img = Image::new(128, 32);
    let wood = Color::rgb(160, 104, 56);
    {
        let mut c = Cell::at(&mut img, 0, 8, 16, 32);
        c.rect(1, 14, 14, 16, wood);
        c.rect(1, 12, 14, 6, shade(wood, 30));
        c.rect(1, 18, 14, 1, shade(wood, -40));
        c.rect(7, 18, 2, 4, Color::YELLOW);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 1, 8, 16, 32);
        c.rect(2, 10, 12, 20, Color::rgb(120, 150, 170));
        c.rect(3, 6, 10, 4, Color::rgb(200, 200, 210));
        c.rect(5, 14, 6, 8, Color::rgb(240, 140, 60));
        c.rect(2, 26, 12, 2, Color::rgb(80, 100, 120));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 2, 8, 16, 32);
        c.rect(2, 12, 12, 18, Color::rgb(120, 110, 110));
        c.rect(3, 8, 10, 4, Color::rgb(140, 130, 128));
        c.rect(5, 20, 6, 6, Color::rgb(40, 30, 30));
        c.rect(6, 22, 4, 3, Color::rgb(250, 140, 40));
        c.rect(6, 4, 3, 4, Color::rgb(90, 85, 85));
        c.auto_outline(OUTLINE);
    }
    {
        // Lightning rod: a metal mast on a stone base.
        let mut c = Cell::at(&mut img, 4, 8, 16, 32);
        c.rect(7, 2, 2, 22, Color::rgb(170, 170, 185));
        c.rect(6, 1, 4, 2, Color::rgb(220, 200, 90));
        c.rect(4, 24, 8, 6, Color::rgb(120, 115, 110));
        c.auto_outline(OUTLINE);
    }
    {
        // Tapper: a small bucket with a spout.
        let mut c = Cell::at(&mut img, 3, 8, 16, 32);
        c.rect(4, 18, 8, 9, Color::rgb(150, 100, 56));
        c.rect(4, 18, 8, 2, Color::rgb(180, 130, 80));
        c.rect(7, 13, 2, 5, Color::rgb(160, 160, 170));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Crop growth strips: each crop uses 8 frames of 16x32, two crops per 256px row.
pub fn crops() -> Image {
    let mut img = Image::new(256, 32 * 2);
    let crops: [(Color, Color); 3] = [
        (Color::rgb(80, 170, 70), Color::rgb(240, 230, 230)),
        (Color::rgb(60, 150, 60), Color::rgb(80, 170, 70)),
        (Color::rgb(70, 140, 50), Color::rgb(235, 130, 40)),
    ];
    for (ci, (leaf, fruit)) in crops.iter().enumerate() {
        let base_x = (ci as i32 % 2) * 128;
        let base_y = (ci as i32 / 2) * 32;
        for f in 0..8 {
            let mut c = Cell { ox: base_x + f * 16, oy: base_y, w: 16, h: 32, img: &mut img };
            match f {
                0 | 1 => {
                    c.rect(5 + f * 3, 27, 2, 2, Color::rgb(120, 90, 50));
                    c.rect(8, 28 - f, 2, 2, Color::rgb(120, 90, 50));
                }
                2..=5 => {
                    let h = 4 + (f - 2) * 4;
                    c.rect(7, 30 - h, 2, h, *leaf);
                    c.ellipse(5.0, (30 - h) as f32 + 2.0, 3.0, 1.5, *leaf);
                    c.ellipse(11.0, (30 - h) as f32 + 4.0, 3.0, 1.5, shade(*leaf, -15));
                }
                _ => {
                    c.rect(7, 14, 2, 16, *leaf);
                    c.ellipse(4.0, 18.0, 3.5, 2.0, *leaf);
                    c.ellipse(12.0, 20.0, 3.5, 2.0, shade(*leaf, -15));
                    if f == 6 {
                        c.ellipse(8.0, 12.0, 4.5, 4.0, *fruit);
                    }
                }
            }
            c.auto_outline(OUTLINE);
        }
    }
    img
}

/// Tilled soil: 4x4 connection variants (bitmask N=1,E=2,S=4,W=8), dry at
/// x 0..64 and watered at x 64..128.
pub fn hoe_dirt() -> Image {
    let mut img = Image::new(128, 64);
    for watered in 0..2 {
        let base = if watered == 1 { Color::rgb(92, 62, 40) } else { Color::rgb(138, 92, 56) };
        for mask in 0..16 {
            let mut c = Cell { ox: watered * 64 + (mask % 4) * 16, oy: (mask / 4) * 16, w: 16, h: 16, img: &mut img };
            c.noise(base, 6, 50 + mask as u32);
            let edge = shade(base, -30);
            if mask & 1 == 0 {
                c.rect(0, 0, 16, 1, edge);
            }
            if mask & 2 == 0 {
                c.rect(15, 0, 1, 16, edge);
            }
            if mask & 4 == 0 {
                c.rect(0, 15, 16, 1, edge);
            }
            if mask & 8 == 0 {
                c.rect(0, 0, 1, 16, edge);
            }
        }
    }
    img
}

/// Tool icons (16x16, 8 columns): hoe, can, axe, pickaxe, scythe, rod.
pub fn tools() -> Image {
    let mut img = Image::new(128, 16);
    let handle = Color::rgb(140, 92, 52);
    let metal = Color::rgb(170, 170, 185);
    {
        let mut c = Cell::at(&mut img, 0, 8, 16, 16);
        for i in 0..11 {
            c.rect(3 + i, 14 - i, 2, 2, handle);
        }
        c.rect(11, 2, 4, 6, metal);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 1, 8, 16, 16);
        c.rect(3, 6, 9, 8, Color::rgb(100, 150, 200));
        c.rect(11, 7, 4, 2, Color::rgb(100, 150, 200));
        c.rect(5, 3, 5, 3, Color::rgb(100, 150, 200));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 2, 8, 16, 16);
        for i in 0..11 {
            c.rect(3 + i, 14 - i, 2, 2, handle);
        }
        c.rect(9, 1, 6, 5, metal);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 3, 8, 16, 16);
        for i in 0..11 {
            c.rect(3 + i, 14 - i, 2, 2, handle);
        }
        c.rect(7, 1, 9, 2, metal);
        c.rect(14, 1, 2, 5, metal);
        c.rect(7, 1, 2, 4, metal);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 4, 8, 16, 16);
        for i in 0..11 {
            c.rect(3 + i, 14 - i, 2, 2, handle);
        }
        for i in 0..8 {
            c.rect(6 + i, 2 + i / 3, 2, 1, metal);
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 5, 8, 16, 16);
        for i in 0..12 {
            c.rect(2 + i, 14 - i, 1, 1, handle);
        }
        c.rect(14, 2, 1, 12, Color::rgb(220, 220, 220));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 6, 8, 16, 16);
        c.rect(4, 6, 8, 9, Color::rgb(180, 185, 195));
        c.rect(3, 5, 10, 2, Color::rgb(150, 155, 165));
        c.rect(4, 2, 1, 4, Color::rgb(120, 120, 130));
        c.rect(11, 2, 1, 4, Color::rgb(120, 120, 130));
        c.rect(4, 2, 8, 1, Color::rgb(120, 120, 130));
        c.auto_outline(OUTLINE);
    }
    img
}

/// A generic villager/farmer sheet: 4 columns x 4 rows of 16x32 frames.
/// Rows: down, right, up, left. Columns: stand, step A, stand, step B.
pub struct Look {
    pub skin: Color,
    pub hair: Color,
    pub shirt: Color,
    pub legs: Color,
    pub long_hair: bool,
    pub beard: bool,
}

pub fn character(look: &Look) -> Image {
    let mut img = Image::new(64, 128);
    for dir in 0..4 {
        for col in 0..4 {
            let mut c = Cell { ox: col * 16, oy: dir * 32, w: 16, h: 32, img: &mut img };
            let bob = if col % 2 == 1 { 1 } else { 0 };
            let step = match col {
                1 => 1,
                3 => -1,
                _ => 0,
            };
            // Legs.
            let ly = 24;
            match dir {
                0 | 2 => {
                    c.rect(5, ly, 2, 7 - step.max(0), look.legs);
                    c.rect(9, ly, 2, 7 + step.min(0), look.legs);
                }
                _ => {
                    c.rect(6 + step, ly, 2, 7, look.legs);
                    c.rect(8 - step, ly, 2, 7, shade(look.legs, -20));
                }
            }
            // Torso.
            c.rect(4, 15 + bob, 8, 10, look.shirt);
            // Arms.
            if dir == 0 || dir == 2 {
                c.rect(3, 16 + bob, 1, 7, look.shirt);
                c.rect(12, 16 + bob, 1, 7, look.shirt);
                c.px(3, 23 + bob, look.skin);
                c.px(12, 23 + bob, look.skin);
            } else {
                c.rect(7 - step, 16 + bob, 2, 7, shade(look.shirt, -20));
            }
            // Head.
            c.rect(4, 5 + bob, 8, 10, look.skin);
            match dir {
                0 => {
                    c.rect(4, 4 + bob, 8, 3, look.hair);
                    c.px(6, 10 + bob, OUTLINE);
                    c.px(9, 10 + bob, OUTLINE);
                    if look.beard {
                        c.rect(5, 12 + bob, 6, 3, look.hair);
                    }
                    if look.long_hair {
                        c.rect(3, 5 + bob, 1, 10, look.hair);
                        c.rect(12, 5 + bob, 1, 10, look.hair);
                    }
                }
                2 => {
                    c.rect(4, 4 + bob, 8, 10, look.hair);
                    if look.long_hair {
                        c.rect(4, 14 + bob, 8, 4, look.hair);
                    }
                }
                1 => {
                    c.rect(4, 4 + bob, 8, 3, look.hair);
                    c.rect(4, 4 + bob, 3, 8, look.hair);
                    c.px(10, 10 + bob, OUTLINE);
                    if look.beard {
                        c.rect(8, 12 + bob, 4, 3, look.hair);
                    }
                    if look.long_hair {
                        c.rect(4, 12 + bob, 3, 5, look.hair);
                    }
                }
                _ => {
                    c.rect(4, 4 + bob, 8, 3, look.hair);
                    c.rect(9, 4 + bob, 3, 8, look.hair);
                    c.px(5, 10 + bob, OUTLINE);
                    if look.beard {
                        c.rect(4, 12 + bob, 4, 3, look.hair);
                    }
                    if look.long_hair {
                        c.rect(9, 12 + bob, 3, 5, look.hair);
                    }
                }
            }
            c.auto_outline(OUTLINE);
        }
    }
    img
}

/// 2x2 grid of 64x64 portraits: neutral, happy, sad, surprised.
pub fn portrait(look: &Look) -> Image {
    let mut img = Image::new(128, 128);
    for e in 0..4 {
        let mut c = Cell::at(&mut img, e, 2, 64, 64);
        c.rect(0, 0, 64, 64, Color::rgb(230, 210, 170));
        c.rect(14, 50, 36, 14, look.shirt);
        c.ellipse(32.0, 30.0, 17.0, 20.0, look.skin);
        c.ellipse(32.0, 16.0, 18.0, 9.0, look.hair);
        if look.long_hair {
            c.rect(14, 16, 5, 34, look.hair);
            c.rect(45, 16, 5, 34, look.hair);
        }
        if look.beard {
            c.ellipse(32.0, 44.0, 12.0, 6.0, look.hair);
        }
        c.rect(24, 28, 3, 4, OUTLINE);
        c.rect(37, 28, 3, 4, OUTLINE);
        match e {
            1 => {
                c.rect(26, 40, 12, 2, OUTLINE);
                c.rect(25, 38, 2, 2, OUTLINE);
                c.rect(37, 38, 2, 2, OUTLINE);
            }
            2 => {
                c.rect(26, 40, 12, 2, OUTLINE);
                c.rect(25, 42, 2, 2, OUTLINE);
                c.rect(37, 42, 2, 2, OUTLINE);
            }
            3 => c.ellipse(32.0, 41.0, 3.0, 3.0, OUTLINE),
            _ => c.rect(27, 40, 10, 2, OUTLINE),
        }
    }
    img
}

/// Breakable debris and big resources: 16x16 cells (weeds, twig, stone) and a
/// 48x96 tree sprite (canopy + trunk + stump) at x=48.
pub fn debris_and_trees(season: &str) -> Image {
    let mut img = Image::new(96, 96);
    let (g, gd) = grass_color(season);
    let leaf = if season == "winter" { Color::rgb(200, 210, 220) } else if season == "fall" { Color::rgb(200, 110, 40) } else { shade(gd, -10) };
    let _ = g;
    {
        // Tree: canopy 48x64 at (48,0), trunk 16x32 at (48+16, 64), stump 16x16 at (0,48).
        let mut c = Cell { ox: 48, oy: 0, w: 48, h: 96, img: &mut img };
        c.rect(19, 56, 10, 38, Color::rgb(110, 72, 40));
        c.ellipse(24.0, 32.0, 22.0, 26.0, shade(leaf, -15));
        c.ellipse(20.0, 26.0, 15.0, 17.0, leaf);
        c.ellipse(28.0, 40.0, 8.0, 6.0, shade(leaf, 10));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell { ox: 0, oy: 48, w: 16, h: 16, img: &mut img };
        c.rect(3, 6, 10, 9, Color::rgb(110, 72, 40));
        c.ellipse(8.0, 6.0, 5.0, 2.0, Color::rgb(200, 160, 110));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Mine tilesheet: 0/1 floor, 2 rock wall, 3 wall face, 4 void.
pub fn mine_sheet() -> Image {
    let mut img = Image::new(128, 16);
    Cell::at(&mut img, 0, 8, 16, 16).noise(Color::rgb(92, 80, 74), 6, 70);
    {
        let mut c = Cell::at(&mut img, 1, 8, 16, 16);
        c.noise(Color::rgb(92, 80, 74), 6, 71);
        c.rect(4, 6, 3, 2, Color::rgb(70, 60, 56));
        c.rect(10, 11, 2, 2, Color::rgb(110, 98, 90));
    }
    Cell::at(&mut img, 2, 8, 16, 16).noise(Color::rgb(55, 46, 44), 8, 72);
    {
        let mut c = Cell::at(&mut img, 3, 8, 16, 16);
        c.noise(Color::rgb(75, 64, 60), 8, 73);
        c.rect(0, 0, 16, 3, Color::rgb(55, 46, 44));
        c.rect(0, 13, 16, 3, Color::rgb(45, 38, 36));
    }
    Cell::at(&mut img, 4, 8, 16, 16).rect(0, 0, 16, 16, Color::BLACK);
    img
}

/// Mud blob: 4 squash frames of 16x24 in a row (original design).
pub fn monster_blob() -> Image {
    let mut img = Image::new(64, 24);
    let body = Color::rgb(140, 100, 60);
    for f in 0..4 {
        let mut c = Cell { ox: f * 16, oy: 0, w: 16, h: 24, img: &mut img };
        let squash = [0.0, 1.0, 2.0, 1.0][f as usize];
        c.ellipse(8.0, 18.0 - squash / 2.0, 7.0 + squash / 2.0, 5.5 - squash / 2.0, body);
        c.ellipse(6.0, 16.0, 2.5, 1.5, shade(body, 40));
        c.rect(5, 17, 1, 2, OUTLINE);
        c.rect(10, 17, 1, 2, OUTLINE);
        // Leaf sprout.
        let top = (12.0 + squash) as i32;
        c.rect(8, top - 3, 1, 3, Color::rgb(70, 140, 50));
        c.rect(9, top - 4, 3, 2, Color::rgb(90, 170, 60));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Cave moth: 4 wing frames of 16x24 (original design).
pub fn monster_moth() -> Image {
    let mut img = Image::new(64, 24);
    for f in 0..4 {
        let mut c = Cell { ox: f * 16, oy: 0, w: 16, h: 24, img: &mut img };
        let flap = [0, 2, 4, 2][f as usize];
        let wing = Color::rgb(150, 145, 160);
        c.rect(7, 8, 2, 8, Color::rgb(80, 70, 80));
        c.ellipse(4.0, 10.0 + flap as f32 / 2.0, 3.5, 4.0 - flap as f32 / 2.0, wing);
        c.ellipse(12.0, 10.0 + flap as f32 / 2.0, 3.5, 4.0 - flap as f32 / 2.0, wing);
        c.px(4, 10, Color::rgb(230, 140, 60));
        c.px(11, 10, Color::rgb(230, 140, 60));
        c.rect(6, 6, 1, 2, OUTLINE);
        c.rect(9, 6, 1, 2, OUTLINE);
        c.auto_outline(OUTLINE);
    }
    img
}

/// Weapons sheet (16x16, 8 columns): 0 = training sword, 1 = dagger,
/// 2 = club, 3 = slingshot.
pub fn weapons() -> Image {
    let mut img = Image::new(128, 16);
    let steel = Color::rgb(190, 195, 210);
    let wood = Color::rgb(120, 80, 40);
    {
        let mut c = Cell::at(&mut img, 0, 8, 16, 16);
        for i in 0..9 {
            c.rect(5 + i, 10 - i, 2, 2, steel);
        }
        c.rect(3, 10, 5, 2, wood);
        c.rect(4, 9, 2, 5, wood);
        c.rect(2, 13, 2, 2, Color::rgb(150, 100, 50));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 1, 8, 16, 16);
        for i in 0..5 {
            c.rect(7 + i, 8 - i, 2, 2, steel);
        }
        c.rect(5, 9, 4, 2, Color::rgb(90, 70, 50));
        c.rect(3, 11, 3, 3, wood);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 2, 8, 16, 16);
        for i in 0..7 {
            c.rect(3 + i, 12 - i, 2, 2, wood);
        }
        c.ellipse(11.5, 4.5, 3.5, 3.5, Color::rgb(140, 95, 55));
        c.px(11, 3, Color::rgb(170, 120, 70));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 3, 8, 16, 16);
        c.rect(7, 8, 2, 7, wood);
        for i in 0..5 {
            c.rect(6 - i, 8 - i, 2, 1, wood);
            c.rect(8 + i, 8 - i, 2, 1, wood);
        }
        c.rect(2, 3, 12, 1, Color::rgb(200, 180, 150));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Speckled hen: 4x4 frames of 16x16 (rows down, right, up, left). Original design.
pub fn hen() -> Image {
    let mut img = Image::new(64, 64);
    let body = Color::rgb(235, 225, 205);
    let speck = Color::rgb(150, 110, 80);
    for row in 0..4 {
        for f in 0..4 {
            let mut c = Cell { ox: f * 16, oy: row * 16, w: 16, h: 16, img: &mut img };
            let bob = if f % 2 == 1 { 1 } else { 0 };
            c.ellipse(8.0, 10.0 + bob as f32, 5.0, 4.0, body);
            let (hx, hy) = match row {
                1 => (12, 5),
                3 => (4, 5),
                _ => (8, 5),
            };
            c.ellipse(hx as f32, hy as f32 + bob as f32, 2.5, 2.5, body);
            c.px(hx, hy - 2 + bob, Color::rgb(210, 60, 50));
            if row != 2 {
                let bx = match row {
                    1 => hx + 2,
                    3 => hx - 2,
                    _ => hx,
                };
                c.px(bx, hy + 1 + bob, Color::rgb(240, 170, 50));
            }
            for (x, y) in [(6, 9), (9, 11), (10, 8), (7, 12)] {
                c.px(x, y + bob, speck);
            }
            c.rect(6, 14, 1, 2, Color::rgb(240, 170, 50));
            c.rect(9, 14 - (f % 2), 1, 2, Color::rgb(240, 170, 50));
            c.auto_outline(OUTLINE);
        }
    }
    img
}

/// Goat: 4x4 frames of 32x32 (rows down, right, up, left). Original design.
pub fn goat() -> Image {
    let mut img = Image::new(128, 128);
    let fur = Color::rgb(205, 195, 175);
    let dark = Color::rgb(120, 100, 80);
    for row in 0..4 {
        for f in 0..4 {
            let mut c = Cell { ox: f * 32, oy: row * 32, w: 32, h: 32, img: &mut img };
            let step = if f % 2 == 1 { 1 } else { 0 };
            let side = row == 1 || row == 3;
            if side {
                c.ellipse(16.0, 18.0, 10.0, 6.0, fur);
                let hx = if row == 1 { 26 } else { 6 };
                c.ellipse(hx as f32, 12.0, 4.0, 4.0, fur);
                c.rect(hx - 1, 6, 1, 3, dark);
                c.rect(hx + 1, 6, 1, 3, dark);
                c.px(if row == 1 { hx + 2 } else { hx - 2 }, 12, OUTLINE);
                for (lx, off) in [(9, step), (13, 0), (19, step), (23, 0)] {
                    c.rect(lx, 23, 2, 6 - off, dark);
                }
            } else {
                c.ellipse(16.0, 19.0, 7.0, 8.0, fur);
                c.ellipse(16.0, 9.0, 5.0, 5.0, fur);
                c.rect(12, 3, 1, 4, dark);
                c.rect(19, 3, 1, 4, dark);
                if row == 0 {
                    c.px(14, 9, OUTLINE);
                    c.px(18, 9, OUTLINE);
                    c.rect(15, 13, 3, 3, Color::rgb(230, 225, 215));
                }
                c.rect(12, 25, 2, 5 - step, dark);
                c.rect(18, 25, 2, 5, dark);
            }
            c.auto_outline(OUTLINE);
        }
    }
    img
}

/// Farm dog: 4x4 frames of 32x32 (rows down, right, up, left). Original
/// design: a brown mutt with a white chest patch and a dark ear.
pub fn dog() -> Image {
    let mut img = Image::new(128, 128);
    let fur = Color::rgb(165, 110, 65);
    let dark = Color::rgb(95, 60, 35);
    let white = Color::rgb(240, 235, 225);
    for row in 0..4 {
        for f in 0..4 {
            let mut c = Cell { ox: f * 32, oy: row * 32, w: 32, h: 32, img: &mut img };
            let step = if f % 2 == 1 { 1 } else { 0 };
            if row == 1 || row == 3 {
                let right = row == 1;
                let hx = if right { 24 } else { 8 };
                let tail = if right { 5 } else { 27 };
                c.ellipse(16.0, 20.0, 9.0, 5.0, fur);
                c.ellipse(hx as f32, 14.0, 4.5, 4.0, fur);
                let snout = if right { hx + 4 } else { hx - 6 };
                c.rect(snout, 15, 3, 2, fur);
                c.px(if right { snout + 2 } else { snout }, 15, OUTLINE);
                c.rect(if right { hx - 3 } else { hx + 1 }, 10, 2, 5, dark);
                c.px(if right { hx + 1 } else { hx - 2 }, 13, OUTLINE);
                c.rect(if right { 18 } else { 11 }, 21, 4, 3, white);
                c.rect(tail, 15 - step, 2, 4, fur);
                for (lx, off) in [(10, step), (13, 0), (19, step), (22, 0)] {
                    c.rect(lx, 24, 2, 5 - off, fur);
                }
            } else {
                let front = row == 0;
                c.ellipse(16.0, 21.0, 6.5, 7.0, fur);
                c.ellipse(16.0, 11.0, 5.5, 5.0, fur);
                c.rect(10, 9, 2, 6, dark);
                c.rect(21, 9, 2, 6, fur);
                if front {
                    c.px(14, 11, OUTLINE);
                    c.px(18, 11, OUTLINE);
                    c.rect(15, 13, 3, 2, OUTLINE);
                    c.rect(14, 18, 5, 6, white);
                } else {
                    c.rect(15, 25 - step, 2, 4, fur);
                }
                c.rect(12, 26, 2, 4 - step, fur);
                c.rect(19, 26, 2, 4, fur);
            }
            c.auto_outline(OUTLINE);
        }
    }
    img
}

/// Children sheet: 16x16 cells, 9 columns x 2 rows (one row per outfit
/// colour). Columns: 0 newborn bundle, 1 sitting baby, 2-3 crawling,
/// 4-7 toddler walking, 8 crib. Original design.
pub fn child() -> Image {
    let mut img = Image::new(16 * 9, 32);
    let skin = Color::rgb(236, 196, 160);
    let hair = Color::rgb(120, 80, 45);
    for row in 0..2 {
        let suit = if row == 0 { Color::rgb(120, 170, 220) } else { Color::rgb(230, 170, 90) };
        for col in 0..8 {
            let mut c = Cell { ox: col * 16, oy: row * 16, w: 16, h: 16, img: &mut img };
            match col {
                0 => {
                    c.ellipse(8.0, 11.0, 5.0, 3.5, Color::rgb(245, 240, 230));
                    c.ellipse(5.0, 10.0, 2.5, 2.5, skin);
                    c.px(4, 10, OUTLINE);
                }
                1 => {
                    c.ellipse(8.0, 12.0, 4.0, 3.0, suit);
                    c.ellipse(8.0, 7.0, 3.5, 3.5, skin);
                    c.rect(6, 4, 4, 1, hair);
                    c.px(7, 7, OUTLINE);
                    c.px(9, 7, OUTLINE);
                    c.rect(3, 11, 2, 2, skin);
                    c.rect(11, 11, 2, 2, skin);
                }
                2 | 3 => {
                    let step = col - 2;
                    c.ellipse(9.0, 11.0, 4.5, 2.5, suit);
                    c.ellipse(4.5, 9.0, 3.0, 3.0, skin);
                    c.rect(3, 6, 3, 1, hair);
                    c.px(3, 9, OUTLINE);
                    c.rect(4 + step, 13, 2, 2, skin);
                    c.rect(11 - step, 13, 2, 2, suit);
                }
                _ => {
                    let f = col - 4;
                    let bob = f % 2;
                    c.rect(5, 8 + bob, 6, 5, suit);
                    c.ellipse(8.0, 5.0 + bob as f32, 3.5, 3.5, skin);
                    c.rect(5, 1 + bob, 6, 2, hair);
                    c.px(7, 5 + bob, OUTLINE);
                    c.px(9, 5 + bob, OUTLINE);
                    c.rect(5, 13, 2, 3 - (f == 1) as i32, skin);
                    c.rect(9, 13, 2, 3 - (f == 3) as i32, skin);
                }
            }
            c.auto_outline(OUTLINE);
        }
        // Crib.
        let mut c = Cell { ox: 8 * 16, oy: row * 16, w: 16, h: 16, img: &mut img };
        let wood = Color::rgb(190, 140, 90);
        c.rect(1, 6, 14, 1, wood);
        c.rect(1, 13, 14, 2, wood);
        for x in (1..16).step_by(3) {
            c.rect(x, 6, 1, 9, wood);
        }
        c.auto_outline(OUTLINE);
    }
    img
}

/// Traveling cart (48x32, 3x2 tiles). Original design: a wooden wagon
/// with a striped awning, crates and two spoked wheels.
pub fn cart() -> Image {
    let mut img = Image::new(48, 32);
    let mut c = Cell { ox: 0, oy: 0, w: 48, h: 32, img: &mut img };
    let wood = Color::rgb(150, 100, 55);
    // Awning on two poles.
    c.rect(5, 4, 1, 12, Color::rgb(90, 60, 35));
    c.rect(42, 4, 1, 12, Color::rgb(90, 60, 35));
    for i in 0..10 {
        let col = if i % 2 == 0 { Color::rgb(70, 140, 110) } else { Color::rgb(235, 225, 195) };
        c.rect(3 + i * 4, 2, 4, 5, col);
        c.px(4 + i * 4, 7, col);
        c.px(5 + i * 4, 7, col);
    }
    // Crates and a sack on the bed.
    c.rect(9, 12, 8, 6, Color::rgb(185, 140, 80));
    c.rect(9, 14, 8, 1, shade(wood, -20));
    c.rect(19, 10, 9, 8, Color::rgb(170, 125, 70));
    c.rect(23, 10, 1, 8, shade(wood, -20));
    c.ellipse(34.0, 15.0, 5.0, 3.5, Color::rgb(215, 195, 150));
    // Wagon box.
    c.rect(3, 17, 42, 8, wood);
    for x in (3..45).step_by(7) {
        c.rect(x, 17, 1, 8, shade(wood, -30));
    }
    c.rect(3, 17, 42, 1, shade(wood, 25));
    // Wheels.
    for wx in [11.0, 37.0] {
        c.ellipse(wx, 27.0, 4.5, 4.5, Color::rgb(80, 55, 35));
        c.ellipse(wx, 27.0, 2.5, 2.5, Color::rgb(130, 95, 60));
        c.px(wx as i32, 27, Color::rgb(60, 40, 25));
    }
    c.auto_outline(OUTLINE);
    img
}

/// A small shed exterior (width x height tiles of 16px) with a door.
pub fn shed(w: i32, h: i32, wall: Color, roof: Color) -> Image {
    let mut img = Image::new((w * 16) as u32, (h * 16) as u32);
    let (pw, ph) = (w * 16, h * 16);
    let mut c = Cell { ox: 0, oy: 0, w: pw, h: ph, img: &mut img };
    let roof_h = ph / 2;
    for y in 0..roof_h {
        let inset = (roof_h - y) / 3;
        c.rect(inset, y, pw - inset * 2, 1, if y % 4 == 0 { shade(roof, -30) } else { roof });
    }
    c.rect(2, roof_h, pw - 4, ph - roof_h, wall);
    for x in (2..pw - 2).step_by(6) {
        c.rect(x, roof_h, 1, ph - roof_h, shade(wall, -25));
    }
    // Door at the bottom middle tile.
    let dx = (w / 2) * 16 + 3;
    c.rect(dx, ph - 14, 10, 14, Color::rgb(90, 60, 40));
    c.px(dx + 8, ph - 7, Color::YELLOW);
    // Small animal hatch on the left.
    c.rect(6, ph - 9, 8, 9, Color::rgb(60, 40, 30));
    c.auto_outline(OUTLINE);
    img
}

/// Furniture sheet: 32 columns of 16px cells. Pieces start at these indices:
/// 0 chair (1x2), 2 table (2x2), 4 rug (3x2), 8 bookcase (2x3), 11 lamp (1x2).
pub fn furniture() -> Image {
    let mut img = Image::new(512, 48);
    let wood = Color::rgb(160, 104, 56);
    {
        let mut c = Cell { ox: 0, oy: 0, w: 16, h: 32, img: &mut img };
        c.rect(4, 4, 8, 12, shade(wood, 10));
        c.rect(4, 16, 8, 3, shade(wood, 30));
        c.rect(4, 19, 2, 11, shade(wood, -20));
        c.rect(10, 19, 2, 11, shade(wood, -20));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell { ox: 32, oy: 0, w: 32, h: 32, img: &mut img };
        c.rect(2, 10, 28, 8, shade(wood, 25));
        c.rect(2, 16, 28, 2, shade(wood, -15));
        c.rect(4, 18, 3, 12, shade(wood, -25));
        c.rect(25, 18, 3, 12, shade(wood, -25));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell { ox: 64, oy: 0, w: 48, h: 32, img: &mut img };
        c.rect(1, 2, 46, 28, Color::rgb(70, 110, 160));
        c.rect(4, 5, 40, 22, Color::rgb(220, 200, 140));
        c.rect(8, 9, 32, 14, Color::rgb(170, 70, 80));
        for x in (10..38).step_by(6) {
            c.rect(x, 14, 3, 4, Color::rgb(230, 210, 150));
        }
    }
    {
        let mut c = Cell { ox: 128, oy: 0, w: 32, h: 48, img: &mut img };
        c.rect(1, 2, 30, 44, shade(wood, -30));
        for (row, y) in [8, 22, 36].iter().enumerate() {
            c.rect(3, *y, 26, 2, shade(wood, 20));
            for k in 0..6 {
                let col = [Color::rgb(200, 70, 60), Color::rgb(70, 130, 200), Color::rgb(230, 190, 70), Color::rgb(90, 160, 90)][(k + row) % 4];
                c.rect(4 + k as i32 * 4, y - 6, 3, 6, col);
            }
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell { ox: 176, oy: 0, w: 16, h: 32, img: &mut img };
        c.rect(7, 10, 2, 18, Color::rgb(70, 70, 80));
        c.rect(4, 27, 8, 3, Color::rgb(60, 60, 70));
        c.rect(3, 2, 10, 9, Color::rgb(250, 230, 160));
        c.rect(3, 9, 10, 2, Color::rgb(220, 180, 100));
        c.auto_outline(OUTLINE);
    }
    img
}

/// A round silo tower (2x3 tiles).
pub fn silo() -> Image {
    let mut img = Image::new(32, 64);
    let mut c = Cell { ox: 0, oy: 0, w: 32, h: 64, img: &mut img };
    c.rect(4, 14, 24, 48, Color::rgb(200, 190, 170));
    for y in (16..62).step_by(6) {
        c.rect(4, y, 24, 1, Color::rgb(170, 160, 140));
    }
    c.ellipse(16.0, 14.0, 13.0, 10.0, Color::rgb(150, 70, 60));
    c.rect(13, 50, 6, 12, Color::rgb(110, 80, 50));
    c.auto_outline(OUTLINE);
    img
}
