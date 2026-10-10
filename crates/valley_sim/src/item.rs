//! Items and the item registry built from content data.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use valley_content::data::{CropData, ObjectData, ToolData};
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Object { id: String, stack: i32, quality: Quality },
    Tool(Tool),
}

impl Item {
    pub fn object(id: &str, stack: i32) -> Self {
        Item::Object { id: id.to_string(), stack, quality: Quality::Normal }
    }
    pub fn object_q(id: &str, stack: i32, quality: Quality) -> Self {
        Item::Object { id: id.to_string(), stack, quality }
    }

    pub fn stack(&self) -> i32 {
        match self {
            Item::Object { stack, .. } => *stack,
            Item::Tool(_) => 1,
        }
    }

    pub fn set_stack(&mut self, n: i32) {
        if let Item::Object { stack, .. } = self {
            *stack = n;
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
            (Item::Object { id: a, quality: qa, .. }, Item::Object { id: b, quality: qb, .. }) => a == b && qa == qb,
            _ => false,
        }
    }

    /// Qualified id like "(O)24" or "(T)Hoe".
    pub fn qualified_id(&self) -> String {
        match self {
            Item::Object { id, .. } => format!("(O){id}"),
            Item::Tool(t) => format!("(T){}", t.kind.data_key()),
        }
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

/// Resolved item data for the session.
#[derive(Clone, Debug, Default)]
pub struct ItemDb {
    pub objects: BTreeMap<String, ObjectInfo>,
    /// Keyed by seed object id.
    pub crops: BTreeMap<String, CropData>,
    pub tools: BTreeMap<String, ToolData>,
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
        db.crops = cm.load_dict::<CropData>("Data/Crops");
        db.tools = cm.load_dict::<ToolData>("Data/Tools");
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

    pub fn display_name(&self, item: &Item) -> String {
        match item {
            Item::Object { id, .. } => self.object(id).map(|o| o.display_name.clone()).unwrap_or_else(|| format!("Error Item ({id})")),
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
            Item::Tool(t) => self.tools.get(t.kind.data_key()).map(|d| d.description.clone()).unwrap_or_default(),
        }
    }

    /// Base sell price (one unit) including quality.
    pub fn sell_price(&self, item: &Item) -> i32 {
        match item {
            Item::Object { id, quality, .. } => self.object(id).map(|o| (o.data.price as f32 * quality.price_mult()).floor() as i32).unwrap_or(0),
            Item::Tool(_) => 0,
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
