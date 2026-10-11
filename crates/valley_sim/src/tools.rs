//! Tool use: starting a swing, and applying its effect at the hit frame.

use engine::Vec2;

use crate::farmer::{Skill, ToolCharge, ToolUse, TOOL_HIT_MS, TOOL_TOTAL_MS};
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
        ToolKind::MilkPail | ToolKind::Shears => Skill::Farming,
    }
}

impl World {
    /// Left-click / use. Starts a tool swing or places seeds. Returns true if
    /// the input was consumed.
    pub fn begin_use(&mut self, cursor: Option<TilePos>) -> bool {
        if self.farmer.mounted.is_some() {
            return false;
        }
        let slot = self.farmer.current_slot;
        let target = self.target_tile(cursor);
        match self.farmer.current_item().cloned() {
            Some(Item::Tool(t)) => {
                if t.kind == ToolKind::FishingRod {
                    self.start_fishing();
                    return true;
                }
                if matches!(t.kind, ToolKind::Hoe | ToolKind::WateringCan) && t.level > 0 {
                    self.farmer.tool_charge = Some(ToolCharge { kind: t.kind, slot, max_power: t.level, ms: 0 });
                    return true;
                }
                let level = self.farmer.effective_level(governing_skill(t.kind));
                let cost = stamina_cost(t.kind, level);
                if cost > 0.0 {
                    let was_exhausted = self.farmer.exhausted;
                    self.farmer.spend_stamina(cost);
                    if self.farmer.exhausted && !was_exhausted {
                        self.emit(WorldEvent::Exhausted);
                    }
                }
                self.farmer.tool_use = Some(ToolUse { kind: t.kind, slot, target, elapsed_ms: 0, applied: false, power: 0 });
                self.sound(match t.kind {
                    ToolKind::Scythe => "swordswipe",
                    _ => "woosh",
                });
                true
            }
            Some(Item::Weapon { .. }) if self.held_weapon_type() == Some(crate::arsenal::TYPE_SLINGSHOT) => self.begin_sling_pull(cursor),
            Some(Item::Weapon { .. }) => self.begin_weapon_swing(),
            Some(Item::Object { id, .. }) if self.items.is_seed(&id) => self.plant(target, &id),
            Some(item) if self.is_placeable(&item) => self.place_current(target),
            _ => false,
        }
    }

    /// Continue or release a charge. Returns true while charging.
    pub(crate) fn update_tool_charge(&mut self, held: bool, dt: u32) -> bool {
        let Some(mut c) = self.farmer.tool_charge.take() else { return false };
        if held {
            let before = c.power();
            c.ms += dt;
            if c.power() > before {
                self.sound("toolCharge");
            }
            self.farmer.tool_charge = Some(c);
            return true;
        }
        let power = c.power();
        let level = self.farmer.effective_level(governing_skill(c.kind));
        let cost = stamina_cost(c.kind, level) * (power + 1) as f32;
        let was = self.farmer.exhausted;
        self.farmer.spend_stamina(cost);
        if self.farmer.exhausted && !was {
            self.emit(WorldEvent::Exhausted);
        }
        let target = self.farmer.facing_tile();
        self.farmer.tool_use = Some(ToolUse { kind: c.kind, slot: c.slot, target, elapsed_ms: 0, applied: false, power });
        self.sound("woosh");
        true
    }

    /// Tiles affected by a charged swing (documented area shapes).
    pub fn charged_area(&self, origin: TilePos, power: i32) -> Vec<TilePos> {
        let f = self.farmer.facing;
        let (dx, dy) = f.delta();
        let (sx, sy) = (dy.abs(), dx.abs()); // sideways axis
        let line = |len: i32| -> Vec<TilePos> { (0..len).map(|i| TilePos::new(origin.x + dx * i, origin.y + dy * i)).collect() };
        match power {
            0 => vec![origin],
            1 => line(3),
            2 => line(5),
            p => {
                let len = if p == 3 { 3 } else { 6 };
                let mut v = Vec::new();
                for i in 0..len {
                    for side in -1..=1 {
                        v.push(TilePos::new(origin.x + dx * i + sx * side, origin.y + dy * i + sy * side));
                    }
                }
                v
            }
        }
    }

    pub(crate) fn advance_tool_use(&mut self, dt: u32) {
        let Some(mut u) = self.farmer.tool_use.take() else { return };
        u.elapsed_ms += dt;
        if !u.applied && u.elapsed_ms >= TOOL_HIT_MS {
            u.applied = true;
            self.farmer.tool_use = Some(u.clone());
            if u.power > 0 {
                for t in self.charged_area(u.target, u.power) {
                    self.apply_tool(u.kind, u.slot, t);
                }
            } else {
                self.apply_tool(u.kind, u.slot, u.target);
            }
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
        if kind != ToolKind::WateringCan && self.break_slime_ball(t) {
            return;
        }
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
                self.melee_hit(1, 3, 0.5, 0.02, 2.0, 0);
                let side = match self.farmer.facing {
                    Dir::Up | Dir::Down => [Dir::Left, Dir::Right],
                    _ => [Dir::Up, Dir::Down],
                };
                for p in [t, t.offset(side[0]), t.offset(side[1])] {
                    self.use_scythe(p);
                }
            }
            ToolKind::FishingRod => {}
            ToolKind::MilkPail | ToolKind::Shears => self.use_animal_tool(kind, t),
        }
    }

    fn use_hoe(&mut self, t: TilePos) {
        if self.dig_artifact_spot(t) {
            return;
        }
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
        if self.water_trough(t) {
            self.sound("slosh");
            return;
        }
        if let Some(TerrainFeature::HoeDirt(d)) = self.location_mut().terrain_mut(t) {
            d.watered = true;
        }
        self.fill_pet_bowl(t);
        self.sound("wateringCan");
    }

    fn use_axe(&mut self, slot: usize, t: TilePos) {
        if self.pickup_object(t) {
            return;
        }
        if self.hit_clump(t, ToolKind::Axe, self.tool_level(slot)) {
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
            if let Some(TerrainFeature::FruitTree(ft)) = loc.state.terrain.get_mut(&t) {
                hit = true;
                ft.shake = 12;
                ft.health -= damage;
                if ft.health <= 0 {
                    remove = true;
                    drops.push((Item::object("388", rng.range(8, 13)), 1));
                    sound = "treecrack";
                }
            } else if let Some(TerrainFeature::Tree(tree)) = loc.state.terrain.get_mut(&t) {
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
        let forester = self.farmer.has_profession("forester");
        for (item, _) in drops {
            let mut stack = item.stack();
            if forester && item.object_id() == Some("388") {
                stack = (stack as f32 * 1.25).round() as i32;
            }
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

    fn use_pickaxe(&mut self, slot: usize, t: TilePos) {
        let power = self.tool_level(slot) + 1;
        if self.hit_mine_rock(t, power) {
            return;
        }
        if self.hit_clump(t, ToolKind::Pickaxe, power - 1) {
            return;
        }
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
                self.cut_grass(t);
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
