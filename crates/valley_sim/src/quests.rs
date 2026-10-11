//! Quests (`Data/Quests`, public slash format:
//! type/title/description/hint/objective/nextQuest/money/rewardText/cancellable/reaction)
//! plus generated "help wanted" delivery requests.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Objective {
    /// Bring `count` of `item` to `npc`.
    Deliver { npc: String, item: String, count: i32 },
    /// Harvest/collect `count` of `item` (counted on pickup).
    Harvest { item: String, count: i32, progress: i32 },
    /// Defeat `count` monsters named `monster`.
    Slay { monster: String, count: i32, progress: i32 },
    /// Visit a location.
    Visit { location: String },
    /// Craft an item.
    Craft { item: String },
    /// Have a building of this kind.
    Build { kind: String },
    /// Meet every villager.
    Social,
    /// No automatic objective.
    Basic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub hint: String,
    pub objective: Objective,
    pub reward_money: i32,
    pub next: Option<String>,
    pub complete: bool,
    /// Days until a timed quest expires (-1 = never).
    pub days_left: i32,
    pub reaction: String,
}

pub fn parse_quest(id: &str, s: &str) -> Option<Quest> {
    let f: Vec<&str> = s.split('/').collect();
    let kind = f.first()?.trim();
    let obj: Vec<&str> = f.get(4).unwrap_or(&"").split_whitespace().collect();
    let num = |i: usize| obj.get(i).and_then(|v| v.parse::<i32>().ok()).unwrap_or(1);
    let objective = match kind {
        "ItemDelivery" | "LostItem" | "SecretLostItem" if obj.len() >= 2 => Objective::Deliver { npc: obj[0].into(), item: obj[1].into(), count: num(2).max(1) },
        "ItemHarvest" if !obj.is_empty() => Objective::Harvest { item: obj[0].into(), count: num(1).max(1), progress: 0 },
        "Monster" if !obj.is_empty() => Objective::Slay { monster: obj[0].replace('_', " "), count: num(1).max(1), progress: 0 },
        "Location" if !obj.is_empty() => Objective::Visit { location: obj[0].into() },
        "Crafting" if !obj.is_empty() => Objective::Craft { item: obj[0].into() },
        "Building" if !obj.is_empty() => Objective::Build { kind: obj.join(" ") },
        "Social" => Objective::Social,
        _ => Objective::Basic,
    };
    Some(Quest {
        id: id.to_string(),
        title: f.get(1).unwrap_or(&"").to_string(),
        description: f.get(2).unwrap_or(&"").to_string(),
        hint: f.get(3).unwrap_or(&"").to_string(),
        objective,
        next: f.get(5).filter(|n| !n.is_empty() && **n != "-1").map(|s| s.to_string()),
        reward_money: f.get(6).and_then(|v| v.parse().ok()).unwrap_or(0).max(0),
        complete: false,
        days_left: -1,
        reaction: f.get(9).unwrap_or(&"").to_string(),
    })
}

impl Quest {
    pub fn progress_text(&self) -> Option<String> {
        match &self.objective {
            Objective::Harvest { count, progress, .. } | Objective::Slay { count, progress, .. } => Some(format!("{}/{}", progress.min(count), count)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quest_types() {
        let q = parse_quest("1", "ItemDelivery/Title/Desc/Hint/Mara X 3/-1/150//true/Thanks!").unwrap();
        assert_eq!(q.objective, Objective::Deliver { npc: "Mara".into(), item: "X".into(), count: 3 });
        assert_eq!(q.reward_money, 150);
        assert_eq!(q.next, None);
        let q = parse_quest("2", "Monster/T/D/H/Mud_Blob 5/3/0//false/").unwrap();
        assert_eq!(q.objective, Objective::Slay { monster: "Mud Blob".into(), count: 5, progress: 0 });
        assert_eq!(q.next.as_deref(), Some("3"));
    }
}
