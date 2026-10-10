//! Tool use: starting a swing, and applying its effect at the hit frame.

use engine::Vec2;

use crate::farmer::{Skill, ToolUse, TOOL_HIT_MS, TOOL_TOTAL_MS};
use crate::item::{Item, ToolKind};
use crate::objects::{DebrisKind, ObjKind};
use crate::terrain::{HoeDirt, TerrainFeature, STUMP_HEALTH};
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos};

/// Stamina cost of one swing: 2 minus 0.1 per level of the governing skill.
pub fn stamina_cost(kind: ToolKind, level: i32) -> f32 {
    match kind {
        ToolKind::Scythe => 0.0,
        ToolKind::FishingRod => 8.0 - level as f32 * 0.1,
        _ => (2.0 - level as f32 * 0.1).max(0.1),
    }
}

pub fn governing_skill(kind: ToolKind) -> Skill {
    match kind {
        ToolKind::Hoe | ToolKind::WateringCan => Skill::Farming,
        ToolKind::Axe => Skill::Foraging,
        ToolKind::Pickaxe => Skill::Mining,
        ToolKind::Scythe => Skill::Combat,
        ToolKind::FishingRod => Skill::Fishing,
    }
}

impl World {
    /// Left-click / use. Starts a tool swing or places seeds. Returns true if
    /// the input was consumed.
    pub fn begin_use(&mut self, cursor: Option<TilePos>) -> bool {
        let slot = self.farmer.current_slot;
        let target = self.target_tile(cursor);
        match self.farmer.current_item().cloned() {
            Some(Item::Tool(t)) => {
                if t.kind == ToolKind::FishingRod {
                    self.hud("Fishing isn't available yet.");
                    return true;
                }
                let level = self.farmer.skills.level(governing_skill(t.kind));
                let cost = stamina_cost(t.kind, level);
                if cost > 0.0 {
                    let was_exhausted = self.farmer.exhausted;
                    self.farmer.spend_stamina(cost);
                    if self.farmer.exhausted && !was_exhausted {
                        self.emit(WorldEvent::Exhausted);
                    }
                }
                self.farmer.tool_use = Some(ToolUse { kind: t.kind, slot, target, elapsed_ms: 0, applied: false });
                self.sound(match t.kind {
                    ToolKind::Scythe => "swordswipe",
                    _ => "woosh",
                });
                true
            }
            Some(Item::Object { id, .. }) if self.items.is_seed(&id) => self.plant(target, &id),
            Some(item) if self.is_placeable(&item) => self.place_current(target),
            _ => false,
        }
    }

    pub(crate) fn advance_tool_use(&mut self, dt: u32) {
        let Some(mut u) = self.farmer.tool_use.take() else { return };
        u.elapsed_ms += dt;
        if !u.applied && u.elapsed_ms >= TOOL_HIT_MS {
            u.applied = true;
            self.farmer.tool_use = Some(u.clone());
            self.apply_tool(u.kind, u.slot, u.target);
            u = match self.farmer.tool_use.take() {
                Some(x) => x,
                None => return,
            };
        }
        if u.elapsed_ms < TOOL_TOTAL_MS {
            self.farmer.tool_use = Some(u);
        }
    }

    fn tool_level(&self, slot: usize) -> i32 {
        self.farmer.inventory.get(slot).and_then(|i| i.tool()).map(|t| t.level).unwrap_or(0)
    }

    pub fn apply_tool(&mut self, kind: ToolKind, slot: usize, t: TilePos) {
        // Any tool clears weeds.
        if let Some(o) = self.location().object(t) {
            if matches!(o.kind, ObjKind::Debris { debris: DebrisKind::Weeds, .. }) && kind != ToolKind::WateringCan {
                self.break_debris(t, DebrisKind::Weeds);
                if kind != ToolKind::Scythe {
                    return;
                }
            }
        }
        match kind {
            ToolKind::Hoe => self.use_hoe(t),
            ToolKind::WateringCan => self.use_watering_can(slot, t),
            ToolKind::Axe => self.use_axe(slot, t),
            ToolKind::Pickaxe => self.use_pickaxe(slot, t),
            ToolKind::Scythe => {
                let side = match self.farmer.facing {
                    Dir::Up | Dir::Down => [Dir::Left, Dir::Right],
                    _ => [Dir::Up, Dir::Down],
                };
                for p in [t, t.offset(side[0]), t.offset(side[1])] {
                    self.use_scythe(p);
                }
            }
            ToolKind::FishingRod => {}
        }
    }

    fn use_hoe(&mut self, t: TilePos) {
        let loc = self.location();
        if loc.is_clear(t) && loc.is_diggable(t) {
            self.location_mut().state.terrain.insert(t, TerrainFeature::HoeDirt(HoeDirt::new()));
            self.sound("hoeHit");
        } else {
            self.sound("clank");
        }
    }

    fn use_watering_can(&mut self, slot: usize, t: TilePos) {
        if self.location().is_water(t) {
            if let Some(tool) = self.farmer.inventory.get_mut(slot).and_then(|i| i.tool_mut()) {
                tool.water = tool.water_capacity();
            }
            self.sound("slosh");
            return;
        }
        let has_water = self.farmer.inventory.get(slot).and_then(|i| i.tool()).is_some_and(|t| t.water > 0);
        if !has_water {
            self.hud("Your watering can is empty.");
            self.sound("woodyStep");
            return;
        }
        if let Some(tool) = self.farmer.inventory.get_mut(slot).and_then(|i| i.tool_mut()) {
            tool.water -= 1;
        }
        if let Some(TerrainFeature::HoeDirt(d)) = self.location_mut().terrain_mut(t) {
            d.watered = true;
        }
        self.sound("wateringCan");
    }

