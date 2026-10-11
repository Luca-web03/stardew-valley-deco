//! Cooking, dated mail, quests and help-wanted requests (world glue).

use engine::Rng;

use crate::item::{category, Item};
use crate::quests::{parse_quest, Objective, Quest};
use crate::world::{World, WorldEvent};

impl World {
    /// Cook a known recipe (ingredients from the inventory).
    pub fn cook(&mut self, name: &str) -> bool {
        let Some(r) = self.cooking.get(name).cloned() else { return false };
        if !self.farmer.cooking_recipes.contains_key(name) {
            return false;
        }
        let out = r.output();
        if !self.farmer.inventory.can_fit(&out) {
            self.hud("Inventory full");
            return false;
        }
        let items = self.items.clone();
        let f = &mut self.farmer;
        let Some(item) = r.craft_from(&items, &mut [&mut f.inventory, &mut f.fridge]) else {
            self.sound("cancel");
            return false;
        };
        self.farmer.inventory.add(item.clone());
        *self.farmer.cooking_recipes.entry(name.to_string()).or_insert(0) += 1;
        self.sound("smallSelect");
        self.emit(WorldEvent::Pickup(item));
        true
    }

    /// Unlock cooking recipes whose conditions are met.
    pub fn unlock_cooking(&mut self) {
        let names: Vec<String> = self.cooking.iter().filter(|(_, r)| r.auto_unlocked(&self.farmer)).map(|(k, _)| k.clone()).collect();
        for n in names {
            self.farmer.cooking_recipes.entry(n).or_insert(0);
        }
    }

    /// Letters keyed by date ("spring_3", "summer_10_2") arrive that morning.
    pub(crate) fn deliver_dated_mail(&mut self) {
        self.unlock_cooking();
        let s = self.clock.season.key();
        for key in [format!("{s}_{}_{}", self.clock.day, self.clock.year), format!("{s}_{}", self.clock.day)] {
            if self.mail_data.contains_key(&key) && !self.farmer.mail_received.contains(&key) && !self.farmer.mailbox.contains(&key) {
                self.farmer.mailbox.push(key);
            }
        }
    }

    /// Accept a quest by id from Data/Quests.
    pub fn add_quest(&mut self, id: &str) -> bool {
        if self.farmer.quests.iter().any(|q| q.id == id) {
            return false;
        }
        let Some(raw) = self.quest_data.get(id).cloned() else { return false };
        let Some(q) = parse_quest(id, &raw) else { return false };
        let title = q.title.clone();
        self.farmer.quests.push(q);
        self.sound("questcomplete");
        self.emit(WorldEvent::NewQuest(title));
        true
    }

