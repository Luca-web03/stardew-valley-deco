//! Items and the item registry built from content data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use valley_content::data::{BigCraftableData, CropData, MachineData, ObjectData, ToolData, WeaponData};
use valley_content::ContentManager;

pub const MAX_STACK: i32 = 999;

/// Object categories (public data constants).
pub mod category {
    pub const SEEDS: i32 = -74;
    pub const VEGETABLE: i32 = -75;
    pub const FRUIT: i32 = -79;
    pub const FLOWER: i32 = -80;
    pub const FORAGE: i32 = -81;
    pub const RESOURCE: i32 = -16;
    pub const COOKING: i32 = -7;
    pub const FISH: i32 = -4;
    pub const FERTILIZER: i32 = -19;
    /// Crafted placeables (fences, sprinklers, paths...).
    pub const CRAFTING_PLACEABLE: i32 = -8;
    pub const BIG_CRAFTABLE: i32 = -9;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default, PartialOrd, Ord)]
pub enum Quality {
    #[default]
    Normal = 0,
    Silver = 1,
    Gold = 2,
    Iridium = 4,
}

impl Quality {
    /// Sell-price multiplier.
    pub fn price_mult(self) -> f32 {
        match self {
            Quality::Normal => 1.0,
            Quality::Silver => 1.25,
            Quality::Gold => 1.5,
            Quality::Iridium => 2.0,
        }
    }
    /// Edibility multiplier for energy/health restored.
    pub fn edible_mult(self) -> f32 {
        match self {
            Quality::Normal => 1.0,
            Quality::Silver => 1.4,
            Quality::Gold => 1.8,
            Quality::Iridium => 2.6,
        }
    }
    pub fn from_level(q: i32) -> Quality {
        match q {
            i32::MIN..=0 => Quality::Normal,
            1 => Quality::Silver,
            2 | 3 => Quality::Gold,
            _ => Quality::Iridium,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum ToolKind {
    Hoe,
    WateringCan,
    Axe,
    Pickaxe,
    Scythe,
    FishingRod,
    MilkPail,
    Shears,
}

impl ToolKind {
    pub fn data_key(self) -> &'static str {
        match self {
            ToolKind::Hoe => "Hoe",
            ToolKind::WateringCan => "WateringCan",
            ToolKind::Axe => "Axe",
            ToolKind::Pickaxe => "Pickaxe",
            ToolKind::Scythe => "Scythe",
            ToolKind::FishingRod => "FishingRod",
            ToolKind::MilkPail => "MilkPail",
            ToolKind::Shears => "Shears",
        }
    }
    pub fn from_key(s: &str) -> Option<ToolKind> {
        Some(match s {
            "Hoe" => ToolKind::Hoe,
            "WateringCan" => ToolKind::WateringCan,
            "Axe" => ToolKind::Axe,
            "Pickaxe" => ToolKind::Pickaxe,
            "Scythe" => ToolKind::Scythe,
            "FishingRod" => ToolKind::FishingRod,
            "MilkPail" | "Milk Pail" => ToolKind::MilkPail,
            "Shears" => ToolKind::Shears,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ToolKind,
    /// 0 basic, 1 copper, 2 steel, 3 gold, 4 iridium.
    pub level: i32,
    /// Watering can only.
    pub water: i32,
    /// Fishing rods: bait (slot 0) and tackle (slot 1).
    #[serde(default)]
    pub attachments: Vec<Option<Item>>,
    /// Uses left on the attached tackle.
    #[serde(default)]
    pub tackle_uses: i32,
    /// Forge enchantment ("Swift", "Bottomless", ...; see `forge`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enchantment: Option<String>,
}

impl Tool {
    pub fn new(kind: ToolKind) -> Self {
        let mut t = Self { kind, level: 0, water: 0, attachments: Vec::new(), tackle_uses: 0, enchantment: None };
        t.water = t.water_capacity();
        t
    }
    /// Basic can holds 40; each upgrade adds capacity (documented values).
    pub fn water_capacity(&self) -> i32 {
        match self.level {
            0 => 40,
            1 => 55,
            2 => 70,
            3 => 85,
            _ => 100,
        }
    }
}

/// Extra state for produced items (machine outputs like "<fruit> Juice").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ItemExtra {
    /// Item this was made from (for flavored names/prices).
    pub preserved_from: Option<String>,
    pub display_name: Option<String>,
    pub price: Option<i32>,
    /// Rings joined into this combined ring (ring object ids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combined: Vec<String>,
}

/// Forge state on a weapon (see `forge`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponMods {
    /// Gem forges in order ("Ruby", "Jade", ...), at most three.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub gems: Vec<String>,
    /// Enchantment from a prismatic shard ("Vampiric", ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enchantment: Option<String>,
    /// Innate enchantments from dragon teeth, with levels.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub innate: BTreeMap<String, i32>,
    /// Weapon id whose look this one wears.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<String>,
    /// Galaxy souls added so far.
    #[serde(skip_serializing_if = "is_zero")]
    pub souls: i32,
}

fn is_zero(v: &i32) -> bool {
    *v == 0
}

impl WeaponMods {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Object {
        id: String,
        stack: i32,
        quality: Quality,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<Box<ItemExtra>>,
    },
    /// Placeable "big craftable" (chests, machines...), 16x32 sprite.
    Big { id: String, stack: i32 },
    Tool(Tool),
    /// Melee weapon from Data/Weapons.
    Weapon {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mods: Option<Box<WeaponMods>>,
    },
    /// Footwear from Data/Boots.
    Boots { id: String },
    /// Furniture from Data/Furniture.
    Furniture { id: String },
    /// Hat, shirt or pants (Data/Hats, Data/Shirts, Data/Pants).
    Clothing { slot: ClothingSlot, id: String },
    /// Trinket from Data/Trinkets.
    Trinket { id: String },
}

/// A trinket (`Data/Trinkets`, public format subset). Effects are game code
/// in the original; here `Port.TrinketStats` ("attack 2 defense 1 speed 1
/// magnet 32 luck 1") gives stat bonuses.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct TrinketData {
    pub display_name: String,
    pub description: String,
    pub texture: Option<String>,
    pub sheet_index: i32,
    pub custom_fields: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClothingSlot {
    Hat,
    Shirt,
    Pants,
}

impl ClothingSlot {
    pub fn prefix(self) -> &'static str {
        match self {
            ClothingSlot::Hat => "H",
            ClothingSlot::Shirt => "S",
            ClothingSlot::Pants => "P",
        }
    }
}

/// A wearable from Data/Hats (slash format) or Data/Shirts / Data/Pants
/// (JSON, public format subset).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClothingInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub price: i32,
    pub texture: Option<String>,
    pub sprite_index: i32,
    /// Colour the clothing gives the farmer, if any.
    pub color: Option<[u8; 3]>,
    /// Hats: hide the hair underneath.
    pub hides_hair: bool,
}

/// `Data/Hats`: "name/description/showRealHair/skipHairstyleOffset/tags/displayName/spriteIndex/texture".
pub fn parse_hat(id: &str, s: &str) -> Option<ClothingInfo> {
    let f: Vec<&str> = s.split('/').collect();
    let name = f.first()?.to_string();
    Some(ClothingInfo {
        display_name: f.get(5).filter(|s| !s.is_empty()).map(|s| s.to_string()).unwrap_or_else(|| name.clone()),
        description: f.get(1).unwrap_or(&"").to_string(),
        hides_hair: f.get(2).is_some_and(|v| v.eq_ignore_ascii_case("hide")),
        sprite_index: f.get(6).and_then(|v| v.parse().ok()).unwrap_or_else(|| id.parse().unwrap_or(0)),
        texture: f.get(7).filter(|s| !s.is_empty()).map(|s| s.to_string()),
        name,
        price: 0,
        color: None,
    })
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
struct ClothingJson {
    name: String,
    display_name: String,
    description: String,
    price: i32,
    texture: Option<String>,
    sprite_index: i32,
    default_color: Option<String>,
}

fn parse_rgb(s: &str) -> Option<[u8; 3]> {
    let v: Vec<u8> = s.split_whitespace().filter_map(|p| p.parse().ok()).collect();
    (v.len() >= 3).then(|| [v[0], v[1], v[2]])
}

impl Item {
    pub fn object(id: &str, stack: i32) -> Self {
        Item::Object { id: id.to_string(), stack, quality: Quality::Normal, extra: None }
    }
    pub fn weapon(id: &str) -> Self {
        Item::Weapon { id: id.to_string(), mods: None }
    }
    /// A weapon's forge state (empty for plain weapons).
    pub fn weapon_mods(&self) -> Option<&WeaponMods> {
        match self {
            Item::Weapon { mods, .. } => mods.as_deref(),
            _ => None,
        }
    }
    pub fn object_q(id: &str, stack: i32, quality: Quality) -> Self {
        Item::Object { id: id.to_string(), stack, quality, extra: None }
    }
    pub fn big(id: &str, stack: i32) -> Self {
        Item::Big { id: id.to_string(), stack }
    }

    pub fn stack(&self) -> i32 {
        match self {
            Item::Object { stack, .. } | Item::Big { stack, .. } => *stack,
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } | Item::Furniture { .. } | Item::Clothing { .. } | Item::Trinket { .. } => 1,
        }
    }

    pub fn set_stack(&mut self, n: i32) {
        match self {
            Item::Object { stack, .. } | Item::Big { stack, .. } => *stack = n,
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } | Item::Furniture { .. } | Item::Clothing { .. } | Item::Trinket { .. } => {}
        }
    }

