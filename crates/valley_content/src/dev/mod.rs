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
    m.insert("Buildings/Dev_Silo.png", art::silo().encode_png());
    m.insert("Buildings/Dev_Pond.png", art::pond().encode_png());
    m.insert("Buildings/Dev_Obelisk.png", art::obelisk().encode_png());
    m.insert("Buildings/Dev_Hut.png", art::hut().encode_png());
    m.insert("Characters/Monsters/Dev_Blob.png", art::monster_blob().encode_png());
    m.insert("Characters/Monsters/Dev_Moth.png", art::monster_moth().encode_png());
    m.insert("TileSheets/weapons.png", art::weapons().encode_png());
    m.insert("Animals/Dev_Hen.png", art::hen().encode_png());
    m.insert("Animals/Dev_Goat.png", art::goat().encode_png());
    m.insert("Animals/Dev_Dog.png", art::dog().encode_png());
    m.insert("Animals/Dev_Horse.png", art::horse().encode_png());
    m.insert("Buildings/Dev_Stable.png", art::shed(4, 2, Color::rgb(150, 110, 70), Color::rgb(90, 110, 70)).encode_png());
    m.insert("LooseSprites/Dev_Cart.png", art::cart().encode_png());
    m.insert("LooseSprites/Port_Crow.png", art::crow().encode_png());
    m.insert("TileSheets/Dev_GiantCrops.png", art::giant_crops().encode_png());
    m.insert("Characters/Dev_Child.png", art::child().encode_png());
    m.insert("Characters/Farmer/Dev_Hats.png", art::hats().encode_png());
    m.insert("TileSheets/Dev_Trinkets.png", art::trinkets().encode_png());
    m.insert("TileSheets/Dev_Clothes.png", art::clothes().encode_png());
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
    let mut objects = json!({
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
        "709": obj("Hardwood", "Basic", -16, 15, ICON_HARDWOOD, -300),
        "72": obj("Diamond", "Minerals", -2, 750, ICON_DIAMOND, -300),
        "386": obj("Iridium Ore", "Basic", -15, 100, ICON_IRIDIUM_ORE, -300),
        "Dev_Bread": obj("Bread", "Cooking", -7, 60, ICON_BREAD, 20),
        "Dev_BerryTart": obj("Berry Tart", "Cooking", -7, 180, ICON_BERRY_TART, 40),
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
        "Dev_Cloth": { "Name": "Cloth", "DisplayName": "Cloth", "Description": "Woven fabric for the sewing machine.", "Type": "Crafting", "Category": -8, "Price": 100,
                       "SpriteIndex": ICON_FIBER, "ContextTags": ["item_cloth"] },
        "Dev_BookGarden": { "Name": "The Weekend Gardener", "DisplayName": "The Weekend Gardener", "Description": "Tips for busy growers. Reading it teaches you a few things.", "Type": "Basic", "Category": 0, "Price": 500,
                            "SpriteIndex": ICON_BOOK, "ContextTags": ["book_item"], "CustomFields": { "Port.BookXp": "Farming 250" } },
        "Dev_BookDunes": { "Name": "Dune Lore", "DisplayName": "Dune Lore", "Description": "Stories of travelers who crossed the dunes.", "Type": "Basic", "Category": 0, "Price": 800,
                           "SpriteIndex": ICON_BOOK, "ContextTags": ["book_item"], "CustomFields": { "Port.BookPower": "Dev_DuneWalker", "Port.BookXp": "Foraging 100" } },
        "Dev_SunShell": { "Name": "Sun Shell", "DisplayName": "Sun Shell", "Description": "A warm golden shell. The islanders trade favors for them.", "Type": "Basic", "Category": 0, "Price": 0,
                          "SpriteIndex": ICON_SUN_SHELL, "CustomFields": { "Port.Collectible": "SunShell" } },
        "Dev_EmberShard": { "Name": "Ember Shard", "DisplayName": "Ember Shard", "Description": "Still warm. The forge in the volcano runs on these.", "Type": "Basic", "Category": 0, "Price": 50,
                            "SpriteIndex": ICON_EMBER_SHARD },
        "Dev_Ticket": { "Name": "Movie Ticket", "DisplayName": "Movie Ticket", "Description": "Good for one showing. Give it to a friend to invite them along.", "Type": "Basic", "Category": 0, "Price": 250,
                        "SpriteIndex": ICON_TICKET, "CanBeGivenAsGift": false },
        "Dev_NoteScrap": { "Name": "Note Scrap", "DisplayName": "Note Scrap", "Description": "A torn page with writing on it. Read it to see what it says.", "Type": "Basic", "Category": 0, "Price": 1,
                           "SpriteIndex": ICON_NOTE_SCRAP },
        "Dev_HomingStone": { "Name": "Homing Stone", "DisplayName": "Homing Stone", "Description": "Squeeze it to return to your farm in a flash.", "Type": "Crafting", "Category": -8, "Price": 20,
                             "SpriteIndex": ICON_HOMING_STONE, "CustomFields": { "Port.WarpTo": "Farm" } },
        "Dev_DuneCharm": { "Name": "Dune Charm", "DisplayName": "Dune Charm", "Description": "Smells faintly of hot sand. Takes you to the dunes.", "Type": "Crafting", "Category": -8, "Price": 40,
                           "SpriteIndex": ICON_DUNE_CHARM, "CustomFields": { "Port.WarpTo": "Dev_Dunes 10 14" } },
        "Dev_Bomb": { "Name": "Blast Charge", "DisplayName": "Blast Charge", "Description": "Clears rocks in a small radius. Stand back!", "Type": "Crafting", "Category": -8, "Price": 50,
                      "SpriteIndex": ICON_BOMB, "CustomFields": { "BombRadius": "3" } },
        "Dev_CrabPot": obj("Crab Pot", "Crafting", -8, 50, ICON_CRAB_POT, -300),
        "Dev_Bait": obj("Bait", "Basic", -21, 1, ICON_BAIT, -300),
        "Dev_WhirlLure": { "Name": "Whirl Lure", "DisplayName": "Whirl Lure", "Description": "A spinning lure. Fish bite sooner. Attach to an upgraded rod.", "Type": "Basic", "Category": -22, "Price": 250,
                           "SpriteIndex": ICON_WHIRL_LURE, "CustomFields": { "Port.Tackle": "bite_speed" } },
        "Dev_CorkFloat": { "Name": "Cork Float", "DisplayName": "Cork Float", "Description": "Makes the catch bar a little taller. Attach to an upgraded rod.", "Type": "Basic", "Category": -22, "Price": 250,
                           "SpriteIndex": ICON_CORK_FLOAT, "CustomFields": { "Port.Tackle": "bar_size" } },
        "Dev_Crawler": obj("Pond Crawler", "Fish", -4, 60, ICON_CRAWLER, 5),
        "Dev_Battery": obj("Storm Cell", "Basic", 0, 500, ICON_BATTERY, -300),
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
        "Dev_RainTotem": {
            "Name": "Rain Totem", "DisplayName": "Rain Totem", "Description": "Use it to make it rain tomorrow.",
            "Type": "Crafting", "Category": -8, "Price": 20, "SpriteIndex": ICON_RAIN_TOTEM, "CustomFields": { "Port.SetWeather": "Rain" }
        },
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
    // Engine convention: slingshot ammo damage on the placeholder ores.
    for (id, dmg) in [("Dev_CopperOre", "10"), ("Dev_IronOre", "20"), ("Dev_GoldOre", "30"), ("Dev_Coal", "15")] {
        objects[id]["CustomFields"] = json!({ "Port.AmmoDamage": dmg });
    }
    objects["Dev_Juice"]["CustomFields"] = json!({ "Port.AgingRate": "2.0" });
    let bigs = json!({
        "Dev_Box": { "Name": "Storage Box", "DisplayName": "Storage Box", "Description": "Holds 36 items.",
                     "Price": 1, "SpriteIndex": 0, "ContextTags": ["storage_item"] },
        "Dev_Juicer": { "Name": "Juicer", "DisplayName": "Juicer", "Description": "Turns produce into juice.",
                        "Price": 50, "SpriteIndex": 1 },
        "Dev_Furnace": { "Name": "Furnace", "DisplayName": "Furnace", "Description": "Smelts ore into bars. Needs coal.",
                         "Price": 50, "SpriteIndex": 2 },
        "Dev_Tapper": { "Name": "Tapper", "DisplayName": "Tapper", "Description": "Place on a grown tree to collect syrup.",
                        "Price": 50, "SpriteIndex": 3 },
        "Dev_LightningRod": { "Name": "Lightning Rod", "DisplayName": "Lightning Rod", "Description": "Catches lightning during storms.",
                              "Price": 50, "SpriteIndex": 4 },
        "Dev_Cask": { "Name": "Cask", "DisplayName": "Cask", "Description": "Ages drinks to a higher quality over time.",
                      "Price": 50, "SpriteIndex": 5, "CustomFields": { "Port.Cask": "true" } },
        "Dev_SewingMachine": { "Name": "Sewing Machine", "DisplayName": "Sewing Machine", "Description": "Turns cloth into clothing.",
                               "Price": 50, "SpriteIndex": 6, "CustomFields": { "Port.SewingMachine": "true" } },
        "Dev_Scarecrow": { "Name": "Straw Scarecrow", "DisplayName": "Straw Scarecrow", "Description": "Keeps crows off the crops within 8 tiles.",
                           "Price": 50, "SpriteIndex": 7, "ContextTags": ["crow_scare", "crow_scare_radius_9"] },
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
        "Homing Stone": "390 10 Dev_Quartz 1/Home/Dev_HomingStone 3/false/default/",
        "Cask": "388 20/Home/Dev_Cask/true/default/",
        "Sewing Machine": "388 10 Dev_IronBar 1/Home/Dev_SewingMachine/true/default/",
        "Crab Pot": "388 40 Dev_IronBar 1/Home/Dev_CrabPot/false/default/",
        "Lightning Rod": "Dev_IronBar 1 Dev_Quartz 1/Home/Dev_LightningRod/true/default/",
        "Straw Scarecrow": "388 50 771 20 Dev_Coal 1/Home/Dev_Scarecrow/true/default/",
        "Rain Totem": "388 1 92 5 Dev_Berry 1/Home/Dev_RainTotem/false/default/",
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
        "Dev_Tailor": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "straw", "ItemId": "(H)Dev_StrawHat", "Price": 300 },
                { "Id": "cap", "ItemId": "(H)Dev_Cap", "Price": 250 },
                { "Id": "beanie", "ItemId": "(H)Dev_Beanie", "Price": 250, "Condition": "SEASON Fall Winter" },
                { "Id": "tee", "ItemId": "(S)Dev_Tee" },
                { "Id": "stripes", "ItemId": "(S)Dev_Stripes" },
                { "Id": "work", "ItemId": "(P)Dev_WorkPants" },
                { "Id": "shorts", "ItemId": "(P)Dev_Shorts", "Condition": "SEASON Summer" }
            ]
        },
        "AdventureShop": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "sword", "ItemId": "(W)Dev_Sword", "Price": 250 },
                { "Id": "dagger", "ItemId": "(W)Dev_Dagger", "Price": 350 },
                { "Id": "club", "ItemId": "(W)Dev_Club", "Price": 900 },
                { "Id": "sling", "ItemId": "(W)Dev_Slingshot", "Price": 500 },
                { "Id": "bomb", "ItemId": "(O)Dev_Bomb", "Price": 150 }
            ]
        },
        "Dev_Bookstall": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "garden", "ItemId": "(O)Dev_BookGarden" },
                { "Id": "dunes", "ItemId": "(O)Dev_BookDunes" },
                { "Id": "pebble", "ItemId": "(TR)Dev_PebbleCharm", "Price": 2000 },
                { "Id": "feather", "ItemId": "(TR)Dev_SwiftFeather", "Price": 3000 }
            ]
        },
        "Dev_MarketStall": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "tart", "ItemId": "(O)Dev_BerryTart", "Price": 300 },
                { "Id": "charm", "ItemId": "(O)Dev_DuneCharm", "Price": 400 },
                { "Id": "cap", "ItemId": "(H)Dev_Cap", "Price": 600, "AvailableStock": 1 }
            ]
        },
        "Dev_PrizeCounter": {
            "Currency": 2,
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "charm", "ItemId": "(O)Dev_DuneCharm", "Price": 300 },
                { "Id": "beanie", "ItemId": "(H)Dev_Beanie", "Price": 1500 },
                { "Id": "stripes", "ItemId": "(S)Dev_Stripes", "Price": 1000 }
            ]
        },
        "Dev_BoxOffice": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [{ "Id": "ticket", "ItemId": "(O)Dev_Ticket", "Price": 500 }]
        },
        "Dev_DuneTrader": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "charm", "ItemId": "(O)Dev_DuneCharm", "TradeItemId": "(O)Dev_Quartz", "TradeItemAmount": 3 },
                { "Id": "cloth", "ItemId": "(O)Dev_Cloth", "Price": 50, "TradeItemId": "(O)Dev_Driftwood", "TradeItemAmount": 2 },
                { "Id": "cask", "ItemId": "(BC)Dev_Cask", "TradeItemId": "(O)Dev_GoldBar", "TradeItemAmount": 1, "AvailableStock": 1 }
            ]
        },
        "Dev_TokenBooth": {
            "Currency": 1,
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "straw", "ItemId": "(H)Dev_StrawHat", "Price": 40 },
                { "Id": "tart", "ItemId": "(O)Dev_BerryTart", "Price": 15 }
            ]
        },
        "Traveler": {
            "Owners": [{ "Name": "AnyOrNone" }],
            "Items": [
                { "Id": "random", "ItemId": "RANDOM_ITEMS (O) @requirePrice @isRandomSale", "MaxItems": 6, "AvailableStock": 1 },
                { "Id": "sapling", "RandomItemId": ["(O)Dev_PlumSapling", "(O)Dev_SpeedGro"], "Price": 900, "AvailableStock": 1 }
            ]
        },
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
                { "Id": "lure", "ItemId": "(O)Dev_WhirlLure", "Price": 500 },
                { "Id": "float", "ItemId": "(O)Dev_CorkFloat", "Price": 500 },
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
        "Dev_Dunes": { "DisplayName": "Sunscorch Dunes", "DefaultArrivalTile": { "X": 6, "Y": 16 },
                       "CreateOnLoad": { "MapPath": "Maps\\Dev_Dunes" } },
        "Dev_Isle": { "DisplayName": "Breezy Isle", "DefaultArrivalTile": { "X": 4, "Y": 14 },
                      "CreateOnLoad": { "MapPath": "Maps\\Dev_Isle" },
                      "Forage": [{ "Id": "shell", "ItemId": "(O)Dev_SunShell", "Chance": 0.9 }] },
        "Dev_Parlor": { "DisplayName": "Dune Parlor", "DefaultArrivalTile": { "X": 6, "Y": 8 },
                        "CreateOnLoad": { "MapPath": "Maps\\Dev_Parlor" } },
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
        ("Port/JojaProjects", json!([
            { "Id": "Garden", "Area": "Garden", "DisplayName": "Garden Renewal", "Description": "Replant the town garden.", "Price": 8000, "Flag": "jojaGarden" },
            { "Id": "Workshop", "Area": "Workshop", "DisplayName": "Workshop Overhaul", "Description": "Re-equip the old workshop.", "Price": 12000, "Flag": "jojaWorkshop" }
        ])),
        ("Data/GiantCrops", json!({
            "Dev_Pumpkin": {
                "FromItemId": "(O)Dev_Pumpkin",
                "HarvestItems": [{ "ItemId": "(O)Dev_Pumpkin", "MinStack": 15, "MaxStack": 21 }],
                "Texture": "TileSheets\\Dev_GiantCrops",
                "TexturePosition": { "X": 0, "Y": 0 },
                "TileSize": { "X": 3, "Y": 3 }
            }
        })),
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
        ("Strings/SpecialOrderStrings", json!({
            "Dev_FishFry_Name": "Riverside Fish Fry",
            "Dev_FishFry_Text": "We're planning a fish fry down by the river this weekend. Could you catch a few for the grill?"
        })),
        ("Data/SpecialOrders", json!({
            "Dev_TurnipDrive": {
                "Name": "Turnip Drive", "Requester": "Tobin", "Duration": "Week", "Repeatable": true,
                "Text": "The shelves are bare and folks keep asking for turnips. Bring me a crate's worth and I'll make it worth your while.",
                "Objectives": [{ "Type": "Deliver", "Text": "Bring turnips to Tobin", "RequiredCount": "10",
                                 "Data": { "AcceptedContextTags": "id_o_dev_turnip", "TargetName": "Tobin" } }],
                "Rewards": [{ "Type": "Money", "Data": { "Amount": "800" } }, { "Type": "Friendship", "Data": {} }]
            },
            "Dev_CaveCritters": {
                "Name": "Cave Critters", "Requester": "Mara", "Duration": "Week", "Repeatable": true,
                "Text": "Something in the old caves keeps nibbling the lantern wicks. Thin out the blobs down there?",
                "Objectives": [{ "Type": "Slay", "Text": "Defeat mud blobs", "RequiredCount": "10", "Data": { "TargetName": "Dev_Blob" } }],
                "Rewards": [{ "Type": "Money", "Data": { "Amount": "1200" } }]
            },
            "Dev_FishFry": {
                "Name": "[LocalizedText Strings\\SpecialOrderStrings:Dev_FishFry_Name]", "Requester": "Mara", "Duration": "Week",
                "Text": "[LocalizedText Strings\\SpecialOrderStrings:Dev_FishFry_Text]",
                "Objectives": [{ "Type": "Fish", "Text": "Catch fish", "RequiredCount": "5", "Data": {} }],
                "Rewards": [{ "Type": "Money", "Data": { "Amount": "600" } }, { "Type": "Mail", "Data": { "MailReceived": "Dev_FishFryDone" } }]
            },
            "Dev_DeepDown": {
                "Name": "Deep Down", "Requester": "Tobin", "Duration": "TwoWeeks", "RequiredTags": "!season_winter",
                "Text": "My grandfather swore there's a cavern lake below the tenth floor. Care to settle the argument?",
                "Objectives": [{ "Type": "ReachMineFloor", "Text": "Reach floor 10 of the mines", "RequiredCount": "10", "Data": {} }],
                "Rewards": [{ "Type": "Money", "Data": { "Amount": "1500" } }]
            }
        })),
        ("Data/CookingRecipes", json!({
            "Garden Salad": "Dev_Turnip 1 Dev_Bean 1/1 1/Dev_Salad/default/",
            "Miner's Stew": "Dev_Pumpkin 1 Dev_Goo 2/1 1/Dev_Stew/default/",
            "Berry Tart": "Dev_Berry 3 Dev_Bread 1/1 1/Dev_BerryTart/null/"
        })),
        ("Data/mail", json!({
            "spring_2": "Hi @,^The general store has fresh seeds every season, and I set aside a few for you.^- Tobin%item object Dev_TurnipSeeds 10 %%[#]Seeds for a new farmer",
            "spring_5": "@,^The caves north of town are full of ore, and of things that bite. Come prepared!^- Mara[#]A warning"
        })),
        ("Data/Quests", json!({
            "Dev_Q1": "ItemHarvest/Getting Started/Plant your first crop and see it through to harvest./Harvest a turnip from your farm./Dev_Turnip/Dev_Q2/50//false/",
            "Dev_Q2": "ItemDelivery/A Gift for Mara/Mara would love to try something from your farm./Bring Mara a turnip./Mara Dev_Turnip 1/-1/100//false/Oh, it's lovely! Thank you, @."
        })),
        ("Data/TV/CookingChannel", json!({
            "1": "Berry Tart/Today on Kitchen Corner: a crumbly crust, a pile of berries, and a slice of bread to soak up the juice. Bake until bubbling!",
            "2": "Garden Salad/Back to basics: crisp turnips and beans with a squeeze of something sour. Fresh is best!"
        })),
        ("Data/Minecarts", json!({
            "Default": {
                "UnlockCondition": "PLAYER_HAS_MAIL Current ccDone_Workshop",
                "LockedMessage": "The minecarts are out of order. A notice says repairs are underway.",
                "ChooseDestinationMessage": "Where to?",
                "Destinations": [
                    { "Id": "Farm", "DisplayName": "Farm", "TargetLocation": "Farm", "TargetTile": { "X": 40, "Y": 12 }, "TargetDirection": "down" },
                    { "Id": "Town", "DisplayName": "Town", "TargetLocation": "Town", "TargetTile": { "X": 7, "Y": 4 }, "TargetDirection": "down" },
                    { "Id": "Dunes", "DisplayName": "Sunscorch Dunes", "Price": 50, "TargetLocation": "Dev_Dunes", "TargetTile": { "X": 14, "Y": 16 }, "TargetDirection": "down" }
                ]
            }
        })),
        ("Data/PassiveFestivals", json!({
            "Dev_LanternMarket": {
                "DisplayName": "Lantern Market", "ShowOnCalendar": true, "Season": "Summer", "StartDay": 10, "EndDay": 12,
                "StartTime": 1700, "StartMessage": "The Lantern Market is open in the dunes tonight!", "OnlyShowMessageOnFirstDay": false,
                "MapReplacements": { "Dev_Dunes": "Dev_Dunes_Market" }
            }
        })),
        ("Data/Trinkets", json!({
            "Dev_PebbleCharm": { "DisplayName": "Pebble Charm", "Description": "A smooth river stone on a cord. Defense +2.", "Texture": "TileSheets\\Dev_Trinkets", "SheetIndex": 0,
                                 "CustomFields": { "Port.TrinketStats": "defense 2" } },
            "Dev_SwiftFeather": { "DisplayName": "Swift Feather", "Description": "Light as air. Speed +1, attack +1.", "Texture": "TileSheets\\Dev_Trinkets", "SheetIndex": 1,
                                  "CustomFields": { "Port.TrinketStats": "speed 1 attack 1" } }
        })),
        ("Data/Powers", json!({
            "Dev_DuneWalker": { "DisplayName": "Dune Walker", "Description": "You know the safe paths across the sand." }
        })),
        ("Data/Movies", json!([
            { "Id": "Dev_LanternFlight", "Seasons": ["Spring", "Summer"], "Title": "Lantern Flight", "Tags": ["family", "adventure"],
              "Description": "A paper lantern drifts across the valley in search of the moon.",
              "Scenes": [
                  { "Id": "0", "Text": "A paper lantern slips its string and rises over the rooftops." },
                  { "Id": "1", "Text": "It drifts past sleeping fields and a very surprised owl." },
                  { "Id": "2", "Text": "At dawn it settles on a hill, glowing beside a field of fireflies." }
              ] },
            { "Id": "Dev_IronWinter", "Seasons": ["Fall", "Winter"], "Title": "The Iron Winter", "Tags": ["drama"],
              "Description": "A blacksmith keeps a frozen town warm through the longest night.",
              "Scenes": [
                  { "Id": "0", "Text": "Snow buries the town; only the forge glows." },
                  { "Id": "1", "Text": "The smith works through the night, hammer ringing like a bell." },
                  { "Id": "2", "Text": "Morning comes, and every chimney smokes again." }
              ] }
        ])),
        ("Data/MoviesReactions", json!([
            { "NPCName": "Mara", "Reactions": [{ "Tag": "adventure", "Response": "love" }, { "Tag": "drama", "Response": "dislike" }] },
            { "NPCName": "Tobin", "Reactions": [{ "Tag": "drama", "Response": "like" }] }
        ])),
        ("Data/TailoringRecipes", json!([
            { "Id": "Dev_StripesFromBerries", "FirstItemTags": ["item_cloth"], "SecondItemTags": ["id_o_dev_berry"], "CraftedItemId": "(S)Dev_Stripes" },
            { "Id": "Dev_ShortsFromBeans", "FirstItemTags": ["item_cloth"], "SecondItemTags": ["id_o_dev_bean"], "CraftedItemId": "(P)Dev_Shorts" },
            { "Id": "Dev_HatFromAnything", "FirstItemTags": ["item_cloth"], "SecondItemTags": ["category_fish"], "SpendRightItem": false, "CraftedItemIds": ["(H)Dev_Cap", "(H)Dev_Beanie"] }
        ])),
        ("Data/SecretNotes", json!({
            "1": "Day 3. The well behind the store hums at night. Nobody else seems to hear it.",
            "2": "If you're reading this: the third boulder in the dunes is hollow.^Bring a pick.",
            "3": "Recipe for courage: one warm meal, one good friend, and an early start."
        })),
        ("Data/Hats", json!({
            "Dev_StrawHat": "Straw Hat/Woven for long days in the sun./true/false//Straw Hat/0/Characters\\Farmer\\Dev_Hats",
            "Dev_Cap": "Field Cap/Keeps the glare out of your eyes./true/false//Field Cap/1/Characters\\Farmer\\Dev_Hats",
            "Dev_Beanie": "Knit Beanie/Warm and a little itchy./hide/false//Knit Beanie/2/Characters\\Farmer\\Dev_Hats"
        })),
        ("Data/Shirts", json!({
            "Dev_Tee": { "Name": "Plain Tee", "DisplayName": "Plain Tee", "Description": "A soft cotton shirt.", "Price": 100, "Texture": "TileSheets\\Dev_Clothes", "SpriteIndex": 3, "DefaultColor": "240 240 235" },
            "Dev_Stripes": { "Name": "Sailor Stripes", "DisplayName": "Sailor Stripes", "Description": "Blue with white stripes.", "Price": 250, "Texture": "TileSheets\\Dev_Clothes", "SpriteIndex": 4, "DefaultColor": "70 120 190" }
        })),
        ("Data/Pants", json!({
            "Dev_WorkPants": { "Name": "Work Pants", "DisplayName": "Work Pants", "Description": "Sturdy and patched at the knees.", "Price": 200, "Texture": "TileSheets\\Dev_Clothes", "SpriteIndex": 5, "DefaultColor": "90 80 70" },
            "Dev_Shorts": { "Name": "Khaki Shorts", "DisplayName": "Khaki Shorts", "Description": "For hot days.", "Price": 150, "Texture": "TileSheets\\Dev_Clothes", "SpriteIndex": 6, "DefaultColor": "170 140 90" }
        })),
        ("Data/FishPondData", json!([
            { "Id": "Dev_Perch", "RequiredTags": ["id_o_dev_perch"], "SpawnTime": 2, "MaxPopulation": 7,
              "ProducedItems": [
                  { "ItemId": "(O)Dev_Bait", "RequiredPopulation": 1, "Chance": 0.6, "MinQuantity": 2, "MaxQuantity": 5 },
                  { "ItemId": "(O)Dev_Driftwood", "RequiredPopulation": 0, "Chance": 0.2, "MinQuantity": 1, "MaxQuantity": 1 }
              ],
              "PopulationGates": { "5": ["(O)Dev_Berry 2 4"] } },
            { "Id": "Dev_AnyFish", "RequiredTags": ["category_fish"],
              "ProducedItems": [{ "ItemId": "(O)Dev_Bait", "RequiredPopulation": 1, "Chance": 0.4, "MinQuantity": 1, "MaxQuantity": 3 }] }
        ])),
        ("Data/Buildings", json!({
            "Dev_HarvestHut": {
                "Name": "Harvest Hut", "Description": "Gathers ripe crops nearby each night and keeps them inside.", "Texture": "Buildings\\Dev_Hut",
                "Size": { "X": 2, "Y": 2 }, "BuildCost": 20000, "BuildDays": 1,
                "BuildMaterials": [{ "ItemId": "(O)390", "Amount": 200 }, { "ItemId": "(O)Dev_Berry", "Amount": 10 }],
                "CustomFields": { "Port.HarvestHut": "true" }
            },
            "Dev_Stable": {
                "Name": "Stable", "Description": "Comes with a horse to ride around the valley.", "Texture": "Buildings\\Dev_Stable",
                "Size": { "X": 4, "Y": 2 }, "BuildCost": 3000, "BuildDays": 1,
                "BuildMaterials": [{ "ItemId": "(O)388", "Amount": 100 }],
                "CustomFields": { "Port.Stable": "true" }
            },
            "Dev_Obelisk": {
                "Name": "Dune Obelisk", "Description": "Touch it to travel to the dunes.", "Texture": "Buildings\\Dev_Obelisk",
                "Size": { "X": 2, "Y": 3 }, "BuildCost": 5000, "BuildDays": 2,
                "BuildMaterials": [{ "ItemId": "(O)390", "Amount": 200 }, { "ItemId": "(O)Dev_GoldBar", "Amount": 2 }],
                "DefaultAction": "ObeliskWarp Dev_Dunes 10 14 true"
            },
            "Dev_Pond": {
                "Name": "Fish Pond", "Description": "Raise fish of one kind. They multiply and share what they find.", "Texture": "Buildings\\Dev_Pond",
                "Size": { "X": 3, "Y": 3 }, "BuildCost": 1500, "BuildDays": 1,
                "BuildMaterials": [{ "ItemId": "(O)390", "Amount": 100 }],
                "CustomFields": { "Port.FishPond": "true" }
            },
            "Coop": {
                "Name": "Henhouse", "Description": "Houses up to four hens.", "Texture": "Buildings\\Dev_Coop",
                "Size": { "X": 4, "Y": 3 }, "BuildCost": 1000, "BuildDays": 1,
                "BuildMaterials": [{ "ItemId": "(O)388", "Amount": 100 }, { "ItemId": "(O)390", "Amount": 50 }],
                "HumanDoor": { "X": 2, "Y": 2 }, "AnimalDoor": { "X": 0, "Y": 2, "Width": 1, "Height": 1 },
                "IndoorMap": "Coop", "MaxOccupants": 4, "ValidOccupantTypes": ["Coop"]
            },
            "Silo": {
                "Name": "Silo", "Description": "Stores hay cut from grass with the scythe.", "Texture": "Buildings\\Dev_Silo",
                "Size": { "X": 2, "Y": 2 }, "BuildCost": 100, "BuildDays": 1,
                "BuildMaterials": [{ "ItemId": "(O)390", "Amount": 100 }, { "ItemId": "(O)Dev_CopperBar", "Amount": 5 }]
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
        ("Data/GarbageCans", json!({
            "DefaultChanceToFindItem": 0.25,
            "BeforeAll": [],
            "AfterAll": [{ "ItemId": "Dev_Twig", "MinStack": 1, "MaxStack": 2 }],
            "GarbageCans": {
                "Dev_Store": { "BaseChance": 0.5, "Items": [
                    { "ItemId": "Dev_Bread", "MinStack": 1, "MaxStack": 1 },
                    { "ItemId": "Dev_Berry", "MinStack": 1, "MaxStack": 3 }
                ]},
                "Dev_House": { "Items": [
                    { "ItemId": "Dev_Driftwood", "MinStack": 1, "MaxStack": 1 },
                    { "ItemId": "Dev_Geode", "MinStack": 1, "MaxStack": 1 }
                ]}
            }
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
                           "CritChance": 0.02, "CritMultiplier": 3.0, "Texture": "TileSheets\\weapons", "SpriteIndex": 0 },
            "Dev_Dagger": { "Name": "Paring Knife", "DisplayName": "Paring Knife",
                            "Description": "Short and quick. Hold still while it does the work.",
                            "Type": 1, "MinDamage": 2, "MaxDamage": 5, "Knockback": 0.5, "Speed": 2,
                            "CritChance": 0.04, "CritMultiplier": 3.0, "Texture": "TileSheets\\weapons", "SpriteIndex": 1 },
            "Dev_Club": { "Name": "Fence Post", "DisplayName": "Fence Post",
                          "Description": "Heavy, slow and very convincing.",
                          "Type": 2, "MinDamage": 8, "MaxDamage": 14, "Knockback": 1.5, "Speed": -4,
                          "CritChance": 0.02, "CritMultiplier": 2.0, "Texture": "TileSheets\\weapons", "SpriteIndex": 2 },
            "Dev_Slingshot": { "Name": "Forked Stick", "DisplayName": "Forked Stick",
                               "Description": "Fires stones and ore. Hold to pull back, release to shoot.",
                               "Type": 4, "MinDamage": 1, "MaxDamage": 3, "Knockback": 0.5, "Speed": 0,
                               "CritChance": 0.02, "CritMultiplier": 2.0, "Texture": "TileSheets\\weapons", "SpriteIndex": 3 }
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
