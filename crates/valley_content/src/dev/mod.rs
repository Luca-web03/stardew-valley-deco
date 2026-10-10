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
    m.insert("Maps/dev_mine.png", art::mine_sheet().encode_png());
    m.insert("TileSheets/furniture.png", art::furniture().encode_png());
    m.insert("Characters/Monsters/Dev_Blob.png", art::monster_blob().encode_png());
    m.insert("Characters/Monsters/Dev_Moth.png", art::monster_moth().encode_png());
    m.insert("TileSheets/weapons.png", art::weapons().encode_png());
    m.insert("Animals/Dev_Hen.png", art::hen().encode_png());
    m.insert("Animals/Dev_Goat.png", art::goat().encode_png());
    m.insert("Buildings/Dev_Coop.png", art::shed(4, 3, Color::rgb(200, 170, 120), Color::rgb(170, 70, 50)).encode_png());
    m.insert("Buildings/Dev_Barn.png", art::shed(6, 4, Color::rgb(170, 90, 70), Color::rgb(110, 80, 60)).encode_png());

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
        "Dev_CopperOre": obj("Copper Ore", "Basic", -15, 5, ICON_COPPER_ORE, -300),
        "Dev_IronOre": obj("Iron Ore", "Basic", -15, 10, ICON_IRON_ORE, -300),
        "Dev_GoldOre": obj("Gold Ore", "Basic", -15, 25, ICON_GOLD_ORE, -300),
        "Dev_Coal": obj("Coal", "Basic", -15, 15, ICON_COAL, -300),
        "Dev_MineRock": obj("Stone", "Litter", 0, 0, ICON_MINE_ROCK, -300),
        "Dev_CopperNode": obj("Stone", "Litter", 0, 0, ICON_COPPER_NODE, -300),
        "Dev_IronNode": obj("Stone", "Litter", 0, 0, ICON_IRON_NODE, -300),
        "Dev_GoldNode": obj("Stone", "Litter", 0, 0, ICON_GOLD_NODE, -300),
        "Dev_Goo": obj("Mud Goo", "Basic", -28, 5, ICON_GOO, -300),
        "Dev_Egg": obj("Egg", "Basic", -5, 50, ICON_EGG, 10),
        "Dev_Bouquet": obj("Bouquet", "Basic", 0, 100, ICON_BOUQUET, -300),
        "Dev_GuardRing": { "Name": "Guard Ring", "DisplayName": "Guard Ring", "Description": "Defense +2.", "Type": "Ring", "Category": -96, "Price": 400,
                           "SpriteIndex": ICON_GUARD_RING, "CustomFields": { "Port.Defense": "2" } },
        "Dev_MagnetRing": { "Name": "Magnet Ring", "DisplayName": "Magnet Ring", "Description": "Pulls items from further away.", "Type": "Ring", "Category": -96, "Price": 400,
                            "SpriteIndex": ICON_MAGNET_RING, "CustomFields": { "Port.Magnet": "2" } },
        "Dev_Bomb": { "Name": "Blast Charge", "DisplayName": "Blast Charge", "Description": "Clears rocks in a small radius. Stand back!", "Type": "Crafting", "Category": -8, "Price": 50,
                      "SpriteIndex": ICON_BOMB, "CustomFields": { "BombRadius": "3" } },
        "Dev_CrabPot": obj("Crab Pot", "Crafting", -8, 50, ICON_CRAB_POT, -300),
        "Dev_Bait": obj("Bait", "Basic", -21, 1, ICON_BAIT, -300),
        "Dev_Crawler": obj("Pond Crawler", "Fish", -4, 60, ICON_CRAWLER, 5),
        "Dev_PlumSapling": obj("Plum Sapling", "Basic", -74, 300, ICON_PLUM_SAPLING, -300),
        "Dev_Plum": obj("Plum", "Basic", -79, 70, ICON_PLUM, 15),
        "Dev_TreeSeed": obj("Tree Seed", "Basic", -74, 5, ICON_TREE_SEED, -300),
        "Dev_Syrup": obj("Tree Syrup", "Basic", -27, 150, ICON_SYRUP, 20),
        "Dev_ArtifactSpot": obj("Artifact Spot", "Litter", 0, 0, ICON_ARTIFACT_SPOT, -300),
        "Dev_OldCoin": obj("Old Coin", "Arch", 0, 120, ICON_OLD_COIN, -300),
        "Dev_Pendant": obj("Promise Pendant", "Basic", 0, 2500, ICON_PENDANT, -300),
        "Dev_CopperBar": obj("Copper Bar", "Basic", -15, 60, ICON_COPPER_BAR, -300),
        "Dev_IronBar": obj("Iron Bar", "Basic", -15, 120, ICON_IRON_BAR, -300),
        "Dev_GoldBar": obj("Gold Bar", "Basic", -15, 250, ICON_GOLD_BAR, -300),
        "Dev_IridiumBar": obj("Iridium Bar", "Basic", -15, 1000, ICON_IRIDIUM_BAR, -300),
        "Dev_Geode": obj("Rock Nodule", "Basic", 0, 50, ICON_GEODE, -300),
        "Dev_Quartz": obj("Clear Quartz", "Minerals", -2, 40, ICON_QUARTZ, -300),
        "Dev_Amberite": obj("Amberite", "Minerals", -12, 80, ICON_AMBERITE, -300),
        "Dev_Milk": obj("Goat Milk", "Basic", -6, 120, ICON_MILK, 15),
        "Dev_Hay": obj("Hay", "Basic", 0, 25, ICON_HAY, -300),
        "Dev_Salad": {
            "Name": "Garden Salad", "DisplayName": "Garden Salad", "Description": "Crunchy and light. Makes you a little quicker.",
            "Type": "Cooking", "Category": -7, "Price": 90, "SpriteIndex": ICON_SALAD, "Edibility": 30,
            "Buffs": [{ "Id": "food", "Duration": 240, "CustomAttributes": { "Speed": 1 } }]
        },
        "Dev_Stew": {
            "Name": "Miner's Stew", "DisplayName": "Miner's Stew", "Description": "Hearty fuel for a long dig.",
            "Type": "Cooking", "Category": -7, "Price": 140, "SpriteIndex": ICON_STEW, "Edibility": 50,
            "Buffs": [{ "Id": "food", "Duration": 360, "CustomAttributes": { "MiningLevel": 2, "Defense": 1 } }]
        },
        "Dev_Dust": obj("Wing Dust", "Basic", -28, 8, ICON_DUST, -300),
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
        "Dev_Furnace": { "Name": "Furnace", "DisplayName": "Furnace", "Description": "Smelts ore into bars. Needs coal.",
                         "Price": 50, "SpriteIndex": 2 },
        "Dev_Tapper": { "Name": "Tapper", "DisplayName": "Tapper", "Description": "Place on a grown tree to collect syrup.",
                        "Price": 50, "SpriteIndex": 3 },
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
        },
        "(BC)Dev_Furnace": {
            "OutputRules": [
                { "Id": "copper", "Triggers": [{ "RequiredItemId": "(O)Dev_CopperOre", "RequiredCount": 5 }], "OutputItem": [{ "ItemId": "(O)Dev_CopperBar" }], "MinutesUntilReady": 30 },
                { "Id": "iron", "Triggers": [{ "RequiredItemId": "(O)Dev_IronOre", "RequiredCount": 5 }], "OutputItem": [{ "ItemId": "(O)Dev_IronBar" }], "MinutesUntilReady": 120 },
                { "Id": "gold", "Triggers": [{ "RequiredItemId": "(O)Dev_GoldOre", "RequiredCount": 5 }], "OutputItem": [{ "ItemId": "(O)Dev_GoldBar" }], "MinutesUntilReady": 300 }
            ],
            "AdditionalConsumedItems": [{ "ItemId": "(O)Dev_Coal", "RequiredCount": 1 }],
            "InvalidItemMessage": "The furnace needs 5 ore."
        }
    });
    let recipes = json!({
        "Storage Box": "388 20/Home/Dev_Box/true/default/",
        "Juicer": "388 15 390 10/Home/Dev_Juicer/true/default/",
        "Sprinkler": "390 5 388 5/Home/Dev_Sprinkler/false/default/",
        "Fertilizer": "771 2/Home/Dev_Fertilizer 5/false/l 1/",
        "Furnace": "Dev_CopperOre 15 390 20/Home/Dev_Furnace/true/default/",
        "Tapper": "388 30 Dev_CopperBar 1/Home/Dev_Tapper/true/default/",
        "Blast Charge": "Dev_CopperOre 4 Dev_Coal 1/Home/Dev_Bomb/false/default/",
        "Crab Pot": "388 40 Dev_IronBar 1/Home/Dev_CrabPot/false/default/",
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
        "CopperOre_Name": "Copper Ore", "CopperOre_Description": "Smelts into a soft metal.",
        "IronOre_Name": "Iron Ore", "IronOre_Description": "A sturdy ore.",
        "GoldOre_Name": "Gold Ore", "GoldOre_Description": "Shiny and valuable.",
        "Coal_Name": "Coal", "Coal_Description": "Burns hot.",
        "MudGoo_Name": "Mud Goo", "MudGoo_Description": "Left behind by a mud blob.",
        "WingDust_Name": "Wing Dust", "WingDust_Description": "Shimmering scales from a cave moth.",
        "CopperBar_Name": "Copper Bar", "CopperBar_Description": "A bar of smelted copper.",
        "IronBar_Name": "Iron Bar", "IronBar_Description": "A bar of smelted iron.",
        "GoldBar_Name": "Gold Bar", "GoldBar_Description": "A bar of smelted gold.",
        "IridiumBar_Name": "Iridium Bar", "IridiumBar_Description": "A bar of an exotic metal.",
        "RockNodule_Name": "Rock Nodule", "RockNodule_Description": "Something rattles inside. A blacksmith can crack it open.",
        "ClearQuartz_Name": "Clear Quartz", "ClearQuartz_Description": "A clear crystal.",
        "Amberite_Name": "Amberite", "Amberite_Description": "A warm, glowing stone.",
        "Bouquet_Name": "Bouquet", "Bouquet_Description": "Give it to someone you'd like to date.",
        "PromisePendant_Name": "Promise Pendant", "PromisePendant_Description": "Give it to the one you want to marry.",
        "PlumSapling_Name": "Plum Sapling", "PlumSapling_Description": "Grows into a plum tree in four weeks. Fruits in summer.",
        "Plum_Name": "Plum", "Plum_Description": "Sweet and juicy.",
        "TreeSeed_Name": "Tree Seed", "TreeSeed_Description": "Plant it to grow a tree.",
        "TreeSyrup_Name": "Tree Syrup", "TreeSyrup_Description": "Sticky and sweet.",
        "ArtifactSpot_Name": "Artifact Spot", "ArtifactSpot_Description": "Something is buried here.",
        "OldCoin_Name": "Old Coin", "OldCoin_Description": "A worn coin from long ago.",
        "CrabPot_Name": "Crab Pot", "CrabPot_Description": "Place it in water and add bait.",
        "Bait_Name": "Bait", "Bait_Description": "Crab pots need this.",
        "PondCrawler_Name": "Pond Crawler", "PondCrawler_Description": "A little clawed critter from the pond.",
        "Egg_Name": "Egg", "Egg_Description": "Fresh from the henhouse.",
        "GoatMilk_Name": "Goat Milk", "GoatMilk_Description": "Rich and creamy.",
        "Hay_Name": "Hay", "Hay_Description": "Dried grass for animal feed.",
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
        "MilkPail": tool("MilkPail", "Milk Pail", 6, 0),
    });
    let shops = json!({
        "Furniture": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "chair", "ItemId": "(F)Dev_Chair" },
                { "Id": "table", "ItemId": "(F)Dev_Table" },
                { "Id": "rug", "ItemId": "(F)Dev_Rug" },
                { "Id": "bookcase", "ItemId": "(F)Dev_Bookcase" },
                { "Id": "lamp", "ItemId": "(F)Dev_Lamp" }
            ]
        },
        "AnimalShop": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "hay", "ItemId": "(O)Dev_Hay", "Price": 50 },
                { "Id": "pail", "ItemId": "(T)MilkPail", "Price": 500 },
                { "Id": "bait", "ItemId": "(O)Dev_Bait", "Price": 5 },
                { "Id": "boots", "ItemId": "(B)Dev_WorkBoots", "Price": 500 },
                { "Id": "ring", "ItemId": "(O)Dev_GuardRing", "Price": 800 },
                { "Id": "magnet", "ItemId": "(O)Dev_MagnetRing", "Price": 800 }
            ]
        },
        "SeedShop": {
            "Owners": [{ "Name": "Tobin" }],
            "Items": [
                { "Id": "turnip", "ItemId": "(O)Dev_TurnipSeeds", "Condition": "SEASON Spring" },
                { "Id": "bean", "ItemId": "(O)Dev_BeanSeeds", "Condition": "SEASON Spring Summer" },
                { "Id": "pumpkin", "ItemId": "(O)Dev_PumpkinSeeds", "Condition": "SEASON Fall" },
                { "Id": "bread", "ItemId": "(O)Dev_Bread", "Price": 120 },
                { "Id": "fert", "ItemId": "(O)Dev_Fertilizer", "Price": 10 },
                { "Id": "speed", "ItemId": "(O)Dev_SpeedGro" },
                { "Id": "rod", "ItemId": "(T)FishingRod", "Price": 200 },
                { "Id": "bouquet", "ItemId": "(O)Dev_Bouquet", "Price": 200 },
                { "Id": "plum", "ItemId": "(O)Dev_PlumSapling", "Price": 1200 },
                { "Id": "pendant", "ItemId": "(O)Dev_Pendant", "Price": 5000 }
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
                  "ArtifactSpots": [
                      { "Id": "coin", "ItemId": "(O)Dev_OldCoin", "Chance": 0.4 },
                      { "Id": "seed", "ItemId": "(O)Dev_TreeSeed", "Chance": 0.5, "MinStack": 1, "MaxStack": 3 }
                  ],
                  "Fish": [
                      { "Id": "darter", "ItemId": "(O)Dev_Darter", "Chance": 0.5 },
                      { "Id": "perch", "ItemId": "(O)Dev_Perch", "Chance": 0.5 },
                      { "Id": "junk", "ItemId": "(O)Dev_Driftwood", "Chance": 1.0 }
                  ] },
        "SeedShop": { "DisplayName": "General Store", "DefaultArrivalTile": { "X": 6, "Y": 9 },
                      "CreateOnLoad": { "MapPath": "Maps\\SeedShop" } },
        "Mine": { "DisplayName": "Mines", "DefaultArrivalTile": { "X": 5, "Y": 8 },
                  "CreateOnLoad": { "MapPath": "Maps\\Mine" } },
        "Greenhouse": { "DisplayName": "Greenhouse", "DefaultArrivalTile": { "X": 4, "Y": 7 },
                        "CreateOnLoad": { "MapPath": "Maps\\Greenhouse" } },
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
        "Wed": "Hello, @.#$b#$q 7001/7002 Wed_after#Do you like stargazing?#$r 7001 20 star_yes#I love it!#$r 7002 -5 star_no#Not really.",
        "star_yes": "Then we should watch the meteor shower together sometime.$h",
        "star_no": "Oh. Well, it's not for everyone.$s",
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
            "Dev_Darter": "River Darter/55/dart/3/9/600 1900/spring summer/sunny/x/2/.3/.1/0/false",
            "Dev_Crawler": "Pond Crawler/trap/.6/x/freshwater/1/6/false"
        })),
        ("Data/Furniture", json!({
            "Dev_Chair": "Wooden Chair/chair/1 2/1 1/4/150/-1/Wooden Chair/0",
            "Dev_Table": "Small Table/table/2 2/2 1/1/300/-1/Small Table/2",
            "Dev_Rug": "Woven Rug/rug/3 2/3 2/1/250/-1/Woven Rug/4",
            "Dev_Bookcase": "Bookcase/bookcase/2 3/2 1/1/450/-1/Bookcase/8",
            "Dev_Lamp": "Floor Lamp/lamp/1 2/1 1/1/200/-1/Floor Lamp/11"
        })),
        ("Data/Boots", json!({
            "Dev_WorkBoots": "Work Boots/Sturdy and comfortable./300/2/0/0/Work Boots/0/55/Maps\\springobjects"
        })),
        ("Strings/Objects", strings),
        ("Data/CookingRecipes", json!({
            "Garden Salad": "Dev_Turnip 1 Dev_Bean 1/1 1/Dev_Salad/default/",
            "Miner's Stew": "Dev_Pumpkin 1 Dev_Goo 2/1 1/Dev_Stew/default/"
        })),
        ("Data/mail", json!({
            "spring_2": "Hi @,^The general store has fresh seeds every season, and I set aside a few for you.^- Tobin%item object Dev_TurnipSeeds 10 %%[#]Seeds for a new farmer",
            "spring_5": "@,^The caves north of town are full of ore, and of things that bite. Come prepared!^- Mara[#]A warning"
        })),
        ("Data/Quests", json!({
            "Dev_Q1": "ItemHarvest/Getting Started/Plant your first crop and see it through to harvest./Harvest a turnip from your farm./Dev_Turnip/Dev_Q2/50//false/",
            "Dev_Q2": "ItemDelivery/A Gift for Mara/Mara would love to try something from your farm./Bring Mara a turnip./Mara Dev_Turnip 1/-1/100//false/Oh, it's lovely! Thank you, @."
        })),
        ("Data/Buildings", json!({
            "Coop": {
                "Name": "Henhouse", "Description": "Houses up to four hens.", "Texture": "Buildings\\Dev_Coop",
                "Size": { "X": 4, "Y": 3 }, "BuildCost": 1000, "BuildDays": 1,
                "BuildMaterials": [{ "ItemId": "(O)388", "Amount": 100 }, { "ItemId": "(O)390", "Amount": 50 }],
                "HumanDoor": { "X": 2, "Y": 2 }, "AnimalDoor": { "X": 0, "Y": 2, "Width": 1, "Height": 1 },
                "IndoorMap": "Coop", "MaxOccupants": 4, "ValidOccupantTypes": ["Coop"]
            },
            "Barn": {
                "Name": "Goat Barn", "Description": "Houses up to four goats.", "Texture": "Buildings\\Dev_Barn",
                "Size": { "X": 6, "Y": 4 }, "BuildCost": 2500, "BuildDays": 2,
                "BuildMaterials": [{ "ItemId": "(O)388", "Amount": 150 }, { "ItemId": "(O)390", "Amount": 100 }],
                "HumanDoor": { "X": 3, "Y": 3 }, "AnimalDoor": { "X": 0, "Y": 3, "Width": 1, "Height": 1 },
                "IndoorMap": "Barn", "MaxOccupants": 4, "ValidOccupantTypes": ["Barn"]
            }
        })),
        ("Data/FarmAnimals", json!({
            "Dev_Hen": {
                "DisplayName": "Speckled Hen", "House": "Coop", "PurchasePrice": 400, "SellPrice": 200,
                "Texture": "Animals\\Dev_Hen", "SpriteWidth": 16, "SpriteHeight": 16,
                "DaysToMature": 2, "DaysToProduce": 1, "HarvestType": "DropOvernight",
                "ProduceItemIds": [{ "ItemId": "(O)Dev_Egg" }], "Sound": "cluck"
            },
            "Dev_Goat": {
                "DisplayName": "Goat", "House": "Barn", "PurchasePrice": 1200, "SellPrice": 600,
                "Texture": "Animals\\Dev_Goat", "SpriteWidth": 32, "SpriteHeight": 32,
                "DaysToMature": 3, "DaysToProduce": 2, "HarvestType": "HarvestWithTool", "HarvestTool": "Milk Pail",
                "ProduceItemIds": [{ "ItemId": "(O)Dev_Milk" }], "Sound": "goat"
            }
        })),
        ("Data/Events/Town", json!({
            "Dev_Intro/t 600 1800": "none/6 15/farmer 2 15 1 Mara 12 9 2/skippable/pause 400/emote Mara 16/move Mara 0 6 2/move Mara -8 0 3/speak Mara \"Oh! You must be the new farmer. I'm Mara.#$b#The general store is just up the road, and the old caves are north-west of here.$h\"/faceDirection Mara 1/pause 300/move Mara 8 0 1/move Mara 0 -6 0/friendship Mara 50/end"
        })),
        ("Data/Festivals/FestivalDates", json!({ "fall16": "Harvest Fair" })),
        ("Data/FruitTrees", json!({
            "Dev_PlumSapling": { "DisplayName": "Plum Tree", "Seasons": ["Summer"], "Fruit": [{ "ItemId": "(O)Dev_Plum" }], "TextureSpriteRow": 0 }
        })),
        ("Data/WildTrees", json!({
            "1": { "SeedItemId": "(O)Dev_TreeSeed", "GrowthChance": 0.2, "TapItems": [{ "ItemId": "(O)Dev_Syrup", "DaysUntilReady": 5 }] }
        })),
        ("Data/Festivals/fall16", json!({
            "name": "Harvest Fair",
            "conditions": "Town/900 1500",
            "host": "Tobin",
            "set-up": "none/20 15/farmer 2 15 1 Tobin 20 13 2 Mara 16 17 1/pause 300/viewport 10 15/emote Mara 32",
            "mainEvent": "pause 200/speak Tobin \"Welcome, everyone, to the Harvest Fair!#$b#The judges have tasted every pumpkin in town... and the prize goes to @!$h\"/money 500/emote Mara 20/speak Mara \"Congratulations!$h\"/pause 400/end",
            "Mara": "I entered my best berries this year. Fingers crossed!$h",
            "Tobin": "Talk to me when you're ready for the judging."
        })),
        ("Data/Bundles", json!({
            "Garden/0": "Spring Basket/O Dev_Sprinkler 2/Dev_Turnip 3 0 Dev_Bean 3 0 Dev_Berry 2 0/0/3/Spring Basket",
            "Garden/1": "Fresh Catch/O Dev_SpeedGro 5/Dev_Perch 1 0 Dev_Darter 1 0/1/2/Fresh Catch",
            "Workshop/0": "Builder's Bundle/BO Dev_Juicer 1/388 50 0 390 50 0 771 20 0/2/3/Builder's Bundle",
            "Workshop/1": "Smith's Bundle/O Dev_IronBar 3/Dev_CopperBar 2 0 Dev_Quartz 1 0 Dev_Coal 5 0/3/3/Smith's Bundle",
            "Workshop/2": "Repair Fund/O Dev_GoldBar 1/-1 1000 1000/4/1/Repair Fund"
        })),
        ("Data/MuseumRewards", json!({
            "first_minerals": { "TargetContextTags": [{ "Tag": "item_type_minerals", "Count": 1 }], "RewardItemId": "(O)Dev_SpeedGro", "RewardItemCount": 5 },
            "two_minerals": { "TargetContextTags": [{ "Tag": "item_type_minerals", "Count": 2 }], "RewardItemId": "(BC)Dev_Furnace", "RewardItemCount": 1 }
        })),
        ("Data/Monsters", json!({
            "Dev_Blob": "24/5/0/0/false/1000/Dev_Goo .6 Dev_CopperOre .1/0/.01/6/2/.0/true/3/Mud Blob",
            "Dev_Moth": "14/4/0/0/true/1000/Dev_Dust .5/0/.01/8/3/.1/true/4/Cave Moth"
        })),
        ("Data/Weapons", json!({
            "Dev_Sword": { "Name": "Training Sword", "DisplayName": "Training Sword",
                           "Description": "A light, dull blade. Better than nothing.",
                           "Type": 3, "MinDamage": 4, "MaxDamage": 8, "Knockback": 1.0, "Speed": 0,
                           "CritChance": 0.02, "CritMultiplier": 3.0, "Texture": "TileSheets\\weapons", "SpriteIndex": 0 }
        })),
        ("Data/Crops", crops),
        ("Data/Tools", tools),
        ("Data/Shops", shops),
        ("Data/Locations", locations),
        ("Data/Characters", characters),
        ("Characters/Dialogue/Mara", mara_dialogue),
        ("Characters/Dialogue/Tobin", tobin_dialogue),
        ("Characters/Dialogue/MarriageDialogueMara", json!({
            "Indoor_Day_0": "Morning! I made tea. Don't let it go cold.$h",
            "Indoor_Day_1": "I rearranged the shelves. I think it looks better, don't you?",
            "Rainy_Day_0": "Listen to that rain. The crops will love it.$h",
            "Outdoor_0": "I'll be outside enjoying the sun for a while."
        })),
        ("Characters/schedules/Mara", mara_schedule),
        ("Characters/schedules/Tobin", tobin_schedule),
    ]
}