    pub fn quality(&self) -> Quality {
        match self {
            Item::Object { quality, .. } => *quality,
            _ => Quality::Normal,
        }
    }

    /// Change an object's quality (other items have none).
    pub fn set_quality(&mut self, q: Quality) {
        if let Item::Object { quality, .. } = self {
            *quality = q;
        }
    }

    pub fn extra(&self) -> Option<&ItemExtra> {
        match self {
            Item::Object { extra, .. } => extra.as_deref(),
            _ => None,
        }
    }

    pub fn big_id(&self) -> Option<&str> {
        match self {
            Item::Big { id, .. } => Some(id),
            _ => None,
        }
    }

    pub fn object_id(&self) -> Option<&str> {
        match self {
            Item::Object { id, .. } => Some(id),
            _ => None,
        }
    }

    pub fn tool(&self) -> Option<&Tool> {
        match self {
            Item::Tool(t) => Some(t),
            _ => None,
        }
    }

    pub fn tool_mut(&mut self) -> Option<&mut Tool> {
        match self {
            Item::Tool(t) => Some(t),
            _ => None,
        }
    }

    pub fn can_stack_with(&self, o: &Item) -> bool {
        match (self, o) {
            (Item::Object { id: a, quality: qa, extra: ea, .. }, Item::Object { id: b, quality: qb, extra: eb, .. }) => a == b && qa == qb && ea == eb,
            (Item::Big { id: a, .. }, Item::Big { id: b, .. }) => a == b,
            _ => false,
        }
    }

