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
