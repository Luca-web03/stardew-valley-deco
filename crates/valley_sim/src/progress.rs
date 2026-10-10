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
        let Some(item) = r.craft(&items, &mut self.farmer) else {
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
        let npc = rng.pick(&self.npcs).map(|n| (n.name.clone(), n.display_name.clone())).unwrap();
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
