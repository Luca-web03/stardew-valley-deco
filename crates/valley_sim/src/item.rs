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
}

impl Tool {
    pub fn new(kind: ToolKind) -> Self {
        let mut t = Self { kind, level: 0, water: 0 };
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
    Weapon { id: String },
    /// Footwear from Data/Boots.
    Boots { id: String },
}

impl Item {
    pub fn object(id: &str, stack: i32) -> Self {
        Item::Object { id: id.to_string(), stack, quality: Quality::Normal, extra: None }
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
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } => 1,
        }
    }

    pub fn set_stack(&mut self, n: i32) {
        match self {
            Item::Object { stack, .. } | Item::Big { stack, .. } => *stack = n,
            Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. } => {}
        }
    }

    pub fn quality(&self) -> Quality {
        match self {
            Item::Object { quality, .. } => *quality,
            _ => Quality::Normal,
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
            Item::Weapon { id } => format!("(W){id}"),
            Item::Boots { id } => format!("(B){id}"),
        }
    }

    pub fn weapon_id(&self) -> Option<&str> {
        match self {
            Item::Weapon { id } => Some(id),
            _ => None,
        }
    }

    /// Tools, weapons and boots can't be stacked.
    pub fn is_equipment(&self) -> bool {
        matches!(self, Item::Tool(_) | Item::Weapon { .. } | Item::Boots { .. })
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
        for asset in ["Strings\\Objects", "Strings\\BigCraftables"] {
            db.strings.insert(asset.to_lowercase(), cm.load_string_dict(asset));
        }
        db.crops = cm.load_dict::<CropData>("Data/Crops");
        db.tools = cm.load_dict::<ToolData>("Data/Tools");
        db.weapons = cm.load_dict::<WeaponData>("Data/Weapons");
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
            Item::Weapon { id } => self.weapons.get(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
            Item::Boots { id } => self.boots.get(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
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
        match item {
            Item::Object { id, .. } => self.object(id).map(|o| o.description.clone()).unwrap_or_default(),
            Item::Big { id, .. } => self.big(id).map(|o| o.description.clone()).unwrap_or_default(),
            Item::Weapon { id } => self.weapons.get(id).map(|o| o.description.clone()).unwrap_or_default(),
            Item::Boots { id } => self.boots.get(id).map(|b| format!("{}\nDefense +{}", b.description, b.defense)).unwrap_or_default(),
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
            Item::Tool(_) | Item::Weapon { .. } => 0,
        }
    }

    pub fn is_seed(&self, id: &str) -> bool {
        let (_, id) = split_qualified(id);
        self.crops.contains_key(id)
    }

    /// Instantiate an item from a (possibly qualified) id.
    pub fn create(&self, qualified: &str, stack: i32) -> Option<Item> {
        let (ty, id) = split_qualified(qualified);
        match ty {
            "O" => self.objects.contains_key(id).then(|| Item::object(id, stack)),
            "BC" => self.bigs.contains_key(id).then(|| Item::big(id, stack)),
            "W" => self.weapons.contains_key(id).then(|| Item::Weapon { id: id.to_string() }),
            "B" => self.boots.contains_key(id).then(|| Item::Boots { id: id.to_string() }),
            "T" => ToolKind::from_key(id).map(|k| Item::Tool(Tool::new(k))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
