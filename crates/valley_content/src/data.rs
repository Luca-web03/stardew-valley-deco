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
    /// What cracking this geode can give.
    pub geode_drops: Vec<GeodeDrop>,
    /// Fall back to the default geode pool when no drop applies.
    #[serde(default = "yes")]
    pub geode_drops_default_items: bool,
    /// Location -> chance to dig this up from an artifact spot there.
    pub artifact_spot_chances: Option<BTreeMap<String, f64>>,
    pub exclude_from_random_sale: bool,
    pub exclude_from_shipping_collection: bool,
    pub exclude_from_fishing_collection: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GeodeDrop {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub random_item_id: Option<Vec<String>>,
    pub condition: Option<String>,
    #[serde(default = "one_f")]
    pub chance: f64,
    pub precedence: i32,
    pub min_stack: i32,
    pub max_stack: i32,
    pub set_flag_on_pickup: Option<String>,
}

fn one_f() -> f64 {
    1.0
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
            geode_drops: Vec::new(),
            geode_drops_default_items: true,
            artifact_spot_chances: None,
            exclude_from_random_sale: false,
            exclude_from_shipping_collection: false,
            exclude_from_fishing_collection: false,
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
    /// Outfits by season/indoors/condition (sprite and portrait assets).
    pub appearance: Vec<CharacterAppearance>,
    /// The NPC only exists while this query passes.
    pub unlock_conditions: Option<String>,
    pub flower_dance_can_dance: Option<bool>,
    pub winter_star_participant: Option<String>,
    /// `UnknownUntilMet` (default), `AlwaysShown`, `HiddenUntilMet`, `HiddenAlways`.
    pub social_tab: Option<String>,
    /// `AlwaysShown` (default), `HiddenUntilMet`, `HiddenAlways`.
    pub calendar: Option<String>,
    pub dumpster_dive_emote: Option<i32>,
    pub dumpster_dive_friendship_effect: Option<i32>,
    pub item_delivery_quests: Option<String>,
    pub perfection_score: Option<bool>,
    pub spouse_gift_jealousy: Option<String>,
    pub spouse_gift_jealousy_friendship_change: Option<i32>,
    pub spouse_wants_children: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct CharacterAppearance {
    pub id: String,
    pub season: Option<String>,
    pub indoors: bool,
    pub outdoors: bool,
    pub condition: Option<String>,
    pub portrait: Option<String>,
    pub sprite: Option<String>,
    pub is_island_attire: bool,
    pub precedence: i32,
    pub weight: i32,
}

impl Default for CharacterAppearance {
    fn default() -> Self {
        Self { id: String::new(), season: None, indoors: true, outdoors: true, condition: None, portrait: None, sprite: None, is_island_attire: false, precedence: 0, weight: 1 }
    }
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
            appearance: Vec::new(),
            unlock_conditions: None,
            flower_dance_can_dance: None,
            winter_star_participant: None,
            social_tab: None,
            calendar: None,
            dumpster_dive_emote: None,
            dumpster_dive_friendship_effect: None,
            item_delivery_quests: None,
            perfection_score: None,
            spouse_gift_jealousy: None,
            spouse_gift_jealousy_friendship_change: None,
            spouse_wants_children: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ShopItemData {
    pub id: String,
    pub item_id: String,
    /// Pick one of these at random instead of `item_id`.
    pub random_item_id: Option<Vec<String>>,
    /// Cap on how many items an item query adds.
    pub max_items: Option<i32>,
    /// Pay with items instead of (or as well as) currency.
    pub trade_item_id: Option<String>,
    pub trade_item_amount: Option<i32>,
    #[serde(default = "neg1")]
    pub price: i32,
    pub condition: Option<String>,
    #[serde(default = "neg1")]
    pub available_stock: i32,
    pub min_stack: i32,
    pub max_stack: i32,
    pub is_recipe: bool,
    pub quality: Option<i32>,
    pub price_modifiers: Option<Vec<QuantityModifier>>,
    pub ignore_shop_price_modifiers: bool,
    /// Run when bought (trigger action strings).
    pub actions_on_purchase: Option<Vec<String>>,
    /// Skip items already listed by earlier entries.
    pub avoid_repeat: bool,
    /// Checked per item an item query produces (`Target` = the item).
    pub per_item_condition: Option<String>,
}

impl Default for ShopItemData {
    fn default() -> Self {
        Self {
            id: String::new(),
            item_id: String::new(),
            random_item_id: None,
            max_items: None,
            trade_item_id: None,
            trade_item_amount: None,
            price: -1,
            condition: None,
            available_stock: -1,
            min_stack: -1,
            max_stack: -1,
            is_recipe: false,
            quality: None,
            price_modifiers: None,
            ignore_shop_price_modifiers: false,
            actions_on_purchase: None,
            avoid_repeat: false,
            per_item_condition: None,
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
    pub sale_price_modifiers: Option<Vec<QuantityModifier>>,
    pub price_modifiers: Option<Vec<QuantityModifier>>,
    /// Items this shop buys from the player (context tags, any match).
    pub salable_item_tags: Vec<String>,
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
    pub fish_areas: BTreeMap<String, FishAreaData>,
    pub artifact_spots: Vec<ArtifactSpotDrop>,
    /// Crops and trees can be planted here (default: farm and greenhouse).
    pub can_plant_here: Option<bool>,
    #[serde(default = "default_min_weeds")]
    pub min_daily_weeds: i32,
    #[serde(default = "default_max_weeds")]
    pub max_daily_weeds: i32,
    #[serde(default = "default_clay")]
    pub chance_for_clay: f64,
}

fn default_min_weeds() -> i32 {
    2
}

fn default_max_weeds() -> i32 {
    5
}

fn default_clay() -> f64 {
    0.03
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
    pub custom_fields: Option<BTreeMap<String, String>>,
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
            custom_fields: None,
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
    pub stack_modifiers: Option<Vec<QuantityModifier>>,
    pub quality_modifiers: Option<Vec<QuantityModifier>>,
    /// Code-defined output ("<type>, <assembly>: <method>").
    pub output_method: Option<String>,
    pub random_item_id: Option<Vec<String>>,
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
    pub invalid_count_message: Option<String>,
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
    pub invalid_count_message: Option<String>,
    pub load_effects: Option<serde_json::Value>,
    pub working_effects: Option<serde_json::Value>,
    /// "farming 5 foraging 2": experience when collecting the output.
    pub experience_gain_on_harvest: Option<String>,
    pub ready_time_modifiers: Option<Vec<QuantityModifier>>,
    /// Conditions that pause the machine ("Outside", "Inside", seasons,
    /// "Sun", "Rain", "Always").
    pub prevent_time_pass: Option<Vec<String>>,
    /// Output only becomes ready at the start of a day.
    pub only_complete_overnight: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct SpawnForageData {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub random_item_id: Option<Vec<String>>,
    pub season: Option<String>,
    pub chance: Option<f64>,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct SpawnFishData {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub random_item_id: Option<Vec<String>>,
    pub season: Option<String>,
    pub chance: Option<f64>,
    pub condition: Option<String>,
    pub fish_area_id: Option<String>,
    pub bobber_position: Option<Rect>,
    pub player_position: Option<Rect>,
    pub min_fishing_level: i32,
    pub min_distance_from_shore: i32,
    /// -1: no limit.
    #[serde(default = "neg1")]
    pub max_distance_from_shore: i32,
    pub apply_daily_luck: bool,
    pub chance_boost_per_luck_level: f64,
    /// Times this can be caught (-1: no limit).
    #[serde(default = "neg1")]
    pub catch_limit: i32,
    pub is_boss_fish: bool,
    pub set_flag_on_catch: Option<String>,
    pub require_magic_bait: bool,
    pub precedence: i32,
    pub ignore_fish_data_requirements: bool,
    #[serde(default = "yes")]
    pub can_be_inherited: bool,
}

/// A named fishing area of a location (`FishAreas`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct FishAreaData {
    pub display_name: Option<String>,
    pub position: Option<Rect>,
    pub crab_pot_fish_types: Vec<String>,
    pub crab_pot_junk_chance: Option<f64>,
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
    /// Tile action run when any non-door tile of the building is used
    /// (e.g. "ObeliskWarp Beach 20 4 true").
    pub default_action: Option<String>,
    pub custom_fields: Option<BTreeMap<String, String>>,
    /// Building this one upgrades (e.g. a bigger coop).
    pub building_to_upgrade: Option<String>,
    /// Hay this building stores (silos).
    pub hay_capacity: i32,
    /// Overnight conversions between the building's chests (e.g. a mill).
    pub item_conversions: Vec<BuildingItemConversion>,
    /// Item stores of the building.
    pub chests: Vec<BuildingChest>,
    /// Actions on tiles of the building (relative to its top-left).
    pub action_tiles: Vec<BuildingActionTile>,
    /// Items placed inside when built.
    pub indoor_items: Vec<IndoorItemAdd>,
    /// Rows of 'X' (blocked) and 'O' (open) over the footprint.
    pub collision_map: Option<String>,
    /// Built at once, without a builder's days.
    pub magical_construction: bool,
    /// Alternative looks (`Skins`).
    pub skins: Vec<BuildingSkin>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuildingSkin {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub texture: String,
    pub condition: Option<String>,
    pub build_days: Option<i32>,
    pub build_cost: Option<i32>,
    pub build_materials: Option<Vec<BuildingMaterial>>,
    pub show_as_separate_construction_entry: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuildingItemConversion {
    pub id: String,
    pub required_tags: Vec<String>,
    #[serde(default = "one")]
    pub required_count: i32,
    #[serde(default = "neg1")]
    pub max_daily_conversions: i32,
    pub source_chest: String,
    pub destination_chest: String,
    pub produced_items: Vec<GenericItem>,
}

/// A generic spawned item (subset of the original's spawn item fields).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GenericItem {
    pub id: Option<String>,
    pub item_id: Option<String>,
    pub random_item_id: Option<Vec<String>>,
    pub condition: Option<String>,
    pub min_stack: i32,
    pub max_stack: i32,
    pub quality: Option<i32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuildingChest {
    pub id: String,
    /// "Chest", "Collect" or "Load".
    #[serde(rename = "Type")]
    pub kind: String,
    pub sound: Option<String>,
    pub invalid_item_message: Option<String>,
    pub invalid_count_message: Option<String>,
    pub chest_full_message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuildingActionTile {
    pub id: Option<String>,
    pub tile: Point,
    pub action: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct IndoorItemAdd {
    pub id: Option<String>,
    pub item_id: String,
    pub tile: Point,
    pub indestructible: bool,
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
            default_action: None,
            custom_fields: None,
            building_to_upgrade: None,
            hay_capacity: 0,
            item_conversions: Vec::new(),
            chests: Vec::new(),
            action_tiles: Vec::new(),
            indoor_items: Vec::new(),
            collision_map: None,
            magical_construction: false,
            skins: Vec::new(),
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
    /// Texture after the produce has been harvested (e.g. shorn).
    pub harvested_texture: Option<String>,
    /// Buying this animal may give one of these instead.
    pub alternate_purchase_types: Vec<AlternatePurchaseAnimals>,
    pub deluxe_produce_minimum_friendship: i32,
    pub deluxe_produce_care_divisor: f64,
    pub deluxe_produce_luck_multiplier: f64,
    /// Profession ids (names or original numbers).
    pub profession_for_faster_produce: Option<String>,
    pub profession_for_quality_boost: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct AlternatePurchaseAnimals {
    pub id: Option<String>,
    pub condition: Option<String>,
    pub animal_ids: Vec<String>,
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
            harvested_texture: None,
            alternate_purchase_types: Vec::new(),
            deluxe_produce_minimum_friendship: 200,
            deluxe_produce_care_divisor: 1200.0,
            deluxe_produce_luck_multiplier: 0.0,
            profession_for_faster_produce: None,
            profession_for_quality_boost: None,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct FruitTreeFruit {
    pub id: Option<String>,
    pub item_id: String,
    pub season: Option<String>,
    pub chance: Option<f64>,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct FruitTreeData {
    pub display_name: String,
    pub seasons: Vec<String>,
    pub fruit: Vec<FruitTreeFruit>,
    pub texture: Option<String>,
    pub texture_sprite_row: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct TapItem {
    pub id: Option<String>,
    pub item_id: String,
    pub days_until_ready: i32,
    pub min_stack: i32,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct WildTreeData {
    pub seed_item_id: Option<String>,
    pub seed_plantable: bool,
    pub growth_chance: f32,
    pub tap_items: Vec<TapItem>,
}

impl Default for WildTreeData {
    fn default() -> Self {
        Self { seed_item_id: None, seed_plantable: true, growth_chance: 0.2, tap_items: vec![] }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct ArtifactSpotDrop {
    pub id: Option<String>,
    pub item_id: String,
    pub random_item_id: Option<Vec<String>>,
    pub chance: Option<f64>,
    pub condition: Option<String>,
    pub min_stack: i32,
    pub max_stack: i32,
    pub precedence: i32,
    /// Keep rolling the next drops after this one drops.
    pub continue_on_drop: bool,
    #[serde(default = "yes")]
    pub apply_generous_enchantment: bool,
}

/// One drop from a giant crop (`Data/GiantCrops` `HarvestItems`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GiantCropHarvestItem {
    pub id: Option<String>,
    pub item_id: String,
    pub chance: f64,
    pub condition: Option<String>,
    pub min_stack: i32,
    pub max_stack: i32,
}

impl Default for GiantCropHarvestItem {
    fn default() -> Self {
        Self { id: None, item_id: String::new(), chance: 1.0, condition: None, min_stack: 1, max_stack: -1 }
    }
}

/// `Data/GiantCrops`: crops that can merge into one giant crop.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase", default)]
pub struct GiantCropData {
    pub from_item_id: String,
    pub harvest_items: Vec<GiantCropHarvestItem>,
    pub texture: String,
    pub texture_position: Point,
    pub tile_size: Point,
    pub health: i32,
    pub chance: f64,
    pub condition: Option<String>,
    pub custom_fields: Option<BTreeMap<String, String>>,
}

impl Default for GiantCropData {
    fn default() -> Self {
        Self {
            from_item_id: String::new(),
            harvest_items: vec![],
            texture: String::new(),
            texture_position: Point::default(),
            tile_size: Point { x: 3, y: 3 },
            health: 3,
            chance: 0.01,
            condition: None,
            custom_fields: None,
        }
    }
}