    /// Qualified id like "(O)24", "(BC)130" or "(T)Hoe".
    pub fn qualified_id(&self) -> String {
        match self {
            Item::Object { id, .. } => format!("(O){id}"),
            Item::Big { id, .. } => format!("(BC){id}"),
            Item::Tool(t) => format!("(T){}", t.kind.data_key()),
            Item::Weapon { id, .. } => format!("(W){id}"),
            Item::Boots { id } => format!("(B){id}"),
            Item::Clothing { slot, id } => format!("({}){id}", slot.prefix()),
            Item::Trinket { id } => format!("(TR){id}"),
            Item::Furniture { id } => format!("(F){id}"),
        }
    }

    pub fn weapon_id(&self) -> Option<&str> {
        match self {
            Item::Weapon { id, .. } => Some(id),
            _ => None,
        }
    }

    /// Tools, weapons and boots can't be stacked.
    pub fn is_equipment(&self) -> bool {
        matches!(self, Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } | Item::Furniture { .. } | Item::Clothing { .. } | Item::Trinket { .. })
    }
}

/// Split "(O)24" into ("O", "24"); unqualified ids default to objects.
pub fn split_qualified(id: &str) -> (&str, &str) {
    if let Some(rest) = id.strip_prefix('(') {
        if let Some(end) = rest.find(')') {
            return (&rest[..end], &rest[end + 1..]);
        }
    }
    ("O", id)
}