    /// Generate today's help-wanted delivery request.
    pub(crate) fn roll_daily_quest(&mut self) {
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 515]));
        self.daily_quest = None;
        if self.npcs.is_empty() || !rng.chance(0.6) {
            return;
        }
        let season = self.clock.season;
        let candidates: Vec<String> = self
            .items
            .crops
            .values()
            .filter(|c| crate::crop::Crop::grows_in(c, season))
            .map(|c| c.harvest_item_id.clone())
            .filter(|id| self.items.object(id).is_some_and(|o| matches!(o.data.category, category::VEGETABLE | category::FRUIT)))
            .collect();
        let Some(item) = rng.pick(&candidates).cloned() else { return };
        let givers: Vec<(String, String)> = self.npcs.iter().filter(|n| self.takes_item_delivery_quests(n)).map(|n| (n.name.clone(), n.display_name.clone())).collect();
        let Some(npc) = rng.pick(&givers).cloned() else { return };
        let name = self.items.object(&item).map(|o| o.display_name.clone()).unwrap_or_default();
        let price = self.items.object(&item).map(|o| o.data.price).unwrap_or(10);
        self.daily_quest = Some(Quest {
            id: format!("help_{}", self.clock.days_played()),
            title: "Help Wanted".into(),
            description: format!("{} is looking for a fresh {name}. Can you bring one by?", npc.1),
            hint: format!("Bring {name} to {}.", npc.1),
            objective: Objective::Deliver { npc: npc.0, item, count: 1 },
            reward_money: price * 3,
            next: None,
            complete: false,
            days_left: 2,
            reaction: "Oh, thank you! This is perfect.".into(),
        });
    }

    pub fn accept_daily_quest(&mut self) -> bool {
        let Some(q) = self.daily_quest.take() else { return false };
        let title = q.title.clone();
        self.farmer.quests.push(q);
        self.emit(WorldEvent::NewQuest(title));
        true
    }

    fn finish_quest(&mut self, idx: usize) {
        let q = self.farmer.quests.remove(idx);
        self.farmer.money += q.reward_money;
        if q.reward_money > 0 {
            self.hud(format!("Quest complete! +{}g", q.reward_money));
        }
        self.sound("questcomplete");
        self.farmer.quests_completed += 1;
        self.emit(WorldEvent::QuestCompleted(q.title.clone()));
        if let Some(next) = q.next {
            self.add_quest(&next);
        }
    }

    fn check_quests(&mut self) {
        loop {
            let done = self.farmer.quests.iter().position(|q| match &q.objective {
                Objective::Harvest { count, progress, .. } | Objective::Slay { count, progress, .. } => progress >= count,
                _ => q.complete,
            });
            match done {
                Some(i) => self.finish_quest(i),
                None => break,
            }
        }
    }

    /// Count newly obtained items for harvest objectives.
    pub(crate) fn quest_obtained(&mut self, item: &Item) {
        self.order_obtained(item);
        let Some(id) = item.object_id() else { return };
        for q in self.farmer.quests.iter_mut() {
            if let Objective::Harvest { item: want, progress, .. } = &mut q.objective {
                if want == id {
                    *progress += item.stack();
                }
            }
        }
        self.check_quests();
    }

    pub(crate) fn quest_killed(&mut self, monster: &str) {
        self.order_slay(monster);
        for q in self.farmer.quests.iter_mut() {
            if let Objective::Slay { monster: want, progress, .. } = &mut q.objective {
                if want.eq_ignore_ascii_case(monster) || want == "Any" {
                    *progress += 1;
                }
            }
        }
        self.check_quests();
    }

    pub(crate) fn quest_visit(&mut self, location: &str) {
        for q in self.farmer.quests.iter_mut() {
            if let Objective::Visit { location: want } = &q.objective {
                if want == location {
                    q.complete = true;
                }
            }
        }
        self.check_quests();
    }

    /// Building and social quests are checked against the world's state.
    pub(crate) fn quest_state_checks(&mut self) {
        let kinds: Vec<String> = self.buildings.iter().filter(|b| b.days_left == 0).map(|b| b.kind.clone()).collect();
        let met_all = self.npcs.iter().all(|n| self.farmer.friendships.get(&n.name).is_some_and(|f| f.met));
        let mut changed = false;
        for q in self.farmer.quests.iter_mut().filter(|q| !q.complete) {
            let done = match &q.objective {
                Objective::Build { kind } => kinds.iter().any(|k| k.eq_ignore_ascii_case(kind) || k.replace(' ', "").eq_ignore_ascii_case(&kind.replace(' ', ""))),
                Objective::Social => met_all,
                _ => false,
            };
            if done {
                q.complete = true;
                changed = true;
            }
        }
        if changed {
            self.check_quests();
        }
    }

    pub(crate) fn quest_crafted(&mut self, item: &Item) {
        let Some(id) = item.object_id().or(item.big_id()) else { return };
        for q in self.farmer.quests.iter_mut() {
            if let Objective::Craft { item: want } = &q.objective {
                if want == id {
                    q.complete = true;
                }
            }
        }
        self.check_quests();
    }

    /// Try to hand the held item to `npc` for a delivery quest.
    pub(crate) fn try_quest_delivery(&mut self, npc: &str) -> bool {
        let slot = self.farmer.current_slot;
        let Some(held) = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from) else { return false };
        let have = self.farmer.inventory.get(slot).map(|i| i.stack()).unwrap_or(0);
        let Some(i) = self.farmer.quests.iter().position(|q| matches!(&q.objective, Objective::Deliver { npc: n, item, count } if n == npc && *item == held && have >= *count)) else {
            return false;
        };
        let Objective::Deliver { count, .. } = self.farmer.quests[i].objective.clone() else { return false };
        self.farmer.inventory.reduce(slot, count);
        let reaction = self.farmer.quests[i].reaction.clone();
        self.finish_quest(i);
        if !reaction.is_empty() {
            self.emit(WorldEvent::Dialogue { npc: Some(npc.to_string()), pages: vec![crate::dialogue::DialoguePage { text: reaction, emotion: crate::dialogue::Emotion::Happy }] });
        }
        true
    }

    /// Daily quest timers.
    pub(crate) fn tick_quest_days(&mut self) {
        for q in self.farmer.quests.iter_mut() {
            if q.days_left > 0 {
                q.days_left -= 1;
            }
        }
        let before = self.farmer.quests.len();
        self.farmer.quests.retain(|q| q.days_left != 0);
        if self.farmer.quests.len() < before {
            self.hud("A request expired.");
        }
    }
}

