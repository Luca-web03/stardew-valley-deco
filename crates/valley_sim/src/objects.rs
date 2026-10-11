use serde::{Deserialize, Serialize};

use crate::inventory::Inventory;
use crate::item::Item;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjKind {
    /// Breakable litter (weeds/twigs/stones).
    Debris { debris: DebrisKind, health: i32 },
    /// Forageable pickup.
    Forage,
    Chest(Inventory),
    /// A machine processing an input; `output` is ready when `minutes_left` <= 0.
    Machine { output: Option<Item>, minutes_left: i32, ready: bool },
    /// Waters nearby tilled soil each morning.
    Sprinkler {
        radius: SprinklerShape,
        /// Pressure nozzle or enricher (`automation`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        attachment: Option<String>,
        /// Fertilizer held by an enricher.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fertilizer: Option<Item>,
    },
    /// Breakable mine rock or ore node.
    MineRock { rock: crate::mines::RockKind, health: i32 },
    /// Mine ladder (down to the next level, or up to the entrance).
    Ladder { down: bool },
    /// A hole in the deep cavern floor (drops several floors).
    Shaft,
    /// Tapper on a tree: produces `item` every `days` days.
    Tapper { output: Option<Item>, days_left: i32 },
    /// Diggable artifact spot.
    ArtifactSpot,
    /// Crab pot on a water tile.
    CrabPot { bait: bool, catch: Option<Item> },
    /// Cask aging `item`; `aged` is in base days.
    Cask { item: Option<Item>, aged: f32 },
    /// A treasure chest that hands over its items when opened.
    Treasure { items: Vec<Item> },
    /// Fence or gate (`fences`): days of health left, gate open.
    Fence { health: i32, open: bool },
    /// Any other placed item.
    Placed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SprinklerShape {
    /// The four orthogonal neighbours.
    Cross,
    /// The 3x3 square around it.
    Square1,
    /// The 5x5 square around it.
    Square2,
    /// The 7x7 square around it (with a pressure nozzle).
    Square3,
}

impl SprinklerShape {
    pub fn tiles(self, c: crate::TilePos) -> Vec<crate::TilePos> {
        let mut v = Vec::new();
        match self {
            SprinklerShape::Cross => {
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    v.push(crate::TilePos::new(c.x + dx, c.y + dy));
                }
            }
            SprinklerShape::Square1 | SprinklerShape::Square2 | SprinklerShape::Square3 => {
                let r = match self {
                    SprinklerShape::Square1 => 1,
                    SprinklerShape::Square2 => 2,
                    _ => 3,
                };
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx != 0 || dy != 0 {
                            v.push(crate::TilePos::new(c.x + dx, c.y + dy));
                        }
                    }
                }
            }
        }
        v
    }
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
    /// True for big craftables (16x32 sprites, "(BC)" ids).
    #[serde(default)]
    pub big: bool,
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
        Self { item_id: item_id.into(), big: false, kind: ObjKind::Debris { debris, health }, flip: false, shake: 0 }
    }
    pub fn forage(item_id: &str) -> Self {
        Self { item_id: item_id.into(), big: false, kind: ObjKind::Forage, flip: false, shake: 0 }
    }
    pub fn new(item_id: &str, big: bool, kind: ObjKind) -> Self {
        Self { item_id: item_id.into(), big, kind, flip: false, shake: 0 }
    }
    /// Qualified id of the item this object came from.
    pub fn qualified_id(&self) -> String {
        if self.big { format!("(BC){}", self.item_id) } else { format!("(O){}", self.item_id) }
    }
    /// The item returned when the object is picked back up.
    pub fn as_item(&self) -> Item {
        if self.big { Item::big(&self.item_id, 1) } else { Item::object(&self.item_id, 1) }
    }
    pub fn passable(&self) -> bool {
        // Open gates and fallen fences can be walked through.
        matches!(self.kind, ObjKind::Fence { open: true, .. } | ObjKind::Fence { health: ..=0, .. })
    }
}