#[derive(Clone, Debug)]
pub struct ObjectInfo {
    pub id: String,
    pub data: ObjectData,
    pub display_name: String,
    pub description: String,
}

impl ObjectInfo {
    pub fn texture(&self) -> String {
        self.data.texture.clone().unwrap_or_else(|| "Maps/springobjects".to_string()).replace('\\', "/")
    }
    pub fn is_edible(&self) -> bool {
        self.data.edibility != -300
    }
    pub fn energy(&self, q: Quality) -> i32 {
        (self.data.edibility as f32 * 2.5 * q.edible_mult()).floor() as i32
    }
    pub fn health(&self, q: Quality) -> i32 {
        if self.data.edibility < 0 {
            0
        } else {
            (self.data.edibility as f32 * 1.125 * q.edible_mult()).floor() as i32
        }
    }
}

#[derive(Clone, Debug)]
pub struct BigInfo {
    pub id: String,
    pub data: BigCraftableData,
    pub display_name: String,
    pub description: String,
}

impl BigInfo {
    pub fn texture(&self) -> String {
        self.data.texture.clone().unwrap_or_else(|| "TileSheets/Craftables".to_string()).replace('\\', "/")
    }
}

/// Footwear (`Data/Boots`, public slash format:
/// name/description/price/defense/immunity/colorIndex/displayName/...).
#[derive(Clone, Debug, PartialEq)]
pub struct BootsInfo {
    pub name: String,
    pub description: String,
    pub price: i32,
    pub defense: i32,
    pub immunity: i32,
    pub display_name: String,
    pub sprite_index: i32,
    pub texture: Option<String>,
}

pub fn parse_boots(id: &str, s: &str) -> Option<BootsInfo> {
    let f: Vec<&str> = s.split('/').collect();
    let num = |i: usize| f.get(i).and_then(|v| v.trim().parse::<i32>().ok()).unwrap_or(0);
    Some(BootsInfo {
        name: f.first()?.to_string(),
        description: f.get(1).unwrap_or(&"").to_string(),
        price: num(2),
        defense: num(3),
        immunity: num(4),
        display_name: f.get(6).filter(|s| !s.is_empty()).or(f.first()).unwrap_or(&"").to_string(),
        sprite_index: f.get(8).and_then(|v| v.parse().ok()).unwrap_or_else(|| id.parse().unwrap_or(0)),
        texture: f.get(9).filter(|s| !s.is_empty()).map(|s| s.to_string()),
    })
}

/// Furniture (`Data/Furniture`, public slash format:
/// name/type/tilesheet size/bounding box size/rotations/price/placement/display name/sprite index/texture/...).
#[derive(Clone, Debug, PartialEq)]
pub struct FurnitureInfo {
    pub name: String,
    pub kind: String,
    /// Sprite size in tiles (w, h); "-1" means the type's default.
    pub sheet_size: (i32, i32),
    /// Footprint in tiles.
    pub footprint: (i32, i32),
    pub price: i32,
    pub display_name: String,
    pub sprite_index: i32,
    pub texture: Option<String>,
}

impl FurnitureInfo {
    /// Rugs and window decorations don't block movement.
    pub fn walkable(&self) -> bool {
        matches!(self.kind.as_str(), "rug" | "window" | "painting" | "decor" | "lamp_wall")
    }
}

fn parse_size(s: &str, default: (i32, i32)) -> (i32, i32) {
    let t: Vec<i32> = s.split_whitespace().filter_map(|v| v.parse().ok()).collect();
    match t.as_slice() {
        [w, h] if *w > 0 && *h > 0 => (*w, *h),
        _ => default,
    }
}

