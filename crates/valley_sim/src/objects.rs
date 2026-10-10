use serde::{Deserialize, Serialize};

use crate::inventory::Inventory;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjKind {
    /// Breakable litter (weeds/twigs/stones). `tool` = which tool clears it best.
    Debris { debris: DebrisKind, health: i32 },
    /// Forageable pickup.
    Forage,
    Chest(Inventory),
    /// Any other placed item (furniture-like, crafted machines later).
    Placed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebrisKind {
    Weeds,
    Twig,
    Stone,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacedObject {
    /// Item id this object represents (used for its sprite and drops).
    pub item_id: String,
    pub kind: ObjKind,
    pub flip: bool,
    /// Visual shake ticks.
    #[serde(skip)]
    pub shake: i32,
}

impl PlacedObject {
    pub fn debris(item_id: &str, debris: DebrisKind) -> Self {
        let health = match debris {
            DebrisKind::Weeds => 1,
            DebrisKind::Twig => 1,
            DebrisKind::Stone => 1,
        };
        Self { item_id: item_id.into(), kind: ObjKind::Debris { debris, health }, flip: false, shake: 0 }
    }
    pub fn forage(item_id: &str) -> Self {
        Self { item_id: item_id.into(), kind: ObjKind::Forage, flip: false, shake: 0 }
    }
    pub fn passable(&self) -> bool {
        false
    }
}
