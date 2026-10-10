use std::collections::BTreeMap;

use engine::{IRect, Vec2};
use serde::{Deserialize, Serialize};

use crate::inventory::Inventory;
use crate::item::{Item, Tool, ToolKind};
use crate::{Dir, TilePos, TILE, TILE_F};

/// Documented experience thresholds for skill levels 1..=10.
pub const XP_LEVELS: [i32; 10] = [100, 380, 770, 1300, 2150, 3300, 4800, 6900, 10000, 15000];

pub const START_STAMINA: i32 = 270;
pub const START_HEALTH: i32 = 100;
pub const START_MONEY: i32 = 500;
pub const BACKPACK: usize = 36;
pub const START_UNLOCKED_SLOTS: usize = 12;

/// Running speed in world pixels per tick (approximation of the original feel).
pub const RUN_SPEED: f32 = 5.0;
pub const WALK_SPEED: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Skill {
    Farming,
    Fishing,
    Foraging,
    Mining,
    Combat,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skills {
    pub xp: BTreeMap<Skill, i32>,
}

impl Skills {
    pub fn xp(&self, s: Skill) -> i32 {
        *self.xp.get(&s).unwrap_or(&0)
    }
    pub fn level(&self, s: Skill) -> i32 {
        let xp = self.xp(s);
        XP_LEVELS.iter().take_while(|&&t| xp >= t).count() as i32
    }
    /// Add experience; returns the new level if it increased.
    pub fn gain(&mut self, s: Skill, amount: i32) -> Option<i32> {
        let before = self.level(s);
        *self.xp.entry(s).or_insert(0) += amount.max(0);
        let after = self.level(s);
        (after > before).then_some(after)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Friendship {
    pub points: i32,
    pub talked_today: bool,
    pub gifts_this_week: i32,
    pub gifted_today: bool,
    pub met: bool,
}

impl Friendship {
    pub const POINTS_PER_HEART: i32 = 250;
    pub fn hearts(&self) -> i32 {
        self.points / Self::POINTS_PER_HEART
    }
}

/// In-progress tool swing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolUse {
    pub kind: ToolKind,
    pub slot: usize,
    pub target: TilePos,
    pub elapsed_ms: u32,
    pub applied: bool,
}

/// Tool swing timing (approximate): effect lands at HIT, swing ends at TOTAL.
pub const TOOL_HIT_MS: u32 = 200;
pub const TOOL_TOTAL_MS: u32 = 400;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Farmer {
    pub name: String,
    pub farm_name: String,
    pub favorite_thing: String,
    pub male: bool,
    /// World-space top-left of the farmer's 64x64 footprint.
    pub pos: Vec2,
    pub facing: Dir,
    pub stamina: f32,
    pub max_stamina: i32,
    pub health: i32,
    pub max_health: i32,
    pub money: i32,
    pub total_earned: i64,
    pub inventory: Inventory,
    pub unlocked_slots: usize,
    pub current_slot: usize,
    pub skills: Skills,
    pub friendships: BTreeMap<String, Friendship>,
    pub mailbox: Vec<String>,
    pub mail_received: Vec<String>,
    pub events_seen: Vec<String>,
    /// Known crafting recipes and how many times each was crafted.
    #[serde(default)]
    pub crafting_recipes: BTreeMap<String, i32>,
    pub tool_use: Option<ToolUse>,
    /// Animation clock while moving (ms).
    pub walk_ms: u32,
    pub moving: bool,
    pub exhausted: bool,
    /// Ticks the farmer is frozen (eating, item hold-up, etc.).
    pub freeze_ms: u32,
    #[serde(default)]
    pub invincible_ms: u32,
    #[serde(default)]
    pub knockback: Vec2,
    /// Deepest mine level reached.
    #[serde(default)]
    pub deepest_mine_level: i32,
    #[serde(default)]
    pub buffs: Vec<crate::buffs::Buff>,
    #[serde(default)]
    pub cooking_recipes: BTreeMap<String, i32>,
    #[serde(default)]
    pub quests: Vec<crate::quests::Quest>,
    /// Letters already read (ids), for the collection tab.
    #[serde(default)]
    pub letters_read: Vec<String>,
}

impl Farmer {
    pub fn new(name: &str, farm_name: &str) -> Self {
        let mut inventory = Inventory::new(BACKPACK);
        for k in [ToolKind::Axe, ToolKind::Hoe, ToolKind::WateringCan, ToolKind::Pickaxe, ToolKind::Scythe] {
            inventory.add(Item::Tool(Tool::new(k)));
        }
        Self {
            name: name.to_string(),
            farm_name: farm_name.to_string(),
            favorite_thing: String::new(),
            male: true,
            pos: Vec2::ZERO,
            facing: Dir::Down,
            stamina: START_STAMINA as f32,
            max_stamina: START_STAMINA,
            health: START_HEALTH,
            max_health: START_HEALTH,
            money: START_MONEY,
            total_earned: 0,
            inventory,
            unlocked_slots: START_UNLOCKED_SLOTS,
            current_slot: 0,
            skills: Skills::default(),
            friendships: BTreeMap::new(),
            mailbox: Vec::new(),
            mail_received: Vec::new(),
            events_seen: Vec::new(),
            crafting_recipes: BTreeMap::new(),
            tool_use: None,
            walk_ms: 0,
            moving: false,
            exhausted: false,
            freeze_ms: 0,
            invincible_ms: 0,
            knockback: Vec2::ZERO,
            deepest_mine_level: 0,
            buffs: Vec::new(),
            cooking_recipes: BTreeMap::new(),
            quests: Vec::new(),
            letters_read: Vec::new(),
        }
    }

    /// Collision box (lower part of the footprint, inset from the sides).
    pub fn bbox_at(pos: Vec2) -> IRect {
        IRect::new(pos.x as i32 + 8, pos.y as i32 + 32, 48, 32)
    }

    pub fn bbox(&self) -> IRect {
        Self::bbox_at(self.pos)
    }

    /// Tile the farmer is standing on (center of the collision box).
    pub fn tile(&self) -> TilePos {
        TilePos::from_world(self.bbox().center())
    }

    pub fn facing_tile(&self) -> TilePos {
        self.tile().offset(self.facing)
    }

    /// Place the farmer standing on `t`.
    pub fn set_tile(&mut self, t: TilePos) {
        self.pos = Vec2::new(t.x as f32 * TILE_F, t.y as f32 * TILE_F - 16.0);
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.pos.x + TILE_F / 2.0, self.pos.y + TILE as f32 / 2.0 + 16.0)
    }

    pub fn current_item(&self) -> Option<&Item> {
        self.inventory.get(self.current_slot)
    }

    pub fn current_item_mut(&mut self) -> Option<&mut Item> {
        self.inventory.get_mut(self.current_slot)
    }

    pub fn friendship(&mut self, npc: &str) -> &mut Friendship {
        self.friendships.entry(npc.to_string()).or_default()
    }

    pub fn hearts(&self, npc: &str) -> i32 {
        self.friendships.get(npc).map(|f| f.hearts()).unwrap_or(0)
    }

    /// Spend stamina; marks exhaustion when it drops to zero or below.
    pub fn spend_stamina(&mut self, amount: f32) {
        self.stamina -= amount;
        if self.stamina <= 0.0 {
            self.exhausted = true;
        }
    }

    /// Sum of all active buff attributes.
    pub fn buff_totals(&self) -> crate::buffs::BuffAttrs {
        let mut t = crate::buffs::BuffAttrs::default();
        for b in &self.buffs {
            t.add(&b.attrs);
        }
        t
    }

    /// Skill level including buffs.
    pub fn effective_level(&self, s: Skill) -> i32 {
        let b = self.buff_totals();
        self.skills.level(s)
            + match s {
                Skill::Farming => b.farming,
                Skill::Fishing => b.fishing,
                Skill::Mining => b.mining,
                Skill::Foraging => b.foraging,
                Skill::Combat => b.combat,
            }
    }

    pub fn is_busy(&self) -> bool {
        self.tool_use.is_some() || self.freeze_ms > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_levels() {
        let mut s = Skills::default();
        assert_eq!(s.level(Skill::Farming), 0);
        assert_eq!(s.gain(Skill::Farming, 100), Some(1));
        assert_eq!(s.gain(Skill::Farming, 279), None);
        assert_eq!(s.gain(Skill::Farming, 1), Some(2));
        s.gain(Skill::Farming, 100000);
        assert_eq!(s.level(Skill::Farming), 10);
    }

    #[test]
    fn tile_placement_roundtrip() {
        let mut f = Farmer::new("A", "B");
        for t in [TilePos::new(0, 0), TilePos::new(5, 9), TilePos::new(40, 2)] {
            f.set_tile(t);
            assert_eq!(f.tile(), t);
        }
    }
}