pub fn parse_furniture(id: &str, s: &str) -> Option<FurnitureInfo> {
    let f: Vec<&str> = s.split('/').collect();
    let kind = f.get(1).unwrap_or(&"decor").to_string();
    let default_size = match kind.as_str() {
        "chair" | "decor" | "lamp" | "torch" => (1, 2),
        "bench" | "couch" => (2, 2),
        "armchair" => (2, 2),
        "dresser" => (2, 2),
        "long table" => (5, 3),
        "table" => (2, 3),
        "rug" => (3, 2),
        "bookcase" => (2, 3),
        "fireplace" => (2, 5),
        "window" => (1, 2),
        "painting" => (2, 2),
        _ => (1, 2),
    };
    let sheet_size = parse_size(f.get(2).unwrap_or(&"-1"), default_size);
    let footprint = parse_size(f.get(3).unwrap_or(&"-1"), (sheet_size.0, if kind == "rug" { sheet_size.1 } else { 1 }));
    Some(FurnitureInfo {
        name: f.first()?.to_string(),
        kind,
        sheet_size,
        footprint,
        price: f.get(5).and_then(|v| v.trim().parse().ok()).unwrap_or(100),
        display_name: f.get(7).filter(|s| !s.is_empty()).or(f.first()).unwrap_or(&"").to_string(),
        sprite_index: f.get(8).and_then(|v| v.trim().parse().ok()).unwrap_or_else(|| id.parse().unwrap_or(0)),
        texture: f.get(9).filter(|s| !s.trim().is_empty()).map(|s| s.to_string()),
    })
}

/// Implicit context tags for an object category (public data convention).
pub fn category_tag(cat: i32) -> Option<&'static str> {
    Some(match cat {
        -2 => "category_gem",
        -4 => "category_fish",
        -5 => "category_egg",
        -6 => "category_milk",
        -7 => "category_cooking",
        -8 => "category_crafting",
        -12 => "category_minerals",
        -14 => "category_meat",
        -15 => "category_metal_resources",
        -16 => "category_building_resources",
        -19 => "category_fertilizer",
        -20 => "category_junk",
        -21 => "category_bait",
        -22 => "category_tackle",
        -25 => "category_ingredients",
        -26 => "category_artisan_goods",
        -27 => "category_syrup",
        -28 => "category_monster_loot",
        -74 => "category_seeds",
        -75 => "category_vegetable",
        -79 => "category_fruits",
        -80 => "category_flowers",
        -81 => "category_greens",
        _ => return None,
    })
}

/// Resolved item data for the session.
#[derive(Clone, Debug, Default)]
pub struct ItemDb {
    pub objects: BTreeMap<String, ObjectInfo>,
    pub bigs: BTreeMap<String, BigInfo>,
    /// Keyed by qualified machine id, e.g. "(BC)12".
    pub machines: BTreeMap<String, MachineData>,
    /// Keyed by seed object id.
    pub crops: BTreeMap<String, CropData>,
    pub tools: BTreeMap<String, ToolData>,
    pub weapons: BTreeMap<String, WeaponData>,
    pub boots: BTreeMap<String, BootsInfo>,
    pub furniture: BTreeMap<String, FurnitureInfo>,
    pub hats: BTreeMap<String, ClothingInfo>,
    pub trinkets: BTreeMap<String, TrinketData>,
    pub shirts: BTreeMap<String, ClothingInfo>,
    pub pants: BTreeMap<String, ClothingInfo>,
    /// Preloaded string tables used at runtime (lowercased asset name).
    pub strings: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
}