impl World {
    /// Upgrade cost: (money, bar item candidates, bars) for the next level.
    pub fn upgrade_cost(&self, level: i32) -> Option<(i32, Option<String>, i32)> {
        let (money, bars): (i32, &[&str]) = match level {
            0 => (2000, &["334", "Dev_CopperBar"]),
            1 => (5000, &["335", "Dev_IronBar"]),
            2 => (10000, &["336", "Dev_GoldBar"]),
            3 => (25000, &["337", "Dev_IridiumBar"]),
            _ => return None,
        };
        let bar = bars.iter().find(|b| self.items.objects.contains_key(**b)).map(|s| s.to_string());
        Some((money, bar, 5))
    }

    /// Hand a tool to the blacksmith for upgrading (2 days).
    pub fn start_tool_upgrade(&mut self, slot: usize) -> Result<(), String> {
        if self.farmer.tool_upgrade.is_some() {
            return Err("The blacksmith is already working on a tool.".into());
        }
        let Some(Item::Tool(tool)) = self.farmer.inventory.get(slot).cloned() else { return Err("That isn't a tool.".into()) };
        if matches!(tool.kind, crate::item::ToolKind::Scythe | crate::item::ToolKind::FishingRod | crate::item::ToolKind::MilkPail | crate::item::ToolKind::Shears) {
            return Err("That tool can't be upgraded here.".into());
        }
        let (money, bar, n) = self.upgrade_cost(tool.level).ok_or("That tool is already as good as it gets.")?;
        let bar = bar.ok_or("Upgrades need metal bars.")?;
        if self.farmer.money < money || self.farmer.inventory.count(&bar) < n {
            return Err(format!("Needs {money}g and {n} bars."));
        }
        self.farmer.money -= money;
        self.farmer.inventory.remove_object(&bar, n);
        self.farmer.inventory.take(slot);
        let mut up = tool.clone();
        up.level += 1;
        up.water = up.water_capacity();
        self.farmer.tool_upgrade = Some((up, 2));
        self.sound("purchase");
        Ok(())
    }

    /// Collect a finished upgrade.
    pub fn collect_tool_upgrade(&mut self) -> bool {
        match &self.farmer.tool_upgrade {
            Some((_, d)) if *d <= 0 => {
                let (tool, _) = self.farmer.tool_upgrade.take().unwrap();
                self.give_item(Item::Tool(tool));
                true
            }
            _ => false,
        }
    }

    /// What a geode holds: its `GeodeDrops` (by precedence, chance and
    /// condition), else a random gem, mineral or resource (approx. pool).
    pub fn geode_contents(&self, geode: &str, rng: &mut engine::Rng) -> Option<Item> {
        if let Some(info) = self.items.object(geode) {
            let mut drops = info.data.geode_drops.clone();
            drops.sort_by_key(|d| d.precedence);
            for d in drops {
                if !rng.chance(d.chance) || !self.check_query(d.condition.as_deref(), None, None, &mut rng.clone()) {
                    continue;
                }
                let id = match &d.random_item_id {
                    Some(ids) if !ids.is_empty() => rng.pick(ids).cloned(),
                    _ => d.item_id.clone(),
                };
                let n = if d.max_stack > d.min_stack { rng.range(d.min_stack.max(1), d.max_stack + 1) } else { d.min_stack.max(1) };
                if let Some(item) = id.and_then(|id| self.items.create(&id, n)) {
                    return Some(item);
                }
            }
            if !info.data.geode_drops.is_empty() && !info.data.geode_drops_default_items {
                return None;
            }
        }
        let pool: Vec<String> = self.items.objects.iter().filter(|(_, o)| matches!(o.data.category, -2 | -12 | -15) && o.data.price > 0).map(|(k, _)| k.clone()).collect();
        rng.pick(&pool).map(|id| Item::object(id, 1))
    }

    pub fn is_geode(&self, id: &str) -> bool {
        self.items.object(id).is_some_and(|o| o.data.name.to_ascii_lowercase().contains("geode")) || matches!(id, "535" | "536" | "537" | "749" | "Dev_Geode")
    }

    /// Crack the held geode: returns the item found.
    pub fn crack_geode(&mut self, slot: usize) -> Result<Item, String> {
        let Some(id) = self.farmer.inventory.get(slot).and_then(|i| i.object_id()).map(String::from) else { return Err("Hold a geode.".into()) };
        if self.items.object(&id).is_none() || !(self.is_geode(&id) || self.items.object(&id).is_some_and(|o| !o.data.geode_drops.is_empty())) {
            return Err("That isn't a geode.".into());
        }
        if self.farmer.money < 25 {
            return Err("Not enough money.".into());
        }
        let mut rng = engine::Rng::new(engine::Rng::seed_from(&[self.seed as i64, self.game_ms as i64, slot as i64, 9]));
        let item = self.geode_contents(&id, &mut rng).ok_or("This geode is empty.")?;
        self.farmer.money -= 25;
        self.farmer.inventory.reduce(slot, 1);
        self.give_item(item.clone());
        self.sound("stoneCrack");
        Ok(item)
    }
}