    fn use_axe(&mut self, slot: usize, t: TilePos) {
        if self.pickup_object(t) {
            return;
        }
        let damage = self.tool_level(slot) + 1;
        if let Some(o) = self.location().object(t) {
            if let ObjKind::Debris { debris: DebrisKind::Twig, .. } = o.kind {
                self.break_debris(t, DebrisKind::Twig);
                return;
            }
        }
        let center = t.world_center();
        let mut drops: Vec<(Item, i32)> = Vec::new();
        let mut remove = false;
        let mut xp = 0;
        let mut sound = "axchop";
        let mut hit = false;
        {
            let mut rng = self.rng.clone();
            let loc = self.location_mut();
            if let Some(TerrainFeature::Tree(tree)) = loc.state.terrain.get_mut(&t) {
                hit = true;
                tree.shake = 12;
                if tree.stage < 1 {
                    remove = true;
                } else if tree.stage < crate::terrain::TREE_MATURE {
                    tree.health -= damage;
                    if tree.health <= 0 {
                        remove = true;
                        drops.push((Item::object("388", 1), 1));
                    }
                } else {
                    tree.health -= damage;
                    if tree.health <= 0 {
                        if tree.stump {
                            remove = true;
                            drops.push((Item::object("388", rng.range(4, 6)), 1));
                            xp = 1;
                        } else {
                            tree.stump = true;
                            tree.health = STUMP_HEALTH;
                            tree.falling = 1;
                            drops.push((Item::object("388", rng.range(12, 17)), 1));
                            if rng.chance(0.5) {
                                drops.push((Item::object("92", rng.range(1, 3)), 1));
                            }
                            xp = 12;
                            sound = "treecrack";
                        }
                    }
                }
            }
            if remove {
                loc.state.terrain.remove(&t);
            }
            self.rng = rng;
        }
        if !hit {
            self.sound("woodyStep");
            return;
        }
        self.sound(sound);
        if xp > 0 {
            if let Some(l) = self.farmer.skills.gain(Skill::Foraging, xp) {
                self.emit(WorldEvent::LevelUp(Skill::Foraging, l));
            }
        }
        for (item, _) in drops {
            let stack = item.stack();
            // Pop wood out a few pieces at a time so they spread visually.
            let id = item.object_id().unwrap_or_default().to_string();
            let mut left = stack;
            while left > 0 {
                let n = left.min(5);
                left -= n;
                self.spawn_drop(Item::object(&id, n), center + Vec2::new(0.0, -32.0));
            }
        }
    }

    fn use_pickaxe(&mut self, _slot: usize, t: TilePos) {
        if self.pickup_object(t) {
            return;
        }
        if let Some(o) = self.location().object(t) {
            if let ObjKind::Debris { debris: DebrisKind::Stone, .. } = o.kind {
                self.break_debris(t, DebrisKind::Stone);
                return;
            }
        }
        let remove_dirt = matches!(self.location().terrain(t), Some(TerrainFeature::HoeDirt(d)) if d.crop.is_none() || d.crop.as_ref().is_some_and(|c| c.dead));
        if remove_dirt {
            self.location_mut().state.terrain.remove(&t);
            self.sound("hammer");
            return;
        }
        self.sound("clank");
    }

    fn use_scythe(&mut self, t: TilePos) {
        if let Some(o) = self.location().object(t) {
            if let ObjKind::Debris { debris: DebrisKind::Weeds, .. } = o.kind {
                self.break_debris(t, DebrisKind::Weeds);
                return;
            }
        }
        let crop = match self.location().terrain(t) {
            Some(TerrainFeature::HoeDirt(d)) => d.crop.clone(),
            Some(TerrainFeature::Grass(_)) => {
                self.location_mut().state.terrain.remove(&t);
                self.sound("cut");
                return;
            }
            _ => None,
        };
        if let Some(c) = crop {
            if c.dead {
                if let Some(TerrainFeature::HoeDirt(d)) = self.location_mut().terrain_mut(t) {
                    d.crop = None;
                }
                return;
            }
            if let Some(data) = self.items.crops.get(&c.seed_id).cloned() {
                if c.is_ready(&data) && data.harvest_method.eq_ignore_ascii_case("Scythe") {
                    self.harvest_at(t, &data);
                }
            }
        }
    }

    fn break_debris(&mut self, t: TilePos, kind: DebrisKind) {
        self.location_mut().state.objects.remove(&t);
        let center = t.world_center();
        match kind {
            DebrisKind::Weeds => {
                self.sound("cut");
                if self.rng.chance(0.5) {
                    self.spawn_drop(Item::object("771", 1), center);
                }
            }
            DebrisKind::Twig => {
                self.sound("axchop");
                self.spawn_drop(Item::object("388", 1), center);
                self.farmer.skills.gain(Skill::Foraging, 1);
            }
            DebrisKind::Stone => {
                self.sound("hammer");
                self.spawn_drop(Item::object("390", 1), center);
                if let Some(l) = self.farmer.skills.gain(Skill::Mining, 1) {
                    self.emit(WorldEvent::LevelUp(Skill::Mining, l));
                }
            }
        }
    }
}