impl ItemDb {
    pub fn load(cm: &mut ContentManager) -> Self {
        let mut db = ItemDb::default();
        for (id, data) in cm.load_dict::<ObjectData>("Data/Objects") {
            let display_name = {
                let n = cm.expand(&data.display_name);
                if n.is_empty() { data.name.clone() } else { n }
            };
            let description = cm.expand(&data.description);
            db.objects.insert(id.clone(), ObjectInfo { id, data, display_name, description });
        }
        for (id, data) in cm.load_dict::<BigCraftableData>("Data/BigCraftables") {
            let display_name = {
                let n = cm.expand(&data.display_name);
                if n.is_empty() { data.name.clone() } else { n }
            };
            let description = cm.expand(&data.description);
            db.bigs.insert(id.clone(), BigInfo { id, data, display_name, description });
        }
        db.machines = cm.load_dict::<MachineData>("Data/Machines");
        for asset in ["Strings\\Objects", "Strings\\BigCraftables", "Strings\\SpecialOrderStrings"] {
            db.strings.insert(asset.to_lowercase(), cm.load_string_dict(asset));
        }
        db.crops = cm.load_dict::<CropData>("Data/Crops");
        db.tools = cm.load_dict::<ToolData>("Data/Tools");
        db.weapons = cm.load_dict::<WeaponData>("Data/Weapons");
        for (id, raw) in cm.load_string_dict("Data/Furniture") {
            if let Some(mut f) = parse_furniture(&id, &raw) {
                f.display_name = cm.expand(&f.display_name);
                db.furniture.insert(id, f);
            }
        }
        for (id, mut t) in cm.load_dict::<TrinketData>("Data/Trinkets") {
            t.display_name = cm.expand(&t.display_name);
            t.description = cm.expand(&t.description);
            db.trinkets.insert(id, t);
        }
        for (id, raw) in cm.load_string_dict("Data/Hats") {
            if let Some(mut h) = parse_hat(&id, &raw) {
                h.display_name = cm.expand(&h.display_name);
                h.description = cm.expand(&h.description);
                db.hats.insert(id, h);
            }
        }
        for (asset, map) in [("Data/Shirts", &mut db.shirts), ("Data/Pants", &mut db.pants)] {
            for (id, c) in cm.load_dict::<ClothingJson>(asset) {
                let display = cm.expand(&c.display_name);
                map.insert(id, ClothingInfo {
                    display_name: if display.is_empty() { c.name.clone() } else { display },
                    description: cm.expand(&c.description),
                    price: c.price,
                    texture: c.texture.filter(|t| !t.is_empty()),
                    sprite_index: c.sprite_index,
                    color: c.default_color.as_deref().and_then(parse_rgb),
                    hides_hair: false,
                    name: c.name,
                });
            }
        }
        for (id, raw) in cm.load_string_dict("Data/Boots") {
            if let Some(mut b) = parse_boots(&id, &raw) {
                b.display_name = cm.expand(&b.display_name);
                b.description = cm.expand(&b.description);
                db.boots.insert(id, b);
            }
        }
        for w in db.weapons.values_mut() {
            w.display_name = cm.expand(&w.display_name);
            if w.display_name.is_empty() {
                w.display_name = w.name.clone();
            }
            w.description = cm.expand(&w.description);
        }
        for t in db.tools.values_mut() {
            t.display_name = cm.expand(&t.display_name);
            if t.display_name.is_empty() {
                t.display_name = t.name.clone();
            }
        }
        db
    }

    pub fn object(&self, id: &str) -> Option<&ObjectInfo> {
        let (_, id) = split_qualified(id);
        self.objects.get(id)
    }

    /// Expand `[LocalizedText Strings\\Objects:Key]`-style tokens using the
    /// string tables loaded with the database.
    pub fn expand_text(&self, text: &str) -> String {
        if !text.contains('[') {
            return text.to_string();
        }
        let mut look = |asset: &str, key: &str| self.strings.get(&asset.replace('/', "\\").to_lowercase()).and_then(|m| m.get(key).cloned());
        valley_content::strings::expand(text, &mut look)
    }

    pub fn big(&self, id: &str) -> Option<&BigInfo> {
        let (_, id) = split_qualified(id);
        self.bigs.get(id)
    }

