//! Original built-in placeholder content pack.
//!
//! Generates a small but complete world (farm, farmhouse, town, shop, two
//! villagers, a handful of items and crops) as in-memory PNG / JSON / TMX files
//! so the engine runs and is testable without any game files. None of this is
//! derived from the original game's assets; only the file formats and asset
//! names match, so the same loaders serve real content.

pub mod art;
pub mod maps;
pub mod paint;

use engine::Color;
use serde_json::json;

use crate::source::MemSource;

pub fn build() -> MemSource {
    let mut m = MemSource::new("builtin-placeholder");

    for season in ["spring", "summer", "fall", "winter"] {
        m.insert(&format!("Maps/{season}_dev_outdoors.png"), art::outdoors(season).encode_png());
        m.insert(&format!("TerrainFeatures/dev_tree_{season}.png"), art::debris_and_trees(season).encode_png());
    }
    m.insert("Maps/dev_indoors.png", art::indoors().encode_png());
    m.insert("Maps/springobjects.png", art::objects().encode_png());
    m.insert("TileSheets/crops.png", art::crops().encode_png());
    m.insert("TerrainFeatures/hoeDirt.png", art::hoe_dirt().encode_png());
    m.insert("TileSheets/tools.png", art::tools().encode_png());
    m.insert("TileSheets/Craftables.png", art::craftables().encode_png());

    let farmer = art::Look {
        skin: Color::rgb(236, 190, 150),
        hair: Color::rgb(110, 70, 40),
        shirt: Color::rgb(70, 140, 90),
        legs: Color::rgb(80, 70, 60),
        long_hair: false,
        beard: false,
    };
    m.insert("Characters/Farmer/dev_farmer.png", art::character(&farmer).encode_png());

    let villagers = [
        ("Mara", art::Look { skin: Color::rgb(200, 150, 110), hair: Color::rgb(40, 30, 60), shirt: Color::rgb(150, 80, 170), legs: Color::rgb(90, 50, 110), long_hair: true, beard: false }),
        ("Tobin", art::Look { skin: Color::rgb(240, 200, 170), hair: Color::rgb(190, 110, 50), shirt: Color::rgb(220, 140, 60), legs: Color::rgb(60, 70, 110), long_hair: false, beard: true }),
    ];
    for (name, look) in &villagers {
        m.insert(&format!("Characters/{name}.png"), art::character(look).encode_png());
        m.insert(&format!("Portraits/{name}.png"), art::portrait(look).encode_png());
    }

    let maps = maps::all();
    for map in &maps {
        m.insert_str(&format!("Maps/{}.tmx", map.id), &crate::tmx::write(map));
    }

    for (path, value) in data_files() {
        m.insert_str(&format!("{path}.json"), &serde_json::to_string_pretty(&value).unwrap());
    }
    m
}

