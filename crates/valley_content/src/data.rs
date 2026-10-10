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
    pub fish: Vec<SpawnFishData>,
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

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct SpawnFishData {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub season: Option<String>,
    pub chance: Option<f64>,
    pub condition: Option<String>,
    pub fish_area_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct WeaponData {
    pub name: String,
    pub display_name: String,
    pub description: String,
    /// 0 stabbing sword, 1 dagger, 2 club/hammer, 3 slashing sword.
    #[serde(rename = "Type")]
    pub kind: i32,
    pub min_damage: i32,
    pub max_damage: i32,
    pub knockback: f32,
    pub speed: i32,
    pub precision: i32,
    pub defense: i32,
    pub area_of_effect: i32,
    pub crit_chance: f32,
    pub crit_multiplier: f32,
    pub texture: String,
    pub sprite_index: i32,
}

impl Default for WeaponData {
    fn default() -> Self {
        Self {
            name: String::new(),
            display_name: String::new(),
            description: String::new(),
            kind: 3,
            min_damage: 1,
            max_damage: 3,
            knockback: 1.0,
            speed: 0,
            precision: 0,
            defense: 0,
            area_of_effect: 0,
            crit_chance: 0.02,
            crit_multiplier: 3.0,
            texture: "TileSheets\\weapons".into(),
            sprite_index: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuildingMaterial {
    pub id: Option<String>,
    pub item_id: String,
    pub amount: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuildingData {
    pub name: String,
    pub description: String,
    pub texture: String,
    pub size: Point,
    pub source_rect: Option<Rect>,
    pub build_cost: i32,
    pub build_materials: Vec<BuildingMaterial>,
    pub build_days: i32,
    pub builder: Option<String>,
    pub human_door: Point,
    pub animal_door: Option<Rect>,
    pub indoor_map: Option<String>,
    pub max_occupants: i32,
    pub valid_occupant_types: Vec<String>,
    pub build_condition: Option<String>,
}

impl Default for BuildingData {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            texture: String::new(),
            size: Point { x: 1, y: 1 },
            source_rect: None,
            build_cost: 0,
            build_materials: vec![],
            build_days: 0,
            builder: Some("Robin".into()),
            human_door: Point { x: -1, y: -1 },
            animal_door: None,
            indoor_map: None,
            max_occupants: 0,
            valid_occupant_types: vec![],
            build_condition: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct AnimalProduce {
    pub id: Option<String>,
    pub item_id: String,
    pub minimum_friendship: i32,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct FarmAnimalData {
    pub display_name: String,
    pub house: String,
    pub purchase_price: i32,
    pub sell_price: i32,
    pub texture: String,
    pub baby_texture: Option<String>,
    pub sprite_width: i32,
    pub sprite_height: i32,
    pub days_to_mature: i32,
    pub days_to_produce: i32,
    pub produce_item_ids: Vec<AnimalProduce>,
    pub deluxe_produce_item_ids: Vec<AnimalProduce>,
    pub harvest_type: String,
    pub harvest_tool: Option<String>,
    pub sound: Option<String>,
    pub can_get_pregnant: bool,
}

impl Default for FarmAnimalData {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            house: "Coop".into(),
            purchase_price: -1,
            sell_price: 0,
            texture: String::new(),
            baby_texture: None,
            sprite_width: 16,
            sprite_height: 16,
            days_to_mature: 1,
            days_to_produce: 1,
            produce_item_ids: vec![],
            deluxe_produce_item_ids: vec![],
            harvest_type: "DropOvernight".into(),
            harvest_tool: None,
            sound: None,
            can_get_pregnant: false,
        }
    }
}
