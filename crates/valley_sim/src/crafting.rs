//! Crafting recipes (`Data/CraftingRecipes`, slash-delimited strings per the
//! public recipe format: "ingredients/unused/output/isBigCraftable/unlock/...").

use std::collections::BTreeMap;

use crate::farmer::{Farmer, Skill};
use crate::item::{Item, ItemDb};

#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    pub name: String,
    /// (object id or negative category, count)
    pub ingredients: Vec<(String, i32)>,
    pub output_id: String,
    pub output_count: i32,
    pub big: bool,
    pub unlock: String,
}

pub fn parse_recipe(name: &str, s: &str) -> Option<Recipe> {
    let f: Vec<&str> = s.split('/').collect();
    let ing: Vec<&str> = f.first()?.split_whitespace().collect();
    let ingredients = ing.chunks(2).filter_map(|c| Some((c.first()?.to_string(), c.get(1)?.parse().ok()?))).collect();
    let mut out = f.get(2)?.split_whitespace();
    let output_id = out.next()?.to_string();
    let output_count = out.next().and_then(|n| n.parse().ok()).unwrap_or(1);
    let big = f.get(3).is_some_and(|b| b.trim().eq_ignore_ascii_case("true"));
    let unlock = f.get(4).unwrap_or(&"").trim().to_string();
    Some(Recipe { name: name.to_string(), ingredients, output_id, output_count, big, unlock })
}

pub fn parse_all(raw: &std::collections::HashMap<String, String>) -> BTreeMap<String, Recipe> {
    raw.iter().filter_map(|(k, v)| parse_recipe(k, v).map(|r| (k.clone(), r))).collect()
}

impl Recipe {
    pub fn output(&self) -> Item {
        let (_, id) = crate::item::split_qualified(&self.output_id);
        if self.big { Item::big(id, self.output_count) } else { Item::object(id, self.output_count) }
    }

    /// Whether this recipe becomes known automatically for the farmer now.
    pub fn auto_unlocked(&self, f: &Farmer) -> bool {
        let t: Vec<&str> = self.unlock.split_whitespace().collect();
        match t.as_slice() {
            ["default"] => true,
            ["l", lvl] => lvl.parse::<i32>().is_ok_and(|l| f.skills.level(Skill::Farming) >= l),
            ["s", skill, lvl] => {
                let s = match skill.to_ascii_lowercase().as_str() {
                    "farming" => Skill::Farming,
                    "fishing" => Skill::Fishing,
                    "foraging" => Skill::Foraging,
                    "mining" => Skill::Mining,
                    "combat" => Skill::Combat,
                    _ => return false,
                };
                lvl.parse::<i32>().is_ok_and(|l| f.skills.level(s) >= l)
            }
            ["f", npc, hearts] => hearts.parse::<i32>().is_ok_and(|h| f.hearts(npc) >= h),
            _ => false,
        }
    }

    fn matches(db: &ItemDb, item: &Item, ing: &str) -> bool {
        match item {
            Item::Object { id, .. } => {
                if id == ing {
                    return true;
                }
                if let Ok(cat) = ing.parse::<i32>() {
                    if cat < 0 {
                        return db.object(id).is_some_and(|o| o.data.category == cat);
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn can_craft(&self, db: &ItemDb, f: &Farmer) -> bool {
        self.ingredients.iter().all(|(ing, n)| {
            let have: i32 = f.inventory.slots.iter().flatten().filter(|i| Self::matches(db, i, ing)).map(|i| i.stack()).sum();
            have >= *n
        })
    }

    /// Consume ingredients and return the output, or None if missing items.
    pub fn craft(&self, db: &ItemDb, f: &mut Farmer) -> Option<Item> {
        if !self.can_craft(db, f) {
            return None;
        }
        for (ing, n) in &self.ingredients {
            let mut left = *n;
            for i in 0..f.inventory.capacity() {
                if left <= 0 {
                    break;
                }
                let Some(item) = f.inventory.get(i) else { continue };
                if !Self::matches(db, item, ing) {
                    continue;
                }
                let take = item.stack().min(left);
                f.inventory.reduce(i, take);
                left -= take;
            }
        }
        Some(self.output())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recipe_strings() {
        let r = parse_recipe("Box", "A 7/Home/Box/true/default/").unwrap();
        assert_eq!(r.ingredients, vec![("A".to_string(), 7)]);
        assert!(r.big);
        assert_eq!(r.output_id, "Box");
        let r = parse_recipe("Lamp", "A 1 B 2/Home/Lamp 5/false/l 1/").unwrap();
        assert_eq!(r.output_count, 5);
        assert_eq!(r.ingredients.len(), 2);
        assert_eq!(r.unlock, "l 1");
    }
}