fn data_files() -> Vec<(&'static str, serde_json::Value)> {
    let obj = |name: &str, kind: &str, cat: i32, price: i32, sprite: i32, edibility: i32| {
        json!({
            "Name": name,
            "DisplayName": format!("[LocalizedText Strings\\Objects:{}_Name]", name.replace(' ', "")),
            "Description": format!("[LocalizedText Strings\\Objects:{}_Description]", name.replace(' ', "")),
            "Type": kind, "Category": cat, "Price": price, "SpriteIndex": sprite, "Edibility": edibility
        })
    };
    use art::*;
    let objects = json!({
        "Dev_TurnipSeeds": obj("Turnip Seeds", "Seeds", -74, 10, ICON_TURNIP_SEEDS, -300),
        "Dev_Turnip": obj("Turnip", "Basic", -75, 40, ICON_TURNIP, 12),
        "Dev_BeanSeeds": obj("Bean Seeds", "Seeds", -74, 30, ICON_BEAN_SEEDS, -300),
        "Dev_Bean": obj("Bean", "Basic", -75, 25, ICON_BEAN, 8),
        "Dev_PumpkinSeeds": obj("Pumpkin Seeds", "Seeds", -74, 50, ICON_PUMPKIN_SEEDS, -300),
        "Dev_Pumpkin": obj("Pumpkin", "Basic", -75, 300, ICON_PUMPKIN, 20),
        "388": obj("Wood", "Basic", -16, 2, ICON_WOOD, -300),
        "390": obj("Stone", "Basic", -16, 2, ICON_STONE, -300),
        "771": obj("Fiber", "Basic", -16, 1, ICON_FIBER, -300),
        "92": obj("Sap", "Basic", -81, 2, ICON_SAP, -300),
        "Dev_Bread": obj("Bread", "Cooking", -7, 60, ICON_BREAD, 20),
        "Dev_Berry": obj("Wild Berry", "Basic", -79, 20, ICON_BERRY, 6),
        "Dev_Weeds": obj("Weeds", "Litter", 0, 0, ICON_WEEDS, -300),
        "Dev_Twig": obj("Twig", "Litter", 0, 0, ICON_TWIG, -300),
        "Dev_Rock": obj("Stone", "Litter", 0, 0, ICON_ROCK, -300),
        "Dev_Juice": obj("Juice", "ArtisanGoods", -26, 50, ICON_JUICE, 15),
        "Dev_Perch": obj("Pond Perch", "Fish", -4, 45, ICON_PERCH, 8),
        "Dev_Darter": obj("River Darter", "Fish", -4, 90, ICON_DARTER, 10),
        "Dev_Driftwood": obj("Driftwood", "Basic", -20, 0, ICON_DRIFTWOOD, -300),
        "Dev_Sprinkler": {
            "Name": "Sprinkler", "DisplayName": "Sprinkler", "Description": "Waters the 4 adjacent tiles every morning.",
            "Type": "Crafting", "Category": -8, "Price": 50, "SpriteIndex": ICON_SPRINKLER, "ContextTags": ["sprinkler_cross"]
        },
        "Dev_Fertilizer": {
            "Name": "Fertilizer", "DisplayName": "Fertilizer", "Description": "Improves the chance of quality crops.",
            "Type": "Basic", "Category": -19, "Price": 2, "SpriteIndex": ICON_FERTILIZER, "CustomFields": { "FertilizerQuality": "1" }
        },
        "Dev_SpeedGro": {
            "Name": "Growth Mix", "DisplayName": "Growth Mix", "Description": "Crops grow 10% faster.",
            "Type": "Basic", "Category": -19, "Price": 20, "SpriteIndex": ICON_SPEEDGRO, "CustomFields": { "FertilizerSpeed": "0.1" }
        },
    });
    let bigs = json!({
        "Dev_Box": { "Name": "Storage Box", "DisplayName": "Storage Box", "Description": "Holds 36 items.",
                     "Price": 1, "SpriteIndex": 0, "ContextTags": ["storage_item"] },
        "Dev_Juicer": { "Name": "Juicer", "DisplayName": "Juicer", "Description": "Turns produce into juice.",
                        "Price": 50, "SpriteIndex": 1 },
    });
    let machines = json!({
        "(BC)Dev_Juicer": {
            "OutputRules": [{
                "Id": "produce",
                "Triggers": [{ "Trigger": "ItemPlacedInMachine", "RequiredTags": ["category_vegetable,category_fruits"] }],
                "OutputItem": [{
                    "ItemId": "(O)Dev_Juice", "CopyPrice": true,
                    "PriceModifiers": [{ "Modification": "Multiply", "Amount": 2.0 }],
                    "ObjectDisplayName": "%PRESERVED_DISPLAY_NAME Juice", "PreserveId": "DROP_IN"
                }],
                "MinutesUntilReady": 120
            }],
            "InvalidItemMessage": "The juicer only takes fruit and vegetables."
        }
    });
    let recipes = json!({
        "Storage Box": "388 20/Home/Dev_Box/true/default/",
        "Juicer": "388 15 390 10/Home/Dev_Juicer/true/default/",
        "Sprinkler": "390 5 388 5/Home/Dev_Sprinkler/false/default/",
        "Fertilizer": "771 2/Home/Dev_Fertilizer 5/false/l 1/",
    });
    let strings = json!({
        "TurnipSeeds_Name": "Turnip Seeds", "TurnipSeeds_Description": "Plant in spring. Ready in 4 days.",
        "Turnip_Name": "Turnip", "Turnip_Description": "A crisp root vegetable.",
        "BeanSeeds_Name": "Bean Seeds", "BeanSeeds_Description": "Plant in spring or summer. Keeps producing.",
        "Bean_Name": "Bean", "Bean_Description": "A handful of green beans.",
        "PumpkinSeeds_Name": "Pumpkin Seeds", "PumpkinSeeds_Description": "Plant in fall. Takes a while.",
        "Pumpkin_Name": "Pumpkin", "Pumpkin_Description": "Big, round and orange.",
        "Wood_Name": "Wood", "Wood_Description": "Useful building material.",
        "Stone_Name": "Stone", "Stone_Description": "A common building material.",
        "Fiber_Name": "Fiber", "Fiber_Description": "Plant fibers.",
        "Sap_Name": "Sap", "Sap_Description": "Sticky tree sap.",
        "Bread_Name": "Bread", "Bread_Description": "A warm loaf.",
        "PondPerch_Name": "Pond Perch", "PondPerch_Description": "A calm fish from still water.",
        "RiverDarter_Name": "River Darter", "RiverDarter_Description": "Quick and hard to catch.",
        "Driftwood_Name": "Driftwood", "Driftwood_Description": "Soggy and useless.",
        "WildBerry_Name": "Wild Berry", "WildBerry_Description": "Found growing wild.",
        "Weeds_Name": "Weeds", "Weeds_Description": "", "Twig_Name": "Twig", "Twig_Description": ""
    });
    let crop = |seasons: &[&str], phases: &[i32], regrow: i32, harvest: &str, sprite: i32| {
        json!({
            "Seasons": seasons, "DaysInPhase": phases, "RegrowDays": regrow,
            "HarvestItemId": harvest, "Texture": "TileSheets\\crops", "SpriteIndex": sprite
        })
    };
    let crops = json!({
        "Dev_TurnipSeeds": crop(&["Spring"], &[1, 1, 1, 1], -1, "Dev_Turnip", 0),
        "Dev_BeanSeeds": crop(&["Spring", "Summer"], &[1, 2, 2, 2], 3, "Dev_Bean", 1),
        "Dev_PumpkinSeeds": crop(&["Fall"], &[2, 3, 3, 3], -1, "Dev_Pumpkin", 2),
    });
    let tool = |class: &str, name: &str, sprite: i32, price: i32| {
        json!({ "ClassName": class, "Name": name, "DisplayName": name, "Texture": "TileSheets\\tools",
                "SpriteIndex": sprite, "MenuSpriteIndex": sprite, "UpgradeLevel": 0, "SalePrice": price })
    };
    let tools = json!({
        "Hoe": tool("Hoe", "Hoe", 0, 0),
        "WateringCan": tool("WateringCan", "Watering Can", 1, 0),
        "Axe": tool("Axe", "Axe", 2, 0),
        "Pickaxe": tool("Pickaxe", "Pickaxe", 3, 0),
        "Scythe": tool("MeleeWeapon", "Scythe", 4, 0),
        "FishingRod": tool("FishingRod", "Fishing Rod", 5, 0),
    });
    let shops = json!({
        "SeedShop": {
            "Owners": [{ "Name": "Tobin" }],
            "Items": [
                { "Id": "turnip", "ItemId": "(O)Dev_TurnipSeeds", "Condition": "SEASON Spring" },
                { "Id": "bean", "ItemId": "(O)Dev_BeanSeeds", "Condition": "SEASON Spring Summer" },
                { "Id": "pumpkin", "ItemId": "(O)Dev_PumpkinSeeds", "Condition": "SEASON Fall" },
                { "Id": "bread", "ItemId": "(O)Dev_Bread", "Price": 120 },
                { "Id": "fert", "ItemId": "(O)Dev_Fertilizer", "Price": 10 },
                { "Id": "speed", "ItemId": "(O)Dev_SpeedGro" },
                { "Id": "rod", "ItemId": "(T)FishingRod", "Price": 200 }
            ]
        }
    });
    let locations = json!({
        "Farm": { "DisplayName": "Farm", "DefaultArrivalTile": { "X": 20, "Y": 12 },
                  "CreateOnLoad": { "MapPath": "Maps\\Farm", "AlwaysActive": true },
                  "Fish": [
                      { "Id": "perch", "ItemId": "(O)Dev_Perch", "Chance": 0.7 },
                      { "Id": "junk", "ItemId": "(O)Dev_Driftwood", "Chance": 1.0 }
                  ] },
        "FarmHouse": { "DisplayName": "Farmhouse", "DefaultArrivalTile": { "X": 5, "Y": 8 },
                       "CreateOnLoad": { "MapPath": "Maps\\FarmHouse" } },
        "Town": { "DisplayName": "Town", "DefaultArrivalTile": { "X": 2, "Y": 15 },
                  "CreateOnLoad": { "MapPath": "Maps\\Town" },
                  "Forage": [
                      { "Id": "berry", "ItemId": "(O)Dev_Berry", "Season": "Spring", "Chance": 0.8 },
                      { "Id": "berry_summer", "ItemId": "(O)Dev_Berry", "Season": "Summer", "Chance": 0.8 }
                  ],
                  "MinDailyForageSpawn": 1, "MaxDailyForageSpawn": 3, "MaxSpawnedForageAtOnce": 6,
                  "Fish": [
                      { "Id": "darter", "ItemId": "(O)Dev_Darter", "Chance": 0.5 },
                      { "Id": "perch", "ItemId": "(O)Dev_Perch", "Chance": 0.5 },
                      { "Id": "junk", "ItemId": "(O)Dev_Driftwood", "Chance": 1.0 }
                  ] },
        "SeedShop": { "DisplayName": "General Store", "DefaultArrivalTile": { "X": 6, "Y": 9 },
                      "CreateOnLoad": { "MapPath": "Maps\\SeedShop" } },
    });
    let characters = json!({
        "Mara": { "DisplayName": "Mara", "BirthSeason": "Spring", "BirthDay": 9, "HomeRegion": "Town",
                  "Gender": "Female", "Age": "Adult", "CanBeRomanced": true,
                  "Home": [{ "Id": "Default", "Location": "Town", "Tile": { "X": 12, "Y": 8 }, "Direction": "down" }] },
        "Tobin": { "DisplayName": "Tobin", "BirthSeason": "Fall", "BirthDay": 21, "HomeRegion": "Town",
                   "Gender": "Male", "Age": "Adult",
                   "Home": [{ "Id": "Default", "Location": "SeedShop", "Tile": { "X": 6, "Y": 4 }, "Direction": "down" }] },
    });
    let mara_dialogue = json!({
        "Introduction": "Oh, you're the one who moved onto the old farm? Welcome!$h",
        "Mon": "Mondays are for long walks.",
        "Tue": "I've been reading about the stars.#$b#Did you know some of them are older than the valley itself?",
        "Wed": "Hello, @.",
        "Thu": "The river is loud today.$s",
        "Fri": "Friday at last!$h",
        "Sat": "I like the quiet mornings.",
        "Sun": "Rest is important too.",
        "spring_9": "Today's my birthday!$h",
        "rain": "I love the sound of rain.$h"
    });
    let tobin_dialogue = json!({
        "Introduction": "Welcome to my store. Seeds are on the counter.$h",
        "Mon": "Business is steady.",
        "Tue": "Plant early in the season for the best harvest.",
        "Wed": "Need anything?",
        "Thu": "Hm.",
        "Fri": "Busy day.",
        "Sat": "We're open until evening.",
        "Sun": "Even shopkeepers need a day off... not today, though."
    });
    let mara_schedule = json!({
        "default": "800 Town 20 14 2/1200 Town 30 20 3/1700 Town 12 8 2",
        "rain": "900 Town 14 9 2/1800 Town 12 8 2"
    });
    let tobin_schedule = json!({
        "default": "900 SeedShop 6 4 2/1700 Town 24 9 2/2000 SeedShop 6 4 2"
    });
    vec![
        ("Data/Objects", objects),
        ("Data/BigCraftables", bigs),
        ("Data/Machines", machines),
        ("Data/CraftingRecipes", recipes),
        ("Data/Fish", json!({
            "Dev_Perch": "Pond Perch/25/smooth/4/14/600 2600/spring summer fall/both/x/1/.4/.1/0/false",
            "Dev_Darter": "River Darter/55/dart/3/9/600 1900/spring summer/sunny/x/2/.3/.1/0/false"
        })),
        ("Strings/Objects", strings),
        ("Data/Crops", crops),
        ("Data/Tools", tools),
        ("Data/Shops", shops),
        ("Data/Locations", locations),
        ("Data/Characters", characters),
        ("Characters/Dialogue/Mara", mara_dialogue),
        ("Characters/Dialogue/Tobin", tobin_dialogue),
        ("Characters/schedules/Mara", mara_schedule),
        ("Characters/schedules/Tobin", tobin_schedule),
    ]
}