    /// All context tags of an item (explicit + implicit id/category tags).
    pub fn context_tags(&self, item: &Item) -> Vec<String> {
        let mut tags = Vec::new();
        match item {
            Item::Object { id, quality, .. } => {
                tags.push(format!("id_o_{}", id.to_lowercase()));
                if let Some(o) = self.object(id) {
                    if let Some(t) = category_tag(o.data.category) {
                        tags.push(t.to_string());
                    }
                    tags.push(format!("item_type_{}", o.data.kind.to_ascii_lowercase()));
                    tags.extend(o.data.context_tags.iter().flatten().cloned());
                }
                tags.push(match quality {
                    Quality::Normal => "quality_none",
                    Quality::Silver => "quality_silver",
                    Quality::Gold => "quality_gold",
                    Quality::Iridium => "quality_iridium",
                }.to_string());
            }
            Item::Big { id, .. } => {
                tags.push(format!("id_bc_{}", id.to_lowercase()));
                if let Some(b) = self.big(id) {
                    tags.extend(b.data.context_tags.iter().flatten().cloned());
                }
            }
            Item::Tool(_) => tags.push("category_tool".into()),
            Item::Weapon { .. } => tags.push("category_weapon".into()),
            Item::Boots { .. } => tags.push("category_boots".into()),
            Item::Trinket { .. } => tags.push("category_trinket".into()),
            Item::Clothing { slot, .. } => {
                tags.push("category_clothing".into());
                tags.push(match slot {
                    ClothingSlot::Hat => "category_hat",
                    ClothingSlot::Shirt => "category_shirt",
                    ClothingSlot::Pants => "category_pants",
                }.into());
            }
            Item::Furniture { .. } => tags.push("category_furniture".into()),
        }
        tags
    }

    /// Does the item match all tags in a list? Supports `!tag` negation and
    /// `a,b` alternatives inside one entry.
    pub fn matches_tags(&self, item: &Item, required: &[String]) -> bool {
        let tags = self.context_tags(item);
        required.iter().all(|entry| {
            entry.split(',').any(|alt| {
                let alt = alt.trim();
                match alt.strip_prefix('!') {
                    Some(t) => !tags.iter().any(|x| x == t),
                    None => tags.iter().any(|x| x == alt),
                }
            })
        })
    }

