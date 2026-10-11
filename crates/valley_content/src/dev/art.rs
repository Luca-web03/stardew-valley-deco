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
pub const BOULDER: i32 = 35;
pub const SANDSTONE: i32 = 36;
pub const MINECART: i32 = 37;
pub const SHRINE: i32 = 38;

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
pub const FRIDGE: i32 = 12;

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
        let mut c = Cell::at(&mut img, MINECART, OUT_COLS, 16, 16);
        c.rect(1, 13, 14, 1, Color::rgb(110, 80, 50));
        c.rect(2, 5, 12, 7, Color::rgb(120, 125, 135));
        c.rect(2, 5, 12, 1, Color::rgb(170, 175, 185));
        c.rect(4, 7, 8, 3, Color::rgb(70, 60, 55));
        c.ellipse(4.5, 12.5, 1.6, 1.6, Color::rgb(50, 50, 55));
        c.ellipse(11.5, 12.5, 1.6, 1.6, Color::rgb(50, 50, 55));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, SANDSTONE, OUT_COLS, 16, 16);
        c.noise(Color::rgb(190, 132, 86), 8, 12);
        for (y, col) in [(3, Color::rgb(214, 160, 108)), (8, Color::rgb(160, 106, 70)), (13, Color::rgb(214, 160, 108))] {
            c.rect(0, y, 16, 1, col);
        }
    }
    {
        let mut c = Cell::at(&mut img, BOULDER, OUT_COLS, 16, 16);
        c.noise(Color::rgb(222, 200, 140), 6, 8);
        c.ellipse(8.0, 9.0, 7.5, 6.5, OUTLINE);
        c.ellipse(8.0, 9.0, 6.5, 5.5, Color::rgb(176, 120, 80));
        c.ellipse(7.0, 7.5, 4.0, 3.0, Color::rgb(204, 150, 100));
        c.rect(5, 12, 6, 1, Color::rgb(140, 92, 60));
    }
    {
        // Shrine: a small stone altar with four candle holders.
        let mut c = Cell::at(&mut img, SHRINE, OUT_COLS, 16, 16);
        c.noise(g, 6, 2);
        c.rect(1, 5, 14, 10, Color::rgb(150, 145, 150));
        c.rect(1, 5, 14, 2, Color::rgb(185, 180, 185));
        c.rect(3, 9, 10, 4, Color::rgb(120, 115, 125));
        for x in [1, 5, 9, 13] {
            c.rect(x, 3, 2, 2, Color::rgb(110, 90, 60));
        }
        c.rect(6, 10, 4, 2, Color::rgb(200, 180, 110));
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
    {
        let mut c = Cell::at(&mut img, FRIDGE, IN_COLS, 16, 16);
        c.rect(2, 0, 12, 16, Color::rgb(235, 238, 240));
        c.rect(2, 6, 12, 1, Color::rgb(170, 180, 190));
        c.rect(11, 2, 1, 3, Color::rgb(120, 130, 140));
        c.rect(11, 8, 1, 5, Color::rgb(120, 130, 140));
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
pub const ICON_HOMING_STONE: i32 = 61;
pub const ICON_DUNE_CHARM: i32 = 62;
pub const ICON_NOTE_SCRAP: i32 = 63;
pub const ICON_BERRY_TART: i32 = 64;
pub const ICON_WHIRL_LURE: i32 = 65;
pub const ICON_CORK_FLOAT: i32 = 66;
pub const ICON_TICKET: i32 = 67;
pub const ICON_BOOK: i32 = 68;
pub const ICON_SUN_SHELL: i32 = 69;
pub const ICON_EMBER_SHARD: i32 = 70;
pub const ICON_RAIN_TOTEM: i32 = 71;
pub const ICON_HARDWOOD: i32 = 72;
pub const ICON_IRIDIUM_ORE: i32 = 73;
pub const ICON_DIAMOND: i32 = 74;
pub const ICON_SLIME_EGG: i32 = 75;
pub const ICON_MUSHROOM: i32 = 76;
pub const ICON_SUNBLOOM_SEEDS: i32 = 77;
pub const ICON_SUNBLOOM: i32 = 78;
pub const ICON_HONEY: i32 = 79;
/// Forge gems: emerald, aquamarine, ruby, amethyst, topaz, jade, then the
/// prismatic shard and the dragon tooth.
pub const ICON_GEMS: i32 = 80;
pub const ICON_PRISMATIC: i32 = 86;
pub const ICON_DRAGON_TOOTH: i32 = 87;
pub const ICON_WOOD_FENCE: i32 = 88;
pub const ICON_STONE_FENCE: i32 = 89;
pub const ICON_GATE: i32 = 90;
pub const ICON_WOOD_FLOOR: i32 = 91;
pub const ICON_STONE_PATH: i32 = 92;
pub const ICON_GRAVEL_PATH: i32 = 93;
pub const ICON_TEA_SAPLING: i32 = 94;
pub const ICON_TEA_LEAVES: i32 = 95;
pub const GEM_COLORS: [(u8, u8, u8); 6] = [(60, 190, 100), (90, 200, 220), (220, 50, 70), (160, 90, 210), (240, 180, 50), (100, 170, 110)];

fn seed_packet(c: &mut Cell, col: Color) {
    c.rect(3, 2, 10, 13, Color::rgb(230, 215, 170));
    c.rect(5, 5, 6, 6, col);
    c.rect(3, 2, 10, 2, shade(col, -40));
    c.auto_outline(OUTLINE);
}

/// A faceted gem in the middle of an icon cell.
fn gem(c: &mut Cell, col: Color) {
    for row in 0..10 {
        let half = if row < 4 { row + 2 } else { 9 - row };
        c.rect(8 - half, 3 + row, half * 2, 1, if row < 3 { shade(col, 40) } else { col });
    }
    c.px(6, 5, Color::rgb(255, 255, 255));
    c.auto_outline(OUTLINE);
}

pub fn objects() -> Image {
    // Rows past the item icons hold 2x2 resource clumps at their sheet
    // indices (600 stump ... 758 mine boulders).
    let mut img = Image::new(16 * OBJ_COLS as u32, 16 * 33);
    clumps(&mut img);
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
        let mut c = Cell::at(&mut img, ICON_SUN_SHELL, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 5.5, 4.5, Color::rgb(250, 200, 120));
        for x in [5, 8, 11] {
            c.rect(x, 6, 1, 6, Color::rgb(220, 150, 80));
        }
        c.rect(6, 13, 4, 1, Color::rgb(220, 150, 80));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_EMBER_SHARD, OBJ_COLS, 16, 16);
        for i in 0..6 {
            c.rect(6 - i / 3, 3 + i, 4 + (i / 2), 1, Color::rgb(230, 90, 40));
        }
        c.rect(7, 9, 3, 4, Color::rgb(200, 60, 30));
        c.px(8, 5, Color::rgb(255, 220, 120));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BOOK, OBJ_COLS, 16, 16);
        c.rect(3, 3, 10, 11, Color::rgb(70, 110, 160));
        c.rect(3, 3, 2, 11, Color::rgb(50, 80, 120));
        c.rect(6, 6, 5, 1, Color::rgb(230, 210, 120));
        c.rect(12, 4, 1, 9, Color::rgb(240, 235, 220));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_TICKET, OBJ_COLS, 16, 16);
        c.rect(2, 4, 12, 8, Color::rgb(240, 200, 90));
        c.rect(5, 4, 1, 8, Color::rgb(200, 150, 60));
        c.rect(7, 6, 5, 1, Color::rgb(150, 60, 50));
        c.rect(7, 9, 4, 1, Color::rgb(150, 60, 50));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_WHIRL_LURE, OBJ_COLS, 16, 16);
        c.rect(7, 2, 1, 5, Color::rgb(180, 180, 190));
        c.ellipse(8.0, 9.0, 3.0, 2.0, Color::rgb(220, 200, 60));
        c.rect(5, 8, 1, 3, Color::rgb(240, 240, 250));
        c.rect(10, 8, 1, 3, Color::rgb(240, 240, 250));
        c.rect(7, 11, 1, 3, Color::rgb(150, 150, 160));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_CORK_FLOAT, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 6.0, 3.5, 3.0, Color::rgb(220, 60, 50));
        c.ellipse(8.0, 10.0, 3.5, 3.0, Color::rgb(200, 160, 110));
        c.rect(8, 1, 1, 3, Color::rgb(80, 80, 90));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_BERRY_TART, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 10.0, 6.0, 3.5, Color::rgb(220, 170, 100));
        c.ellipse(8.0, 9.0, 4.5, 2.2, Color::rgb(150, 40, 90));
        for (x, y) in [(6, 9), (9, 8), (10, 10)] {
            c.px(x, y, Color::rgb(210, 90, 150));
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_NOTE_SCRAP, OBJ_COLS, 16, 16);
        c.rect(3, 3, 10, 11, Color::rgb(240, 230, 200));
        c.px(12, 3, Color::rgb(0, 0, 0));
        for y in [5, 7, 9, 11] {
            c.rect(4, y, 7, 1, Color::rgb(150, 130, 110));
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_HOMING_STONE, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 5.5, 4.5, Color::rgb(120, 140, 170));
        for (x, y) in [(6, 8), (7, 7), (9, 7), (10, 9), (8, 10)] {
            c.px(x, y, Color::rgb(170, 240, 255));
        }
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_DUNE_CHARM, OBJ_COLS, 16, 16);
        c.rect(7, 2, 2, 4, Color::rgb(140, 90, 50));
        c.ellipse(8.0, 10.0, 4.0, 4.5, Color::rgb(230, 170, 80));
        c.ellipse(8.0, 10.0, 2.0, 2.0, Color::rgb(250, 230, 150));
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
        let mut c = Cell::at(&mut img, ICON_HARDWOOD, OBJ_COLS, 16, 16);
        c.rect(2, 6, 12, 6, Color::rgb(110, 60, 35));
        c.rect(2, 6, 12, 1, Color::rgb(150, 90, 50));
        c.ellipse(13.0, 9.0, 2.0, 3.0, Color::rgb(190, 130, 80));
        c.rect(5, 9, 5, 1, Color::rgb(80, 45, 25));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_IRIDIUM_ORE, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 6.0, 5.0, Color::rgb(90, 85, 95));
        c.ellipse(6.0, 8.0, 2.0, 1.5, Color::rgb(200, 90, 220));
        c.ellipse(10.0, 11.0, 1.5, 1.5, Color::rgb(170, 70, 200));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_DIAMOND, OBJ_COLS, 16, 16);
        c.rect(4, 5, 8, 2, Color::rgb(200, 240, 255));
        for i in 0..6 {
            c.rect(4 + i / 2 + 1, 7 + i, 8 - i - 2, 1, Color::rgb(150, 220, 250));
        }
        c.px(6, 5, Color::WHITE);
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_SLIME_EGG, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 9.0, 5.0, 6.0, Color::rgb(120, 200, 90));
        c.ellipse(6.5, 7.0, 1.6, 2.0, Color::rgb(190, 240, 160));
        c.px(10, 11, Color::rgb(70, 140, 60));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_MUSHROOM, OBJ_COLS, 16, 16);
        c.ellipse(8.0, 7.0, 6.0, 4.0, Color::rgb(170, 110, 70));
        c.px(6, 5, Color::rgb(230, 210, 180));
        c.px(10, 6, Color::rgb(230, 210, 180));
        c.rect(6, 9, 4, 5, Color::rgb(235, 225, 200));
        c.auto_outline(OUTLINE);
    }
    seed_packet(&mut Cell::at(&mut img, ICON_SUNBLOOM_SEEDS, OBJ_COLS, 16, 16), Color::rgb(240, 200, 60));
    {
        // Sunbloom: a round yellow flower on a stem.
        let mut c = Cell::at(&mut img, ICON_SUNBLOOM, OBJ_COLS, 16, 16);
        c.rect(7, 8, 2, 7, Color::rgb(70, 150, 60));
        c.ellipse(5.0, 12.0, 2.5, 1.2, Color::rgb(80, 170, 70));
        for (dx, dy) in [(0.0, -3.0), (3.0, 0.0), (0.0, 3.0), (-3.0, 0.0)] {
            c.ellipse(8.0 + dx, 6.0 + dy, 2.2, 2.2, Color::rgb(245, 205, 60));
        }
        c.ellipse(8.0, 6.0, 1.8, 1.8, Color::rgb(150, 90, 40));
        c.auto_outline(OUTLINE);
    }
    {
        // Honey: a squat jar with an amber fill.
        let mut c = Cell::at(&mut img, ICON_HONEY, OBJ_COLS, 16, 16);
        c.rect(4, 5, 8, 10, Color::rgb(230, 160, 40));
        c.rect(4, 5, 8, 2, Color::rgb(250, 200, 90));
        c.rect(5, 2, 6, 3, Color::rgb(170, 120, 70));
        c.px(6, 9, Color::rgb(255, 230, 150));
        c.auto_outline(OUTLINE);
    }
    for (i, (r, g, b)) in GEM_COLORS.iter().enumerate() {
        let col = Color::rgb(*r, *g, *b);
        let mut c = Cell::at(&mut img, ICON_GEMS + i as i32, OBJ_COLS, 16, 16);
        gem(&mut c, col);
    }
    {
        let mut c = Cell::at(&mut img, ICON_PRISMATIC, OBJ_COLS, 16, 16);
        gem(&mut c, Color::rgb(230, 230, 250));
        c.px(6, 6, Color::rgb(240, 90, 90));
        c.px(8, 8, Color::rgb(90, 200, 90));
        c.px(10, 10, Color::rgb(90, 120, 240));
    }
    {
        // Dragon tooth: a long curved fang.
        let mut c = Cell::at(&mut img, ICON_DRAGON_TOOTH, OBJ_COLS, 16, 16);
        for i in 0..9 {
            let w = (5 - i / 2).max(1);
            c.rect(4 + i / 3, 3 + i, w, 1, Color::rgb(235, 225, 200));
        }
        c.px(5, 4, Color::rgb(255, 250, 235));
        c.auto_outline(OUTLINE);
    }
    // Fence and path icons.
    for (icon, post) in [(ICON_WOOD_FENCE, Color::rgb(160, 110, 60)), (ICON_STONE_FENCE, Color::rgb(150, 150, 155))] {
        let mut c = Cell::at(&mut img, icon, OBJ_COLS, 16, 16);
        c.rect(3, 3, 3, 12, post);
        c.rect(10, 3, 3, 12, post);
        c.rect(1, 6, 14, 2, shade(post, 20));
        c.rect(1, 10, 14, 2, shade(post, 20));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_GATE, OBJ_COLS, 16, 16);
        let wood = Color::rgb(170, 120, 70);
        c.rect(2, 4, 12, 10, wood);
        c.rect(2, 8, 12, 1, shade(wood, -40));
        c.rect(7, 4, 1, 10, shade(wood, -40));
        c.px(12, 9, Color::rgb(60, 60, 70));
        c.auto_outline(OUTLINE);
    }
    for (icon, base) in [(ICON_WOOD_FLOOR, Color::rgb(170, 120, 70)), (ICON_STONE_PATH, Color::rgb(160, 160, 165)), (ICON_GRAVEL_PATH, Color::rgb(150, 135, 110))] {
        let mut c = Cell::at(&mut img, icon, OBJ_COLS, 16, 16);
        c.rect(2, 4, 12, 9, base);
        c.rect(2, 4, 12, 2, shade(base, 25));
        c.px(5, 8, shade(base, -35));
        c.px(10, 10, shade(base, -35));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_TEA_SAPLING, OBJ_COLS, 16, 16);
        c.rect(7, 8, 2, 7, Color::rgb(110, 80, 50));
        c.ellipse(8.0, 6.0, 4.0, 3.0, Color::rgb(70, 140, 70));
        c.ellipse(5.0, 9.0, 2.0, 1.5, Color::rgb(90, 160, 80));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, ICON_TEA_LEAVES, OBJ_COLS, 16, 16);
        for (x, y) in [(5.0, 6.0), (10.0, 7.0), (7.0, 11.0)] {
            c.ellipse(x, y, 3.0, 2.0, Color::rgb(80, 160, 90));
        }
        c.px(5, 6, Color::rgb(140, 210, 140));
        c.auto_outline(OUTLINE);
    }
    {
        // Rain totem: a carved post with a cloud.
        let mut c = Cell::at(&mut img, ICON_RAIN_TOTEM, OBJ_COLS, 16, 16);
        c.rect(6, 5, 4, 10, Color::rgb(150, 100, 60));
        c.rect(6, 8, 4, 1, Color::rgb(100, 65, 35));
        c.rect(6, 11, 4, 1, Color::rgb(100, 65, 35));
        c.ellipse(8.0, 4.0, 5.5, 2.5, Color::rgb(170, 190, 220));
        c.ellipse(6.0, 3.5, 2.5, 2.0, Color::rgb(200, 215, 235));
        c.px(4, 7, Color::rgb(80, 140, 230));
        c.px(12, 6, Color::rgb(80, 140, 230));
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
    let mut img = Image::new(128, 64);
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
    {
        // Cask: a hooped barrel lying on a stand.
        let mut c = Cell::at(&mut img, 5, 8, 16, 32);
        c.ellipse(8.0, 22.0, 7.0, 7.5, Color::rgb(150, 95, 50));
        c.rect(1, 18, 14, 1, Color::rgb(90, 90, 100));
        c.rect(1, 25, 14, 1, Color::rgb(90, 90, 100));
        c.ellipse(8.0, 22.0, 2.0, 2.0, Color::rgb(110, 70, 40));
        c.rect(2, 29, 12, 2, Color::rgb(100, 65, 35));
        c.auto_outline(OUTLINE);
    }
    {
        // Slime incubator: a glass dome on a warm base.
        let mut c = Cell::at(&mut img, 8, 8, 16, 32);
        c.rect(2, 22, 12, 8, Color::rgb(150, 120, 90));
        c.rect(3, 24, 10, 1, Color::rgb(230, 120, 60));
        c.ellipse(8.0, 17.0, 6.0, 7.0, Color::rgba(170, 220, 230, 200));
        c.ellipse(6.0, 14.0, 1.5, 2.5, Color::rgb(235, 250, 255));
        c.auto_outline(OUTLINE);
    }
    {
        // Slime ball: a wobbly green blob.
        let mut c = Cell::at(&mut img, 9, 8, 16, 32);
        c.ellipse(8.0, 23.0, 7.0, 7.0, Color::rgb(110, 190, 80));
        c.ellipse(6.0, 20.0, 2.5, 2.0, Color::rgb(180, 235, 150));
        c.ellipse(10.0, 26.0, 2.0, 1.5, Color::rgb(80, 150, 60));
        c.auto_outline(OUTLINE);
    }
    {
        // Mushroom box: a crate of logs sprouting caps.
        let mut c = Cell::at(&mut img, 10, 8, 16, 32);
        c.rect(1, 20, 14, 10, Color::rgb(130, 90, 55));
        c.rect(1, 20, 14, 2, Color::rgb(160, 115, 70));
        c.ellipse(5.0, 18.0, 3.0, 2.0, Color::rgb(170, 110, 70));
        c.ellipse(11.0, 17.0, 3.0, 2.0, Color::rgb(190, 130, 80));
        c.auto_outline(OUTLINE);
    }
    {
        // Bee house: a stacked box on legs with a dark entrance.
        let mut c = Cell::at(&mut img, 12, 8, 16, 32);
        c.rect(3, 26, 2, 5, Color::rgb(100, 70, 40));
        c.rect(11, 26, 2, 5, Color::rgb(100, 70, 40));
        c.rect(2, 12, 12, 14, Color::rgb(220, 200, 150));
        c.rect(2, 18, 12, 1, Color::rgb(170, 150, 110));
        c.rect(1, 9, 14, 3, Color::rgb(140, 100, 60));
        c.rect(6, 22, 4, 2, Color::rgb(50, 40, 30));
        c.px(4, 15, Color::rgb(240, 200, 60));
        c.px(11, 14, Color::rgb(240, 200, 60));
        c.auto_outline(OUTLINE);
    }
    {
        // Seed maker: a hopper on a box with a little chute.
        let mut c = Cell::at(&mut img, 11, 8, 16, 32);
        c.rect(2, 16, 12, 14, Color::rgb(110, 140, 90));
        c.rect(4, 10, 8, 6, Color::rgb(150, 120, 80));
        c.rect(5, 22, 6, 3, Color::rgb(60, 50, 40));
        c.px(7, 23, Color::rgb(220, 200, 120));
        c.auto_outline(OUTLINE);
    }
    {
        // Scarecrow: a straw figure on a post with a hat.
        let mut c = Cell::at(&mut img, 7, 8, 16, 32);
        c.rect(7, 12, 2, 19, Color::rgb(130, 90, 50));
        c.rect(2, 15, 12, 2, Color::rgb(130, 90, 50));
        c.rect(5, 14, 6, 9, Color::rgb(170, 70, 60));
        c.ellipse(8.0, 10.0, 3.5, 3.5, Color::rgb(230, 200, 120));
        c.rect(3, 6, 10, 2, Color::rgb(110, 80, 50));
        c.rect(5, 3, 6, 4, Color::rgb(110, 80, 50));
        c.px(7, 10, Color::rgb(40, 30, 30));
        c.px(9, 10, Color::rgb(40, 30, 30));
        c.rect(2, 17, 2, 2, Color::rgb(230, 200, 120));
        c.rect(12, 17, 2, 2, Color::rgb(230, 200, 120));
        c.auto_outline(OUTLINE);
    }
    {
        // Sewing machine: a small table with an arm and a spool.
        let mut c = Cell::at(&mut img, 6, 8, 16, 32);
        c.rect(1, 22, 14, 3, Color::rgb(140, 100, 60));
        c.rect(2, 25, 2, 6, Color::rgb(100, 70, 40));
        c.rect(12, 25, 2, 6, Color::rgb(100, 70, 40));
        c.rect(3, 13, 10, 3, Color::rgb(60, 120, 110));
        c.rect(10, 13, 3, 9, Color::rgb(60, 120, 110));
        c.rect(4, 16, 1, 5, Color::rgb(200, 200, 210));
        c.rect(6, 10, 3, 3, Color::rgb(220, 80, 80));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Crop growth strips: each crop uses 8 frames of 16x32, two crops per 256px row.
pub fn crops() -> Image {
    let mut img = Image::new(256, 32 * 2);
    let crops: [(Color, Color); 4] = [
        (Color::rgb(80, 170, 70), Color::rgb(240, 230, 230)),
        (Color::rgb(60, 150, 60), Color::rgb(80, 170, 70)),
        (Color::rgb(70, 140, 50), Color::rgb(235, 130, 40)),
        (Color::rgb(70, 150, 60), Color::rgb(245, 205, 60)),
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
    {
        // Lava: glowing orange with darker crust streaks.
        let mut c = Cell::at(&mut img, 5, 8, 16, 16);
        c.noise(Color::rgb(230, 100, 30), 10, 74);
        for (x, y, w) in [(1, 3, 6), (8, 7, 7), (3, 12, 5)] {
            c.rect(x, y, w, 1, Color::rgb(140, 40, 20));
        }
        c.px(5, 9, Color::rgb(255, 220, 120));
        c.px(12, 4, Color::rgb(255, 220, 120));
    }
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

/// Horse: 4x4 frames of 32x32 (rows down, right, up, left). Original
/// design: a bay horse with a dark mane and a pale blaze.
pub fn horse() -> Image {
    let mut img = Image::new(128, 128);
    let coat = Color::rgb(150, 95, 55);
    let mane = Color::rgb(60, 40, 30);
    for row in 0..4 {
        for f in 0..4 {
            let mut c = Cell { ox: f * 32, oy: row * 32, w: 32, h: 32, img: &mut img };
            let step = if f % 2 == 1 { 1 } else { 0 };
            if row == 1 || row == 3 {
                let right = row == 1;
                let hx = if right { 26 } else { 6 };
                c.ellipse(16.0, 17.0, 11.0, 6.0, coat);
                c.rect(if right { 21 } else { 7 }, 8, 4, 9, coat);
                c.ellipse(hx as f32, 9.0, 4.0, 3.0, coat);
                c.rect(if right { 20 } else { 9 }, 6, 3, 9, mane);
                c.rect(if right { 1 } else { 28 }, 14, 3, 9 - step, mane);
                c.px(if right { hx + 1 } else { hx - 1 }, 8, OUTLINE);
                for (lx, off) in [(8, step), (11, 0), (20, step), (23, 0)] {
                    c.rect(lx, 22, 2, 8 - off, shade(coat, -15));
                }
            } else {
                let front = row == 0;
                c.ellipse(16.0, 18.0, 7.0, 9.0, coat);
                c.rect(13, 3, 6, 10, coat);
                c.rect(15, 2, 2, 12, mane);
                if front {
                    c.rect(15, 6, 2, 6, Color::rgb(235, 225, 210));
                    c.px(13, 6, OUTLINE);
                    c.px(18, 6, OUTLINE);
                } else {
                    c.rect(15, 24, 2, 6 - step, mane);
                }
                c.rect(11, 25, 2, 6 - step, shade(coat, -15));
                c.rect(19, 25, 2, 6, shade(coat, -15));
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

/// Harvest hut (32x32, 2x2 tiles). Original design: a domed fieldstone
/// hut with a mossy cap and a little wooden door.
pub fn hut() -> Image {
    let mut img = Image::new(32, 32);
    let mut c = Cell { ox: 0, oy: 0, w: 32, h: 32, img: &mut img };
    c.ellipse(16.0, 19.0, 14.0, 12.0, Color::rgb(150, 145, 135));
    for (x, y) in [(7, 14), (13, 10), (20, 12), (24, 18), (9, 22), (18, 20)] {
        c.ellipse(x as f32, y as f32, 2.5, 1.8, Color::rgb(175, 170, 160));
    }
    c.ellipse(16.0, 9.0, 11.0, 5.0, Color::rgb(90, 140, 70));
    c.rect(13, 22, 6, 9, Color::rgb(120, 80, 45));
    c.px(17, 26, Color::rgb(230, 200, 80));
    c.auto_outline(OUTLINE);
    img
}

/// Obelisk building (32x48, 2x3 tiles). Original design: a carved stone
/// pillar with a glowing band.
pub fn obelisk() -> Image {
    let mut img = Image::new(32, 48);
    let mut c = Cell { ox: 0, oy: 0, w: 32, h: 48, img: &mut img };
    let stone = Color::rgb(150, 140, 160);
    c.rect(4, 40, 24, 6, shade(stone, -20));
    for y in 4..40 {
        let inset = (y - 4) / 8;
        c.rect(9 - inset, y, 14 + inset * 2, 1, if y % 6 == 0 { shade(stone, -15) } else { stone });
    }
    c.rect(14, 0, 4, 4, stone);
    c.rect(10, 18, 12, 3, Color::rgb(230, 170, 80));
    c.auto_outline(OUTLINE);
    img
}

/// Fish pond building (48x48, 3x3 tiles). Original design: a stone-rimmed
/// pool with lily pads.
pub fn pond() -> Image {
    let mut img = Image::new(48, 48);
    let mut c = Cell { ox: 0, oy: 0, w: 48, h: 48, img: &mut img };
    c.ellipse(24.0, 25.0, 23.0, 21.0, Color::rgb(130, 130, 125));
    for i in 0..14 {
        let a = i as f32 / 14.0 * std::f32::consts::TAU;
        let (x, y) = (24.0 + a.cos() * 21.0, 25.0 + a.sin() * 19.0);
        c.ellipse(x, y, 3.0, 2.5, Color::rgb(160, 160, 150));
    }
    c.ellipse(24.0, 25.0, 17.0, 15.0, Color::rgb(56, 120, 200));
    c.ellipse(22.0, 22.0, 12.0, 9.0, Color::rgb(80, 145, 215));
    for (x, y) in [(15.0, 30.0), (31.0, 19.0)] {
        c.ellipse(x, y, 3.0, 2.0, Color::rgb(70, 150, 70));
    }
    c.auto_outline(OUTLINE);
    img
}

/// Draw placeholder hat `kind` (0 straw, 1 cap, 2 beanie) facing `dir`
/// (0 down, 1 right, 2 up, 3 left) into a 16x16 cell. Original designs.
fn paint_hat(c: &mut Cell, kind: i32, dir: i32) {
    let side = dir == 1 || dir == 3;
    let flip = |x: i32| if dir == 3 { 15 - x } else { x };
    match kind {
        0 => {
            let straw = Color::rgb(230, 200, 120);
            c.rect(1, 9, 14, 2, straw);
            c.rect(4, 4, 8, 6, straw);
            c.rect(4, 8, 8, 1, Color::rgb(170, 70, 50));
        }
        1 => {
            let cap = Color::rgb(60, 110, 180);
            c.rect(4, 4, 8, 5, cap);
            c.rect(5, 3, 6, 1, cap);
            if side {
                for x in 10..14 {
                    c.px(flip(x), 8, shade(cap, -30));
                }
            } else if dir == 0 {
                c.rect(4, 9, 8, 1, shade(cap, -30));
            }
        }
        _ => {
            let knit = Color::rgb(200, 80, 90);
            c.rect(4, 4, 8, 6, knit);
            c.rect(5, 3, 6, 1, knit);
            c.rect(4, 8, 8, 2, shade(knit, 30));
            c.rect(7, 1, 2, 2, Color::rgb(240, 240, 230));
        }
    }
    c.auto_outline(OUTLINE);
}

/// Worn hats sheet: 3 hats (columns) x 4 facings (rows), 16x16 cells.
pub fn hats() -> Image {
    let mut img = Image::new(48, 64);
    for kind in 0..3 {
        for dir in 0..4 {
            let mut c = Cell { ox: kind * 16, oy: dir * 16, w: 16, h: 16, img: &mut img };
            paint_hat(&mut c, kind, dir);
        }
    }
    img
}

/// Clothing icons, 16x16: 0-2 hats, 3 tee, 4 striped shirt, 5 work pants,
/// 6 shorts. Original designs.
pub fn clothes() -> Image {
    let mut img = Image::new(128, 16);
    for kind in 0..3 {
        let mut c = Cell { ox: kind * 16, oy: 0, w: 16, h: 16, img: &mut img };
        paint_hat(&mut c, kind, 0);
    }
    for (i, (body, stripe)) in [(Color::rgb(240, 240, 235), None), (Color::rgb(70, 120, 190), Some(Color::rgb(230, 230, 240)))].iter().enumerate() {
        let mut c = Cell { ox: (3 + i as i32) * 16, oy: 0, w: 16, h: 16, img: &mut img };
        c.rect(4, 3, 8, 11, *body);
        c.rect(1, 3, 3, 4, *body);
        c.rect(12, 3, 3, 4, *body);
        c.rect(6, 3, 4, 1, shade(*body, -40));
        if let Some(s) = stripe {
            for y in (5..14).step_by(3) {
                c.rect(4, y, 8, 1, *s);
            }
        }
        c.auto_outline(OUTLINE);
    }
    for (i, (col, long)) in [(Color::rgb(90, 80, 70), true), (Color::rgb(170, 140, 90), false)].iter().enumerate() {
        let mut c = Cell { ox: (5 + i as i32) * 16, oy: 0, w: 16, h: 16, img: &mut img };
        let h = if *long { 12 } else { 7 };
        c.rect(4, 2, 8, 3, *col);
        c.rect(4, 4, 3, h, *col);
        c.rect(9, 4, 3, h, *col);
        c.rect(4, 2, 8, 1, shade(*col, -30));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Trinket icons, 16x16: 0 pebble charm, 1 swift feather. Original designs.
pub fn trinkets() -> Image {
    let mut img = Image::new(32, 16);
    {
        let mut c = Cell { ox: 0, oy: 0, w: 16, h: 16, img: &mut img };
        c.rect(7, 1, 2, 4, Color::rgb(150, 110, 70));
        c.ellipse(8.0, 9.0, 4.5, 4.0, Color::rgb(120, 170, 160));
        c.ellipse(7.0, 8.0, 1.5, 1.2, Color::rgb(200, 240, 230));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell { ox: 16, oy: 0, w: 16, h: 16, img: &mut img };
        for i in 0..10 {
            c.rect(4 + i / 2, 13 - i, 3, 1, Color::rgb(240, 240, 250));
        }
        c.rect(5, 12, 1, 3, Color::rgb(140, 120, 100));
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

/// 32x32 cell at an object sheet index (2x2 cells).
fn clump_cell(img: &mut Image, index: i32) -> Cell<'_> {
    Cell { ox: (index % OBJ_COLS) * 16, oy: (index / OBJ_COLS) * 16, w: 32, h: 32, img }
}

fn clumps(img: &mut Image) {
    let bark = Color::rgb(120, 80, 45);
    {
        // Large stump: wide trunk with roots and rings on top.
        let mut c = clump_cell(img, 600);
        c.rect(5, 12, 22, 16, bark);
        c.rect(2, 24, 6, 5, shade(bark, -15));
        c.rect(24, 24, 6, 5, shade(bark, -15));
        c.ellipse(16.0, 12.0, 11.0, 5.0, Color::rgb(200, 160, 100));
        c.ellipse(16.0, 12.0, 7.0, 3.0, Color::rgb(170, 130, 80));
        c.ellipse(16.0, 12.0, 3.0, 1.5, Color::rgb(200, 160, 100));
        for x in [9, 15, 21] {
            c.rect(x, 18, 1, 9, shade(bark, -30));
        }
        c.auto_outline(OUTLINE);
    }
    {
        // Hollow log lying on its side.
        let mut c = clump_cell(img, 602);
        c.rect(3, 12, 26, 15, bark);
        c.rect(3, 12, 26, 3, shade(bark, 25));
        c.ellipse(4.5, 19.5, 4.0, 7.5, Color::rgb(190, 150, 95));
        c.ellipse(4.5, 19.5, 2.2, 4.5, Color::rgb(50, 35, 25));
        for x in [10, 16, 22] {
            c.rect(x, 17, 4, 1, shade(bark, -30));
        }
        c.rect(18, 9, 2, 3, Color::rgb(80, 150, 60));
        c.auto_outline(OUTLINE);
    }
    {
        // Meteorite: dark, pitted, faintly glowing.
        let mut c = clump_cell(img, 622);
        c.ellipse(16.0, 18.0, 14.0, 12.0, Color::rgb(70, 60, 75));
        c.ellipse(13.0, 14.0, 7.0, 5.0, Color::rgb(95, 85, 100));
        for (x, y) in [(9, 20), (19, 13), (21, 23), (13, 25)] {
            c.ellipse(x as f32, y as f32, 2.0, 1.5, Color::rgb(45, 35, 50));
        }
        c.px(17, 19, Color::rgb(240, 120, 60));
        c.px(10, 15, Color::rgb(240, 120, 60));
        c.auto_outline(OUTLINE);
    }
    {
        // Boulder.
        let mut c = clump_cell(img, 672);
        c.ellipse(16.0, 19.0, 14.0, 11.0, Color::rgb(130, 128, 120));
        c.ellipse(12.0, 14.0, 8.0, 5.0, Color::rgb(160, 158, 150));
        c.rect(18, 20, 6, 1, Color::rgb(100, 98, 92));
        c.rect(9, 24, 5, 1, Color::rgb(100, 98, 92));
        c.auto_outline(OUTLINE);
    }
    for v in 0..4 {
        let mut c = clump_cell(img, 752 + 2 * v);
        let base = shade(Color::rgb(110, 100, 95), v * 8 - 12);
        c.ellipse(16.0, 19.0, 13.0 + v as f32 * 0.5, 11.0, base);
        c.ellipse(12.0 + v as f32, 14.0, 6.0, 4.0, shade(base, 25));
        c.rect(15, 22, 7, 1, shade(base, -30));
        c.auto_outline(OUTLINE);
    }
}

/// Crow: two pecking frames and two flying frames (16x16 each).
pub fn crow() -> Image {
    let mut img = Image::new(64, 16);
    let body = Color::rgb(40, 40, 55);
    for f in 0..4 {
        let mut c = Cell::at(&mut img, f, 4, 16, 16);
        if f < 2 {
            let dy = f; // head bob
            c.ellipse(7.0, 11.0, 5.0, 3.5, body);
            c.ellipse(11.0, 8.0 + dy as f32 * 2.0, 2.5, 2.5, body);
            c.rect(13, 8 + dy * 2, 2, 1, Color::rgb(220, 180, 60));
            c.px(11, 7 + dy * 2, Color::WHITE);
            c.rect(2, 10, 3, 1, shade(body, -10));
            c.rect(6, 14, 1, 2, Color::rgb(220, 180, 60));
            c.rect(9, 14, 1, 2, Color::rgb(220, 180, 60));
        } else {
            let up = f == 2;
            c.ellipse(8.0, 9.0, 4.5, 2.5, body);
            c.ellipse(12.0, 8.0, 2.0, 2.0, body);
            c.rect(14, 8, 2, 1, Color::rgb(220, 180, 60));
            if up {
                c.rect(4, 3, 3, 5, body);
                c.rect(8, 2, 3, 6, body);
            } else {
                c.rect(4, 10, 3, 5, body);
                c.rect(8, 10, 3, 5, body);
            }
        }
        c.auto_outline(OUTLINE);
    }
    img
}

/// A giant pumpkin (48x48).
pub fn giant_crops() -> Image {
    let mut img = Image::new(48, 48);
    let mut c = Cell::at(&mut img, 0, 1, 48, 48);
    let orange = Color::rgb(235, 130, 40);
    c.ellipse(24.0, 28.0, 22.0, 17.0, orange);
    for x in [12.0, 24.0, 36.0] {
        c.ellipse(x, 28.0, 3.0, 15.0, shade(orange, -30));
    }
    c.ellipse(18.0, 20.0, 6.0, 4.0, shade(orange, 30));
    c.rect(22, 6, 4, 7, Color::rgb(90, 120, 50));
    c.ellipse(30.0, 9.0, 5.0, 2.5, Color::rgb(80, 160, 60));
    c.auto_outline(OUTLINE);
    img
}

/// Emote bubbles: 16 rows (emote id / 4), four 16x16 frames each that grow
/// the bubble in; the last frame carries the symbol.
pub fn emotes() -> Image {
    let mut img = Image::new(64, 16 * 16);
    // 7x7 glyphs, '#' = ink.
    let glyph = |row: i32| -> (&'static [&'static str], Color) {
        match row {
            2 => (&["..###..", ".#...#.", ".....#.", "...##..", "...#...", ".......", "...#..."], Color::rgb(60, 90, 200)),
            3 => (&["#.....#", ".#...#.", "..#.#..", ".......", ".#####.", "#.....#", "......."], Color::rgb(200, 50, 40)),
            4 => (&["...#...", "...#...", "...#...", "...#...", "...#...", ".......", "...#..."], Color::rgb(220, 120, 30)),
            5 => (&[".##.##.", "#######", "#######", ".#####.", "..###..", "...#...", "......."], Color::rgb(230, 70, 110)),
            6 => (&["#####..", "...#...", "..#....", "#####..", "....###", ".....#.", "....###"], Color::rgb(90, 90, 160)),
            7 => (&[".......", ".#...#.", ".......", "..###..", ".#...#.", ".......", "...#..."], Color::rgb(80, 120, 200)),
            8 => (&[".......", ".#...#.", ".......", ".#...#.", "..###..", ".......", "......."], Color::rgb(60, 160, 70)),
            9 => (&["#.....#", ".#...#.", "..#.#..", "...#...", "..#.#..", ".#...#.", "#.....#"], Color::rgb(200, 50, 40)),
            10 => (&[".......", ".......", "#.#.#..", ".......", ".......", ".......", "......."], Color::rgb(80, 80, 80)),
            14 => (&["...###.", "...#.#.", "...#.#.", "...#...", ".###...", "####...", ".##...."], Color::rgb(120, 60, 160)),
            15 => (&[".......", "##...##", "#######", ".#####.", ".......", ".......", "......."], Color::rgb(240, 120, 150)),
            _ => (&[".......", ".......", "..###..", "..###..", "..###..", ".......", "......."], Color::rgb(120, 120, 120)),
        }
    };
    for row in 0..16 {
        for f in 0..4 {
            let mut c = Cell { ox: f * 16, oy: row * 16, w: 16, h: 16, img: &mut img };
            let r = 2.0 + f as f32 * 1.6;
            c.ellipse(8.0, 7.0, r + 0.6, r * 0.85 + 0.6, OUTLINE);
            c.ellipse(8.0, 7.0, r, r * 0.85, Color::rgb(255, 255, 250));
            if f == 3 {
                c.rect(7, 13, 2, 2, Color::rgb(255, 255, 250));
                c.px(6, 14, OUTLINE);
                c.px(9, 14, OUTLINE);
                let (rows, ink) = glyph(row);
                for (y, line) in rows.iter().enumerate() {
                    for (x, ch) in line.chars().enumerate() {
                        if ch == '#' {
                            c.px(5 + x as i32 - 1, 3 + y as i32, ink);
                        }
                    }
                }
            }
        }
    }
    img
}

/// Simple 4-frame monster sheets (16x24 frames): 0 crab, 1 mole, 2 larva,
/// 3 skeleton, 4 mummy.
pub fn monster_simple(body: Color, style: u8) -> Image {
    let mut img = Image::new(64, 24);
    let dark = shade(body, -50);
    for f in 0..4 {
        let mut c = Cell { ox: f * 16, oy: 0, w: 16, h: 24, img: &mut img };
        let step = (f % 2) as i32;
        match style {
            0 => {
                c.ellipse(8.0, 17.0, 7.0, 5.0, body);
                c.ellipse(6.0, 15.0, 2.5, 1.5, shade(body, 30));
                for x in [2, 5, 10, 13] {
                    c.rect(x, 21 + step, 1, 2, dark);
                }
                c.px(6, 18, Color::BLACK);
                c.px(10, 18, Color::BLACK);
            }
            1 => {
                c.ellipse(8.0, 17.0, 6.0, 5.0, body);
                c.ellipse(8.0, 14.0, 4.0, 3.0, shade(body, 20));
                c.rect(7, 16, 2, 1, Color::rgb(230, 150, 160));
                c.rect(3 + step, 21, 3, 2, Color::rgb(230, 150, 160));
                c.rect(10 - step, 21, 3, 2, Color::rgb(230, 150, 160));
            }
            2 => {
                for i in 0..4 {
                    c.ellipse(4.0 + i as f32 * 2.8, 19.0 - (i % 2) as f32 * step as f32, 2.6, 2.6, if i == 3 { shade(body, -20) } else { body });
                }
                c.px(13, 18, Color::BLACK);
            }
            3 => {
                c.ellipse(8.0, 6.0, 4.0, 4.0, body);
                c.px(6, 6, Color::BLACK);
                c.px(10, 6, Color::BLACK);
                c.rect(7, 10, 2, 7, body);
                for y in [11, 13, 15] {
                    c.rect(5, y, 6, 1, body);
                }
                c.rect(6 - step, 17, 1, 6, body);
                c.rect(9 + step, 17, 1, 6, body);
            }
            _ => {
                c.rect(5, 4, 6, 18, body);
                for y in (5..22).step_by(3) {
                    c.rect(5, y, 6, 1, dark);
                }
                c.px(7, 7, Color::rgb(200, 40, 40));
                c.px(9, 7, Color::rgb(200, 40, 40));
                c.rect(3 - step, 10, 2, 2, body);
                c.rect(11 + step, 10, 2, 2, body);
            }
        }
        c.auto_outline(OUTLINE);
    }
    img
}

/// A hand-painted-style overview map of the placeholder valley (300x180).
pub fn world_map() -> Image {
    let mut img = Image::new(300, 180);
    let mut c = Cell { ox: 0, oy: 0, w: 300, h: 180, img: &mut img };
    c.noise(Color::rgb(110, 160, 80), 6, 41);
    // Sea along the bottom-left and a river.
    c.rect(0, 150, 120, 30, Color::rgb(70, 130, 200));
    for y in 0..150 {
        let x = 118 + ((y as f32 / 18.0).sin() * 4.0) as i32;
        c.rect(x, y, 5, 1, Color::rgb(80, 145, 210));
    }
    // Mountains (north).
    for i in 0..9 {
        let x = 120 + i * 18;
        for k in 0..14 {
            c.rect(x + k / 2, 22 - k, 18 - k, 1, shade(Color::rgb(120, 110, 100), -k * 2));
        }
    }
    // Farm fields.
    c.rect(20, 40, 90, 70, Color::rgb(150, 110, 70));
    for y in (44..106).step_by(6) {
        c.rect(24, y, 82, 2, Color::rgb(120, 85, 55));
    }
    c.rect(56, 44, 14, 10, Color::rgb(170, 70, 50));
    // Town.
    c.rect(150, 40, 80, 60, Color::rgb(185, 175, 150));
    for (x, y) in [(156, 46), (176, 46), (196, 46), (156, 70), (186, 70), (210, 76)] {
        c.rect(x, y, 14, 12, Color::rgb(170, 80, 60));
        c.rect(x + 2, y + 6, 4, 6, Color::rgb(90, 60, 40));
    }
    // Mine mouth.
    c.ellipse(150.0, 22.0, 6.0, 5.0, Color::rgb(40, 30, 30));
    // Dunes.
    c.rect(230, 110, 60, 50, Color::rgb(220, 190, 130));
    for i in 0..4 {
        c.ellipse(245.0 + i as f32 * 12.0, 130.0 + (i % 2) as f32 * 10.0, 6.0, 3.0, Color::rgb(200, 165, 105));
    }
    // Isle in the sea.
    c.ellipse(50.0, 165.0, 20.0, 9.0, Color::rgb(230, 210, 150));
    c.ellipse(50.0, 163.0, 12.0, 5.0, Color::rgb(90, 160, 80));
    // Roads.
    c.rect(110, 70, 40, 3, Color::rgb(200, 180, 140));
    c.rect(228, 98, 3, 14, Color::rgb(200, 180, 140));
    c.auto_outline(OUTLINE);
    img
}

/// Copy an image, mapping each opaque pixel's colour through `f`.
pub fn recolor(src: &Image, f: impl Fn(Color) -> Color) -> Image {
    let mut out = Image::new(src.width, src.height);
    for y in 0..src.height as i32 {
        for x in 0..src.width as i32 {
            let c = src.get(x as u32, y as u32);
            if c.a > 0 {
                out.set(x, y, f(c));
            }
        }
    }
    out
}

/// A grey-and-white second dog breed.
pub fn dog2() -> Image {
    recolor(&dog(), |c| {
        let l = ((c.r as u32 + c.g as u32 + c.b as u32) / 3) as u8;
        Color::rgba(l.saturating_add(20), l.saturating_add(20), l.saturating_add(28), c.a)
    })
}

/// A ginger cat (the dog's sheet, recoloured and with a slimmer build).
pub fn cat() -> Image {
    recolor(&dog(), |c| Color::rgba(c.r.saturating_add(40), (c.g as u32 * 9 / 10) as u8, (c.b as u32 * 6 / 10) as u8, c.a))
}

/// A fence sheet: 16x32 frames for each neighbour mask (N=1, E=2, S=4,
/// W=8) in four columns, then the closed and open gate.
pub fn fence(post: Color) -> Image {
    let mut img = Image::new(64, 160);
    let rail = shade(post, 20);
    for mask in 0..16 {
        let mut c = Cell::at(&mut img, mask, 4, 16, 32);
        let top = if mask & 1 != 0 { 4 } else { 14 };
        let bottom = if mask & 4 != 0 { 32 } else { 30 };
        c.rect(6, top, 4, bottom - top, post);
        c.rect(6, top, 4, 1, shade(post, 30));
        if mask & 2 != 0 {
            c.rect(10, 18, 6, 2, rail);
            c.rect(10, 24, 6, 2, rail);
        }
        if mask & 8 != 0 {
            c.rect(0, 18, 6, 2, rail);
            c.rect(0, 24, 6, 2, rail);
        }
        c.auto_outline(OUTLINE);
    }
    let wood = Color::rgb(170, 120, 70);
    {
        let mut c = Cell::at(&mut img, 16, 4, 16, 32);
        c.rect(0, 16, 16, 12, wood);
        c.rect(0, 21, 16, 1, shade(wood, -40));
        c.rect(7, 16, 1, 12, shade(wood, -40));
        c.px(13, 22, Color::rgb(60, 60, 70));
        c.auto_outline(OUTLINE);
    }
    {
        let mut c = Cell::at(&mut img, 17, 4, 16, 32);
        c.rect(0, 16, 3, 12, wood);
        c.rect(13, 16, 3, 12, shade(wood, -20));
        c.auto_outline(OUTLINE);
    }
    img
}

/// Floors and paths: 64x64 blocks of 4x4 tiles (one per neighbour mask):
/// wood floor at (0,0), stone path at (64,0), gravel at (0,64).
pub fn flooring() -> Image {
    let mut img = Image::new(128, 128);
    let kinds = [(0, 0, Color::rgb(170, 120, 70)), (64, 0, Color::rgb(160, 160, 165)), (0, 64, Color::rgb(150, 135, 110))];
    for (k, (bx, by, base)) in kinds.iter().enumerate() {
        for mask in 0..16 {
            let mut c = Cell { ox: bx + (mask % 4) * 16, oy: by + (mask / 4) * 16, w: 16, h: 16, img: &mut img };
            c.noise(*base, 8, (k as u32) * 31 + mask as u32);
            match k {
                0 => {
                    for y in [3, 7, 11, 15] {
                        c.rect(0, y, 16, 1, shade(*base, -30));
                    }
                }
                1 => {
                    c.rect(0, 8, 16, 1, shade(*base, -30));
                    c.rect(8, 0, 1, 8, shade(*base, -30));
                    c.rect(4, 8, 1, 8, shade(*base, -30));
                }
                _ => {
                    for i in 0..10 {
                        c.px((i * 7 + mask) % 16, (i * 5) % 16, shade(*base, if i % 2 == 0 { -30 } else { 25 }));
                    }
                }
            }
            // Edges where the floor ends.
            let edge = shade(*base, -50);
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

/// Bushes: small (0,0,16,32), medium (16,0,32,32), large (48,0,48,32), young
/// and grown tea (96 / 112); the row below shows berries or fresh leaves.
pub fn bushes() -> Image {
    let mut img = Image::new(128, 64);
    let leaf = Color::rgb(60, 130, 60);
    for ready in [false, true] {
        let oy = if ready { 32 } else { 0 };
        {
            let mut c = bush_blob(&mut img, 0, oy, 16, 14, 1, leaf);
            c.auto_outline(OUTLINE);
        }
        {
            let mut c = bush_blob(&mut img, 16, oy, 32, 20, 2, leaf);
            if ready {
                for (x, y) in [(7, 18), (14, 22), (22, 17), (26, 24), (11, 27), (19, 26)] {
                    c.rect(x, y, 2, 2, Color::rgb(200, 50, 90));
                }
            }
            c.auto_outline(OUTLINE);
        }
        {
            let mut c = bush_blob(&mut img, 48, oy, 48, 26, 3, leaf);
            c.auto_outline(OUTLINE);
        }
        {
            let mut c = Cell { ox: 96, oy, w: 16, h: 32, img: &mut img };
            c.rect(7, 26, 2, 6, Color::rgb(110, 80, 50));
            c.ellipse(8.0, 23.0, 4.0, 4.0, Color::rgb(80, 150, 80));
            c.auto_outline(OUTLINE);
        }
        {
            let mut c = Cell { ox: 112, oy, w: 16, h: 32, img: &mut img };
            c.ellipse(8.0, 22.0, 7.0, 9.0, Color::rgb(70, 140, 70));
            if ready {
                for (x, y) in [(4, 16), (10, 19), (6, 24), (11, 26)] {
                    c.rect(x, y, 2, 2, Color::rgb(150, 220, 140));
                }
            }
            c.auto_outline(OUTLINE);
        }
    }
    img
}

fn bush_blob(img: &mut Image, ox: i32, oy: i32, w: i32, h: i32, salt: u32, leaf: Color) -> Cell<'_> {
    let mut c = Cell { ox, oy, w, h: 32, img };
    let rx = w as f32 / 2.0 - 1.0;
    c.ellipse(w as f32 / 2.0, 32.0 - h as f32 / 2.0, rx, h as f32 / 2.0 - 1.0, leaf);
    for i in 0..(w / 3) {
        let px = 2 + (super::paint::hash(i, salt as i32, 5) % (w as u32 - 4)) as i32;
        let py = 32 - h + 3 + (super::paint::hash(i, salt as i32, 6) % (h as u32 - 6)) as i32;
        c.px(px, py, shade(leaf, 30));
    }
    c
}

/// Marker tiles for the `Paths` map layer (not drawn in game).
pub fn paths_sheet() -> Image {
    let mut img = Image::new(64, 16);
    for i in 0..4 {
        let mut c = Cell::at(&mut img, i, 4, 16, 16);
        c.rect(2, 2, 12, 12, [Color::rgb(80, 160, 80), Color::rgb(60, 130, 60), Color::rgb(40, 100, 40), Color::rgb(140, 100, 60)][i as usize]);
    }
    img
}
