//! Monster eradication goals (`Data/MonsterSlayerQuests`, public format):
//! kills of the listed `Targets` (monster names) count toward `Count`; the
//! guild master hands over `RewardItemId` with `RewardDialogue` once a goal
//! is met, sets `RewardFlag`/`RewardFlagAll`, sends `RewardMail`/
//! `RewardMailAll` tomorrow, and from then on the guild shop sells the
//! reward for `RewardItemPrice` (unless -1). Tile actions: `Port.KillList`
//! (the goals board) and `Port.GuildMaster` (claim rewards). Goals are
//! listed in key order.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::world::{World, WorldEvent};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MonsterGoal {
    pub display_name: Option<String>,
    pub targets: Vec<String>,
    pub count: i32,
    pub reward_item_id: Option<String>,
    #[serde(default = "neg1")]
    pub reward_item_price: i32,
    pub reward_dialogue: Option<String>,
    pub reward_dialogue_flag: Option<String>,
    pub reward_flag: Option<String>,
    pub reward_flag_all: Option<String>,
    pub reward_mail: Option<String>,
    pub reward_mail_all: Option<String>,
}

fn neg1() -> i32 {
    -1
}

pub fn load(cm: &mut valley_content::ContentManager) -> BTreeMap<String, MonsterGoal> {
    cm.load_dict("Data/MonsterSlayerQuests")
}

/// The shop reward items are added to once claimed.
pub const GUILD_SHOP: &str = "AdventureShop";

impl World {
    /// Kills counted toward a goal.
    pub fn goal_kills(&self, goal: &MonsterGoal) -> i32 {
        goal.targets.iter().map(|t| self.farmer.monster_kills.get(t).copied().unwrap_or(0)).sum()
    }

    pub fn goal_complete(&self, id: &str) -> bool {
        self.monster_goals.get(id).is_some_and(|g| self.goal_kills(g) >= g.count.max(1))
    }

    fn goal_claimed(&self, id: &str, goal: &MonsterGoal) -> bool {
        let flag = goal.reward_flag.clone().unwrap_or_else(|| format!("Gil_{id}"));
        self.farmer.mail_received.contains(&flag)
    }

    /// Count a kill for the goals (by monster name).
    pub(crate) fn count_monster_kill(&mut self, name: &str) {
        *self.farmer.monster_kills.entry(name.to_string()).or_insert(0) += 1;
        let done: Vec<String> = self
            .monster_goals
            .iter()
            .filter(|(id, g)| g.targets.iter().any(|t| t == name) && self.goal_kills(g) == g.count.max(1) && !self.goal_claimed(id, g))
            .map(|(id, _)| id.clone())
            .collect();
        for id in done {
            let name = self.goal_name(&id);
            self.hud(format!("Monster eradication goal complete: {name}. See the guild."));
            self.sound("achievement");
        }
    }

    pub fn goal_name(&self, id: &str) -> String {
        self.monster_goals.get(id).and_then(|g| g.display_name.clone()).map(|n| self.items.expand_text(&n)).unwrap_or_else(|| id.to_string())
    }

    /// Lines for the goals board: (name, kills, goal, done).
    pub fn goal_lines(&self) -> Vec<(String, i32, i32, bool)> {
        self.monster_goals
            .iter()
            .map(|(id, g)| {
                let k = self.goal_kills(g);
                (self.goal_name(id), k.min(g.count), g.count, k >= g.count.max(1))
            })
            .collect()
    }

    /// The goals board.
    pub(crate) fn show_kill_list(&mut self) -> bool {
        if self.monster_goals.is_empty() {
            self.emit(WorldEvent::Message("Monster Eradication Goals: none posted.".into()));
            return true;
        }
        let mut text = String::from("Monster Eradication Goals^^");
        for (name, k, n, done) in self.goal_lines() {
            text.push_str(&format!("{}{name}: {k}/{n}^", if done { "* " } else { "" }));
        }
        let letter = crate::mail::Letter { text, title: Some("Monster Eradication Goals".into()), ..Default::default() };
        self.emit(WorldEvent::Letter { id: "Port.KillList".into(), letter });
        true
    }

    /// The guild master: hand over the rewards for completed goals.
    pub(crate) fn claim_goal_rewards(&mut self) -> bool {
        let ready: Vec<(String, MonsterGoal)> = self.monster_goals.iter().filter(|(id, g)| self.goal_kills(g) >= g.count.max(1) && !self.goal_claimed(id, g)).map(|(id, g)| (id.clone(), g.clone())).collect();
        let Some((id, goal)) = ready.into_iter().next() else {
            let text = if self.monster_goals.keys().all(|id| self.goal_complete(id)) && !self.monster_goals.is_empty() {
                "You've done it all. The caves are a little quieter thanks to you."
            } else {
                "Keep at it. The board says what's left to hunt."
            };
            self.emit(WorldEvent::Dialogue { npc: Some("Gil".into()), pages: vec![crate::dialogue::DialoguePage { text: text.into(), emotion: crate::dialogue::Emotion::Neutral }] });
            return true;
        };
        let flag = goal.reward_flag.clone().unwrap_or_else(|| format!("Gil_{id}"));
        for f in [Some(flag), goal.reward_flag_all.clone(), goal.reward_dialogue_flag.clone()].into_iter().flatten() {
            if !self.farmer.mail_received.contains(&f) {
                self.farmer.mail_received.push(f);
            }
        }
        for m in [&goal.reward_mail, &goal.reward_mail_all].into_iter().flatten() {
            if !self.farmer.mail_tomorrow.contains(m) {
                self.farmer.mail_tomorrow.push(m.clone());
            }
        }
        let text = goal.reward_dialogue.as_deref().map(|d| self.items.expand_text(d)).unwrap_or_else(|| format!("You've cleared the {} goal. Take this, you've earned it.", self.goal_name(&id)));
        self.emit(WorldEvent::Dialogue { npc: Some("Gil".into()), pages: vec![crate::dialogue::DialoguePage { text, emotion: crate::dialogue::Emotion::Happy }] });
        if let Some(item) = goal.reward_item_id.as_deref().and_then(|i| self.items.create(i, 1)) {
            self.give_item(item);
        }
        self.sound("reward");
        true
    }

    /// Claimed rewards the guild shop now sells: (item id, price).
    pub fn goal_shop_items(&self) -> Vec<(String, i32)> {
        self.monster_goals
            .iter()
            .filter(|(id, g)| g.reward_item_price >= 0 && g.reward_item_id.is_some() && self.goal_claimed(id, g))
            .map(|(_, g)| (g.reward_item_id.clone().unwrap(), g.reward_item_price))
            .collect()
    }
}