    pub fn display_name(&self, item: &Item) -> String {
        if let Some(n) = item.extra().and_then(|e| e.display_name.clone()) {
            return n;
        }
        match item {
            Item::Object { id, .. } => self.object(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Big { id, .. } => self.big(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Weapon { id, .. } => self.weapons.get(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Boots { id } => self.boots.get(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Clothing { slot, id } => self.clothing(*slot, id).map(|c| c.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Trinket { id } => self.trinkets.get(id).map(|t| t.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Furniture { id } => self.furniture.get(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Tool(t) => {
                let base = self.tools.get(t.kind.data_key()).map(|d| d.display_name.clone()).unwrap_or_else(|| t.kind.data_key().to_string());
                match t.level {
                    1 => format!("Copper {base}"),
                    2 => format!("Steel {base}"),
                    3 => format!("Gold {base}"),
                    4 => format!("Iridium {base}"),
                    _ => base,
                }
            }
        }
    }

    pub fn description(&self, item: &Item) -> String {
        let base = self.base_description(item);
        let forged = crate::forge::forge_lines(item);
        if forged.is_empty() {
            base
        } else {
            format!("{base}\n{}", forged.join("\n"))
        }
    }

    fn base_description(&self, item: &Item) -> String {
        match item {
            Item::Object { id, .. } => self.object(id).map(|o| o.description.clone()).unwrap_or_default(),
            Item::Big { id, .. } => self.big(id).map(|o| o.description.clone()).unwrap_or_default(),
            Item::Weapon { id, .. } => self.weapons.get(id).map(|o| o.description.clone()).unwrap_or_default(),
            Item::Boots { id } => self.boots.get(id).map(|b| format!("{}\nDefense +{}", b.description, b.defense)).unwrap_or_default(),
            Item::Clothing { slot, id } => self.clothing(*slot, id).map(|c| c.description.clone()).unwrap_or_default(),
            Item::Trinket { id } => self.trinkets.get(id).map(|t| t.description.clone()).unwrap_or_default(),
            Item::Furniture { id } => self.furniture.get(id).map(|f| format!("Furniture ({}). Can be placed indoors.", f.kind)).unwrap_or_default(),
            Item::Tool(t) => self.tools.get(t.kind.data_key()).map(|d| d.description.clone()).unwrap_or_default(),
        }
    }

    /// Base sell price (one unit) including quality.
    pub fn sell_price(&self, item: &Item) -> i32 {
        match item {
            Item::Object { id, quality, extra, .. } => {
                let base = extra.as_ref().and_then(|e| e.price).or_else(|| self.object(id).map(|o| o.data.price)).unwrap_or(0);
                (base as f32 * quality.price_mult()).floor() as i32
            }
            Item::Big { id, .. } => self.big(id).map(|b| b.data.price).unwrap_or(0),
            Item::Boots { id } => self.boots.get(id).map(|b| b.price / 2).unwrap_or(0),
            Item::Clothing { slot, id } => self.clothing(*slot, id).map(|c| c.price / 2).unwrap_or(0),
            Item::Trinket { .. } => 250,
            Item::Furniture { id } => self.furniture.get(id).map(|f| f.price / 2).unwrap_or(0),
            Item::Tool(_) | Item::Weapon { .. } => 0,
        }
    }

    pub fn is_seed(&self, id: &str) -> bool {
        let (_, id) = split_qualified(id);
        self.crops.contains_key(id)
    }

    /// Instantiate an item from a (possibly qualified) id.
    pub fn clothing(&self, slot: ClothingSlot, id: &str) -> Option<&ClothingInfo> {
        match slot {
            ClothingSlot::Hat => self.hats.get(id),
            ClothingSlot::Shirt => self.shirts.get(id),
            ClothingSlot::Pants => self.pants.get(id),
        }
    }

    pub fn create(&self, qualified: &str, stack: i32) -> Option<Item> {
        let (ty, id) = split_qualified(qualified);
        match ty {
            "O" => self.objects.contains_key(id).then(|| Item::object(id, stack)),
            "BC" => self.bigs.contains_key(id).then(|| Item::big(id, stack)),
            "W" => self.weapons.contains_key(id).then(|| Item::weapon(id)),
            "B" => self.boots.contains_key(id).then(|| Item::Boots { id: id.to_string() }),
            "F" => self.furniture.contains_key(id).then(|| Item::Furniture { id: id.to_string() }),
            "TR" => self.trinkets.contains_key(id).then(|| Item::Trinket { id: id.to_string() }),
            "H" => self.hats.contains_key(id).then(|| Item::Clothing { slot: ClothingSlot::Hat, id: id.to_string() }),
            "S" => self.shirts.contains_key(id).then(|| Item::Clothing { slot: ClothingSlot::Shirt, id: id.to_string() }),
            "P" => self.pants.contains_key(id).then(|| Item::Clothing { slot: ClothingSlot::Pants, id: id.to_string() }),
            "T" => ToolKind::from_key(id).map(|k| Item::Tool(Tool::new(k))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_furniture() {
        let f = parse_furniture("X", "Oak Table/table/-1/-1/1/350/-1/Oak Table/12").unwrap();
        assert_eq!(f.sheet_size, (2, 3));
        assert_eq!(f.footprint, (2, 1));
        assert_eq!(f.price, 350);
        let r = parse_furniture("Y", "Mat/rug/3 2/3 2/1/200/-1/Mat").unwrap();
        assert!(r.walkable());
        assert_eq!(r.footprint, (3, 2));
    }

    #[test]
    fn qualified_ids() {
        assert_eq!(split_qualified("(O)24"), ("O", "24"));
        assert_eq!(split_qualified("24"), ("O", "24"));
        assert_eq!(split_qualified("(BC)12"), ("BC", "12"));
    }

    #[test]
    fn db_loads_placeholder() {
        let mut cm = ContentManager::dev_only();
        let db = ItemDb::load(&mut cm);
        assert_eq!(db.display_name(&Item::object("Dev_Turnip", 1)), "Turnip");
        assert!(db.is_seed("(O)Dev_TurnipSeeds"));
        assert_eq!(db.sell_price(&Item::object_q("Dev_Turnip", 1, Quality::Gold)), 60);
        assert_eq!(db.display_name(&Item::Tool(Tool::new(ToolKind::WateringCan))), "Watering Can");
    }
}
