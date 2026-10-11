//! Achievements. With `Data/Achievements` (public slash format:
//! "name^description^shownInCollections^prerequisite^hat") the original
//! ids and names are used, with conditions from the public achievement
//! list; without it, the engine's own milestones (`pets::ACHIEVEMENTS`)
//! apply. Conditions are checked each night.

use std::collections::BTreeMap;

use crate::farmer::Skill;
use crate::world::{World, WorldEvent};

#[derive(Clone, Debug, PartialEq)]
pub struct AchievementInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub shown: bool,
}

pub fn parse(raw: &BTreeMap<String, String>) -> Vec<AchievementInfo> {
    let mut v: Vec<AchievementInfo> = raw
        .iter()
        .map(|(id, s)| {
            let f: Vec<&str> = s.split('^').collect();
            AchievementInfo {
                id: id.clone(),
                name: f.first().unwrap_or(&"").to_string(),
                description: f.get(1).unwrap_or(&"").to_string(),
                shown: f.get(2).map(|x| x.trim() != "false").unwrap_or(true),
            }
        })
        .collect();
    v.sort_by_key(|a| a.id.parse::<i32>().unwrap_or(i32::MAX));
    v
}

impl World {
    /// The achievements to show: (id, name, description, earned).
    pub fn achievement_list(&self) -> Vec<(String, String, String, bool)> {
        let has = |id: &str| self.farmer.achievements.iter().any(|a| a == id);
        if self.achievement_data.is_empty() {
            crate::pets::ACHIEVEMENTS.iter().map(|a| (a.id.to_string(), a.name.to_string(), a.description.to_string(), has(a.id))).collect()
        } else {
            self.achievement_data.iter().filter(|a| a.shown || has(&a.id)).map(|a| (a.id.clone(), a.name.clone(), a.description.clone(), has(&a.id))).collect()
        }
    }

    /// Conditions for the original achievement ids (public list).
    fn original_checks(&self) -> Vec<(&'static str, bool)> {
        let f = &self.farmer;
        let parts = self.perfection_parts();
        let full = |name: &str| parts.iter().any(|(n, _, d)| *n == name && *d >= 1.0);
        let hearts: Vec<i32> = f.friendships.values().map(|x| x.hearts()).collect();
        let count_at = |h: i32| hearts.iter().filter(|x| **x >= h).count();
        let cooked = f.cooking_recipes.values().filter(|n| **n > 0).count();
        let crafted = f.crafting_recipes.values().filter(|n| **n > 0).count();
        let fish_kinds = f.fish_caught.len();
        let fish_total: i32 = f.fish_caught.values().map(|v| v.0).sum();
        let skills = [Skill::Farming, Skill::Mining, Skill::Foraging, Skill::Fishing, Skill::Combat];
        let tens = skills.iter().filter(|s| f.skills.level(**s) >= 10).count();
        let crops: Vec<String> = self.items.crops.values().map(|c| crate::item::split_qualified(&c.harvest_item_id).1.to_string()).collect();
        let shipped_crop = |id: &String| f.shipped.get(id).copied().unwrap_or(0);
        let poly = !crops.is_empty() && crops.iter().all(|c| shipped_crop(c) >= 15);
        let mono = crops.iter().any(|c| shipped_crop(c) >= 300);
        vec![
            ("0", f.total_earned >= 15_000),
            ("1", f.total_earned >= 50_000),
            ("2", f.total_earned >= 250_000),
            ("3", f.total_earned >= 1_000_000),
            ("4", f.total_earned >= 10_000_000),
            ("5", full("Museum")),
            ("6", count_at(5) >= 1),
            ("7", count_at(10) >= 1),
            ("8", count_at(10) >= 8),
            ("9", count_at(5) >= 4),
            ("10", count_at(5) >= 10),
            ("11", count_at(5) >= 20),
            ("12", cooked >= 10),
            ("13", cooked >= 25),
            ("14", full("Dishes cooked")),
            ("15", f.house_level >= 1),
            ("16", f.house_level >= 2),
            ("17", crafted >= 15),
            ("18", crafted >= 30),
            ("19", full("Items crafted")),
            ("20", fish_kinds >= 10),
            ("21", fish_kinds >= 24),
            ("22", full("Fish caught")),
            ("23", fish_total >= 100),
            ("24", f.bundles.museum.len() >= 40),
            ("25", f.quests_completed >= 10),
            ("26", f.quests_completed >= 40),
            ("27", poly),
            ("28", mono),
            ("29", f.spouse.is_some() && f.children.len() >= 2),
            ("30", tens >= 1),
            ("31", tens >= 5),
            ("34", full("Items shipped")),
        ]
    }

    /// Night: award achievements whose conditions are met.
    pub(crate) fn check_achievements(&mut self) {
        let checks: Vec<(String, bool)> = if self.achievement_data.is_empty() {
            self.engine_checks().into_iter().map(|(k, v)| (k.to_string(), v)).collect()
        } else {
            let known: Vec<&str> = self.achievement_data.iter().map(|a| a.id.as_str()).collect();
            self.original_checks().into_iter().filter(|(k, _)| known.contains(k)).map(|(k, v)| (k.to_string(), v)).collect()
        };
        for (id, ok) in checks {
            if ok && !self.farmer.achievements.iter().any(|a| *a == id) {
                self.farmer.achievements.push(id.clone());
                let name = self
                    .achievement_data
                    .iter()
                    .find(|a| a.id == id)
                    .map(|a| a.name.clone())
                    .or_else(|| crate::pets::ACHIEVEMENTS.iter().find(|a| a.id == id).map(|a| a.name.to_string()))
                    .unwrap_or(id.clone());
                self.sound("achievement");
                self.emit(WorldEvent::Hud(format!("Achievement: {name}")));
            }
        }
    }

    fn engine_checks(&self) -> Vec<(&'static str, bool)> {
        let f = &self.farmer;
        let shipped: i32 = f.shipped.values().sum();
        let fish: i32 = f.fish_caught.values().map(|v| v.0).sum();
        let cooked: i32 = f.cooking_recipes.values().sum();
        let crafted: i32 = f.crafting_recipes.values().sum();
        let best_hearts = f.friendships.values().map(|x| x.hearts()).max().unwrap_or(0);
        let won = |g: &str| f.counters.get(&format!("Arcade.{g}.Wins")).copied().unwrap_or(0) > 0;
        vec![
            ("earn_15k", f.total_earned >= 15_000),
            ("earn_100k", f.total_earned >= 100_000),
            ("ship_100", shipped >= 100),
            ("fish_10", fish >= 10),
            ("cook_10", cooked >= 10),
            ("craft_15", crafted >= 15),
            ("mine_40", f.deepest_mine_level >= 40),
            ("slay_100", self.monsters_killed >= 100),
            ("friend_5", best_hearts >= 5),
            ("married", f.spouse.is_some()),
            ("arcade_ranger", won("ranger")),
            ("arcade_rails", won("rails")),
        ]
    }
}
