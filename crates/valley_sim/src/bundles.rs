//! Community bundles (`Data/Bundles`, public format: key "Area/index",
//! value "name/reward/requirements/color/numRequired/displayName") and
//! museum donations (`Data/MuseumRewards`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::item::{Item, ItemDb, Quality};
use crate::world::{World, WorldEvent};

#[derive(Clone, Debug, PartialEq)]
pub struct Requirement {
    /// Object id, or "-1" for money.
    pub item_id: String,
    pub count: i32,
    pub min_quality: Quality,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BundleDef {
    pub key: String,
    pub area: String,
    pub name: String,
    pub reward: Option<Item>,
    pub reqs: Vec<Requirement>,
    pub num_required: usize,
}

impl BundleDef {
    pub fn is_money(&self) -> bool {
        self.reqs.len() == 1 && self.reqs[0].item_id == "-1"
    }
}

fn parse_reward(s: &str, db: &ItemDb) -> Option<Item> {
    let t: Vec<&str> = s.split_whitespace().collect();
    let (ty, id, n) = (t.first()?, t.get(1)?, t.get(2).and_then(|n| n.parse().ok()).unwrap_or(1));
    match *ty {
        "O" => db.create(id, n),
        "BO" => db.create(&format!("(BC){id}"), n),
        _ => db.create(&format!("({ty}){id}"), n),
    }
}

pub fn parse_bundles(raw: &std::collections::HashMap<String, String>, db: &ItemDb) -> Vec<BundleDef> {
    let mut out: Vec<BundleDef> = raw
        .iter()
        .filter_map(|(key, v)| {
            let area = key.split('/').next()?.to_string();
            let f: Vec<&str> = v.split('/').collect();
            let toks: Vec<&str> = f.get(2)?.split_whitespace().collect();
            let reqs: Vec<Requirement> = toks
                .chunks(3)
                .filter(|c| c.len() >= 2)
                .map(|c| Requirement {
                    item_id: c[0].to_string(),
                    count: c[1].parse().unwrap_or(1),
                    min_quality: Quality::from_level(c.get(2).and_then(|q| q.parse().ok()).unwrap_or(0)),
                })
                .collect();
            let num_required = f.get(4).and_then(|n| n.trim().parse().ok()).filter(|n: &usize| *n > 0).unwrap_or(reqs.len());
            let name = f.get(5).filter(|s| !s.is_empty()).or(f.first()).unwrap_or(&"").to_string();
            Some(BundleDef { key: key.clone(), area, name, reward: f.get(1).and_then(|r| parse_reward(r, db)), reqs, num_required })
        })
        .collect();
    out.sort_by(|a, b| a.area.cmp(&b.area).then_with(|| {
        let ai: i32 = a.key.split('/').nth(1).and_then(|n| n.parse().ok()).unwrap_or(0);
        let bi: i32 = b.key.split('/').nth(1).and_then(|n| n.parse().ok()).unwrap_or(0);
        ai.cmp(&bi)
    }));
    out
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BundleProgress {
    /// Per bundle key: which requirements are filled.
    pub filled: BTreeMap<String, Vec<bool>>,
    pub completed: Vec<String>,
    pub areas_completed: Vec<String>,
    /// Museum donations (object ids).
    pub museum: Vec<String>,
    pub museum_rewards: Vec<String>,
}

impl World {
    pub fn bundle_areas(&self) -> Vec<String> {
        let mut v: Vec<String> = self.bundles.iter().map(|b| b.area.clone()).collect();
        v.dedup();
        v
    }

    pub fn bundle_filled(&self, key: &str, i: usize) -> bool {
        self.farmer.bundles.filled.get(key).and_then(|v| v.get(i)).copied().unwrap_or(false)
    }

    /// Fill requirement `i` of bundle `key` from the inventory (or money).
    pub fn donate_to_bundle(&mut self, key: &str, i: usize) -> Result<(), String> {
        let b = self.bundles.iter().find(|b| b.key == key).cloned().ok_or("No such bundle")?;
        if self.farmer.bundles.completed.contains(&b.key) {
            return Err("Already complete.".into());
        }
        let req = b.reqs.get(i).cloned().ok_or("No such requirement")?;
        if self.bundle_filled(key, i) {
            return Err("Already donated.".into());
        }
        if req.item_id == "-1" {
            if self.farmer.money < req.count {
                return Err("Not enough money.".into());
            }
            self.farmer.money -= req.count;
        } else {
            let slot = (0..self.farmer.inventory.capacity()).find(|&s| match self.farmer.inventory.get(s) {
                Some(Item::Object { id, stack, quality, .. }) => *id == req.item_id && *stack >= req.count && *quality >= req.min_quality,
                _ => false,
            });
            let slot = slot.ok_or("You don't have that.")?;
            self.farmer.inventory.reduce(slot, req.count);
        }
        let n = b.reqs.len();
        let entry = self.farmer.bundles.filled.entry(key.to_string()).or_insert_with(|| vec![false; n]);
        entry[i] = true;
        let done = entry.iter().filter(|x| **x).count() >= b.num_required;
        self.sound("newArtifact");
        if done {
            self.farmer.bundles.completed.push(b.key.clone());
            if let Some(r) = &b.reward {
                self.give_item(r.clone());
            }
            self.emit(WorldEvent::Hud(format!("{} bundle complete!", b.name)));
            let area_done = self.bundles.iter().filter(|x| x.area == b.area).all(|x| self.farmer.bundles.completed.contains(&x.key));
            if area_done && !self.farmer.bundles.areas_completed.contains(&b.area) {
                self.farmer.bundles.areas_completed.push(b.area.clone());
                self.farmer.mail_received.push(format!("ccDone_{}", b.area));
                self.emit(WorldEvent::Message(format!("The {} has been restored!", b.area)));
            }
        }
        Ok(())
    }

    /// Items in the inventory that could go to the museum.
    pub fn museum_donatable(&self, item: &Item) -> bool {
        let Some(id) = item.object_id() else { return false };
        if self.farmer.bundles.museum.iter().any(|x| x == id) {
            return false;
        }
        self.items.object(id).is_some_and(|o| {
            let t = o.data.kind.to_ascii_lowercase();
            t == "minerals" || t == "arch"
        })
    }

    pub fn donate_to_museum(&mut self, slot: usize) -> Result<(), String> {
        let Some(item) = self.farmer.inventory.get(slot).cloned() else { return Err("Nothing there.".into()) };
        if !self.museum_donatable(&item) {
            return Err("The museum can't use that.".into());
        }
        let id = item.object_id().unwrap().to_string();
        self.farmer.inventory.reduce(slot, 1);
        self.farmer.bundles.museum.push(id);
        self.sound("newArtifact");
        self.check_museum_rewards();
        Ok(())
    }

    fn check_museum_rewards(&mut self) {
        let donated: Vec<Item> = self.farmer.bundles.museum.iter().map(|id| Item::object(id, 1)).collect();
        let rewards = self.museum_rewards.clone();
        for (rid, r) in rewards {
            if self.farmer.bundles.museum_rewards.contains(&rid) {
                continue;
            }
            let ok = r.targets.iter().all(|(tag, count)| {
                let n = donated.iter().filter(|it| tag == "" || self.items.matches_tags(it, &[tag.clone()])).count() as i32;
                n >= *count
            });
            if !ok {
                continue;
            }
            self.farmer.bundles.museum_rewards.push(rid.clone());
            if let Some(item) = r.item.as_ref().and_then(|i| self.items.create(i, r.count.max(1))) {
                self.give_item(item);
                self.emit(WorldEvent::Hud("The museum sent you a reward!".into()));
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MuseumReward {
    /// (context tag, count); "" tag = any donation.
    pub targets: Vec<(String, i32)>,
    pub item: Option<String>,
    pub count: i32,
}

pub fn parse_museum_rewards(v: &serde_json::Value) -> BTreeMap<String, MuseumReward> {
    let mut out = BTreeMap::new();
    let Some(obj) = v.as_object() else { return out };
    for (k, r) in obj {
        let targets = r
            .get("TargetContextTags")
            .and_then(|t| t.as_array())
            .map(|a| a.iter().map(|t| (t.get("Tag").and_then(|x| x.as_str()).unwrap_or("").to_string(), t.get("Count").and_then(|c| c.as_i64()).unwrap_or(1) as i32)).collect())
            .unwrap_or_default();
        out.insert(
            k.clone(),
            MuseumReward { targets, item: r.get("RewardItemId").and_then(|x| x.as_str()).map(String::from), count: r.get("RewardItemCount").and_then(|x| x.as_i64()).unwrap_or(1) as i32 },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundle_strings() {
        let mut raw = std::collections::HashMap::new();
        raw.insert("Garden/1".to_string(), "Greens/O X 2/A 1 0 B 2 1 C 1 0/0/2/Garden Greens".to_string());
        raw.insert("Garden/0".to_string(), "Fund/O Y 1/-1 500 500/4".to_string());
        let b = parse_bundles(&raw, &ItemDb::default());
        assert_eq!(b[0].key, "Garden/0");
        assert!(b[0].is_money());
        assert_eq!(b[1].reqs.len(), 3);
        assert_eq!(b[1].reqs[1].min_quality, Quality::Silver);
        assert_eq!(b[1].num_required, 2);
        assert_eq!(b[1].name, "Garden Greens");
    }
}
