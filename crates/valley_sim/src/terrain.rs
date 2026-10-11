use serde::{Deserialize, Serialize};

use crate::crop::Crop;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoeDirt {
    pub watered: bool,
    pub crop: Option<Crop>,
    /// Fertilizer object id, if any.
    pub fertilizer: Option<String>,
}

impl HoeDirt {
    pub fn new() -> Self {
        Self { watered: false, crop: None, fertilizer: None }
    }
}

impl Default for HoeDirt {
    fn default() -> Self {
        Self::new()
    }
}

/// Growth stage at which a wild tree is fully grown.
pub const TREE_MATURE: i32 = 5;
pub const TREE_HEALTH: i32 = 10;
/// Approximation: hits needed for a stump at basic level.
pub const STUMP_HEALTH: i32 = 5;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tree {
    /// Tree type id (e.g. "1" oak, "2" maple, "3" pine).
    pub kind: String,
    pub stage: i32,
    pub health: i32,
    pub stump: bool,
    pub tapped: bool,
    pub flip: bool,
    /// Ticks of the shake animation left (visual only).
    #[serde(skip)]
    pub shake: i32,
    /// Ticks into the falling animation (visual only); > 0 while falling.
    #[serde(skip)]
    pub falling: i32,
}

impl Tree {
    pub fn new(kind: &str, stage: i32) -> Self {
        Self { kind: kind.into(), stage, health: TREE_HEALTH, stump: false, tapped: false, flip: false, shake: 0, falling: 0 }
    }
    pub fn mature(&self) -> bool {
        self.stage >= TREE_MATURE
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grass {
    /// 1..=4 tufts.
    pub weeds: i32,
}

/// Days for a fruit tree sapling to mature (documented).
pub const FRUIT_TREE_DAYS: i32 = 28;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FruitTree {
    /// Sapling item id (key into Data/FruitTrees).
    pub kind: String,
    pub days_until_mature: i32,
    /// Fruit currently hanging (max 3).
    pub fruit: Vec<String>,
    /// Days since maturity (quality rises with age).
    pub age_after_mature: i32,
    pub health: i32,
    #[serde(skip)]
    pub shake: i32,
}

impl FruitTree {
    pub fn new(kind: &str) -> Self {
        Self { kind: kind.into(), days_until_mature: FRUIT_TREE_DAYS, fruit: vec![], age_after_mature: 0, health: 10, shake: 0 }
    }
    pub fn mature(&self) -> bool {
        self.days_until_mature <= 0
    }
    /// Fruit quality from years since maturity (documented: +1 level per year).
    pub fn fruit_quality(&self) -> crate::item::Quality {
        crate::item::Quality::from_level(match self.age_after_mature / 112 {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 4,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TerrainFeature {
    HoeDirt(HoeDirt),
    Tree(Tree),
    Grass(Grass),
    FruitTree(FruitTree),
    /// Floor or path (`Data/FloorsAndPaths` key; see `fences`).
    Flooring(String),
    /// Bush (see `bushes`).
    Bush(crate::bushes::Bush),
}

impl TerrainFeature {
    pub fn passable(&self) -> bool {
        match self {
            TerrainFeature::HoeDirt(_) | TerrainFeature::Grass(_) | TerrainFeature::Flooring(_) => true,
            TerrainFeature::Tree(t) => t.stage < 1 && !t.stump,
            TerrainFeature::FruitTree(f) => f.days_until_mature > FRUIT_TREE_DAYS - 7,
            TerrainFeature::Bush(_) => false,
        }
    }
    pub fn hoe_dirt(&self) -> Option<&HoeDirt> {
        match self {
            TerrainFeature::HoeDirt(h) => Some(h),
            _ => None,
        }
    }
    pub fn hoe_dirt_mut(&mut self) -> Option<&mut HoeDirt> {
        match self {
            TerrainFeature::HoeDirt(h) => Some(h),
            _ => None,
        }
    }
}
