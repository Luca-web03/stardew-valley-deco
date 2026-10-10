//! Typed views over the game's data assets (`Data/*.json` as unpacked).
//!
//! Field names follow the public data format documented for content packs.
//! Every field has a default so partial or modded data still loads.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

fn neg300() -> i32 {
    -300
}
fn neg1() -> i32 {
    -1
}
fn one() -> i32 {
    1
}
fn default_forage_max() -> i32 {
    6
}
fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ObjectData {
    pub name: String,
    pub display_name: String,
    pub description: String,
    #[serde(rename = "Type")]
    pub kind: String,
    pub category: i32,
    pub price: i32,
    pub texture: Option<String>,
    pub sprite_index: i32,
    #[serde(default = "neg300")]
    pub edibility: i32,
    pub is_drink: bool,
    #[serde(default = "yes")]
    pub can_be_given_as_gift: bool,
    #[serde(default = "yes")]
    pub can_be_trashed: bool,
    pub context_tags: Option<Vec<String>>,
    pub buffs: Option<serde_json::Value>,
    pub custom_fields: Option<BTreeMap<String, String>>,
}

impl Default for ObjectData {
    fn default() -> Self {
        Self {
            name: String::new(),
            display_name: String::new(),
            description: String::new(),
            kind: "Basic".into(),
            category: 0,
            price: 0,
            texture: None,
            sprite_index: 0,
            edibility: -300,
            is_drink: false,
            can_be_given_as_gift: true,
            can_be_trashed: true,
            context_tags: None,
            buffs: None,
            custom_fields: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct CropData {
    pub seasons: Vec<String>,
    pub days_in_phase: Vec<i32>,
    #[serde(default = "neg1")]
    pub regrow_days: i32,
    pub is_raised: bool,
    pub is_paddy_crop: bool,
    #[serde(default = "yes")]
    pub needs_watering: bool,
    pub harvest_item_id: String,
    #[serde(default = "one")]
    pub harvest_min_stack: i32,
    #[serde(default = "one")]
    pub harvest_max_stack: i32,
    pub harvest_min_quality: i32,
    pub harvest_max_quality: Option<i32>,
    pub harvest_max_increase_per_farming_level: f32,
    pub extra_harvest_chance: f64,
    pub harvest_method: String,
    pub texture: String,
    pub sprite_index: i32,
    pub tint_colors: Vec<String>,
}

impl Default for CropData {
    fn default() -> Self {
        Self {
            seasons: vec![],
            days_in_phase: vec![],
            regrow_days: -1,
            is_raised: false,
            is_paddy_crop: false,
            needs_watering: true,
            harvest_item_id: String::new(),
            harvest_min_stack: 1,
            harvest_max_stack: 1,
            harvest_min_quality: 0,
            harvest_max_quality: None,
            harvest_max_increase_per_farming_level: 0.0,
            extra_harvest_chance: 0.0,
            harvest_method: "Grab".into(),
            texture: "TileSheets\\crops".into(),
            sprite_index: 0,
            tint_colors: vec![],
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct CharacterHome {
    pub id: String,
    pub condition: Option<String>,
    pub location: String,
    pub tile: Point,
    pub direction: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct CharacterData {
    pub display_name: String,
    pub birth_season: Option<String>,
    pub birth_day: i32,
    pub home_region: String,
    pub gender: String,
    pub age: String,
    pub can_be_romanced: bool,
    pub can_socialize: Option<String>,
    pub can_receive_gifts: bool,
    pub home: Vec<CharacterHome>,
    pub size: Point,
    pub texture_name: Option<String>,
    pub spouse_room: Option<serde_json::Value>,
}

impl Default for CharacterData {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            birth_season: None,
            birth_day: 0,
            home_region: "Other".into(),
            gender: "Undefined".into(),
            age: "Adult".into(),
            can_be_romanced: false,
            can_socialize: None,
            can_receive_gifts: true,
            home: vec![],
            size: Point { x: 16, y: 32 },
            texture_name: None,
            spouse_room: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ShopItemData {
    pub id: String,
    pub item_id: String,
    #[serde(default = "neg1")]
    pub price: i32,
    pub condition: Option<String>,
    #[serde(default = "neg1")]
    pub available_stock: i32,
    pub min_stack: i32,
    pub max_stack: i32,
    pub is_recipe: bool,
}

impl Default for ShopItemData {
    fn default() -> Self {
        Self {
            id: String::new(),
            item_id: String::new(),
            price: -1,
            condition: None,
            available_stock: -1,
            min_stack: -1,
            max_stack: -1,
            is_recipe: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ShopOwnerData {
    pub name: String,
    pub id: Option<String>,
    pub condition: Option<String>,
    pub dialogues: Option<serde_json::Value>,
    pub portrait: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ShopData {
    pub currency: i32,
    pub owners: Vec<ShopOwnerData>,
    pub items: Vec<ShopItemData>,
    pub sale_price_modifiers: Option<serde_json::Value>,
    pub price_modifiers: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct CreateLocationData {
    pub map_path: String,
    #[serde(rename = "Type")]
    pub kind: Option<String>,
    pub always_active: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct LocationData {
    pub display_name: Option<String>,
    pub default_arrival_tile: Option<Point>,
    pub exclude_from_npc_pathfinding: bool,
    pub create_on_load: Option<CreateLocationData>,
    pub music: Option<serde_json::Value>,
    pub music_default: Option<String>,
    pub forage: Vec<SpawnForageData>,
    #[serde(default = "default_forage_max")]
    pub max_spawned_forage_at_once: i32,
    pub min_daily_forage_spawn: i32,
    pub max_daily_forage_spawn: i32,
    pub fish: Option<serde_json::Value>,
    pub artifact_spots: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ToolData {
    pub class_name: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub texture: String,
    pub sprite_index: i32,
    pub menu_sprite_index: i32,
    pub upgrade_level: i32,
    pub sale_price: i32,
}

/// Data/Characters.json etc. are dictionaries keyed by ID.
pub type DataDict<T> = BTreeMap<String, T>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct BigCraftableData {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub price: i32,
    pub fragility: i32,
    pub can_be_placed_outdoors: bool,
    pub can_be_placed_indoors: bool,
    pub is_lamp: bool,
    pub texture: Option<String>,
    pub sprite_index: i32,
    pub context_tags: Option<Vec<String>>,
}

impl Default for BigCraftableData {
    fn default() -> Self {
        Self {
            name: String::new(),
            display_name: String::new(),
            description: String::new(),
            price: 0,
            fragility: 0,
            can_be_placed_outdoors: true,
            can_be_placed_indoors: true,
            is_lamp: false,
            texture: None,
            sprite_index: 0,
            context_tags: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct MachineTrigger {
    pub id: Option<String>,
    pub trigger: String,
    pub required_item_id: Option<String>,
    pub required_tags: Option<Vec<String>>,
    #[serde(default = "one")]
    pub required_count: i32,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct QuantityModifier {
    pub id: Option<String>,
    pub condition: Option<String>,
    pub modification: String,
    pub amount: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct MachineOutputItem {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub condition: Option<String>,
    pub min_stack: i32,
    pub max_stack: i32,
    pub quality: Option<i32>,
    pub preserve_type: Option<String>,
    pub preserve_id: Option<String>,
    pub copy_price: bool,
    pub copy_quality: bool,
    pub object_internal_name: Option<String>,
    pub object_display_name: Option<String>,
    pub price_modifiers: Option<Vec<QuantityModifier>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct MachineOutputRule {
    pub id: String,
    pub triggers: Vec<MachineTrigger>,
    pub output_item: Vec<MachineOutputItem>,
    #[serde(default = "neg1")]
    pub minutes_until_ready: i32,
    #[serde(default = "neg1")]
    pub days_until_ready: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct MachineItemAdditionalConsumed {
    pub item_id: String,
    #[serde(default = "one")]
    pub required_count: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct MachineData {
    pub output_rules: Vec<MachineOutputRule>,
    pub additional_consumed_items: Vec<MachineItemAdditionalConsumed>,
    pub invalid_item_message: Option<String>,
    pub load_effects: Option<serde_json::Value>,
    pub working_effects: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct SpawnForageData {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub season: Option<String>,
    pub chance: Option<f64>,
    pub condition: Option<String>,
}
