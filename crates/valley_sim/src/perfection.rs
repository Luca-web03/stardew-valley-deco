//! A perfection score summarising overall progress. The categories and
//! weights are engine-defined (approximate), shown in the Collections tab.

use crate::farmer::Skill;
use crate::world::World;

/// (category, weight, fraction done 0..=1).
pub type PerfectionPart = (&'static str, f32, f32);

impl World {
    pub fn perfection_parts(&self) -> Vec<PerfectionPart> {
        let ratio = |done: usize, total: usize| if total == 0 { 1.0 } else { (done as f32 / total as f32).min(1.0) };
        let f = &self.farmer;
        let shippable: Vec<&String> = self.items.objects.iter().filter(|(_, o)| o.data.price > 0 && o.data.kind != "Litter" && o.data.category != -4).map(|(k, _)| k).collect();
        let shipped = shippable.iter().filter(|k| f.shipped.contains_key(**k)).count();
        let fish_total = self.fish.values().filter(|x| x.difficulty > 0).count();
        let fish_caught = self.fish.iter().filter(|(k, x)| x.difficulty > 0 && f.fish_caught.contains_key(*k)).count();
        let cooked = self.cooking.keys().filter(|k| f.cooking_recipes.get(*k).is_some_and(|n| *n > 0)).count();
        let crafted = self.recipes.keys().filter(|k| f.crafting_recipes.get(*k).is_some_and(|n| *n > 0)).count();
        let villagers = self.npcs.len();
        let close = self.npcs.iter().filter(|n| f.friendships.get(&n.name).is_some_and(|x| x.hearts() >= 8)).count();
        let skills = [Skill::Farming, Skill::Mining, Skill::Foraging, Skill::Fishing, Skill::Combat];
        let levels: i32 = skills.iter().map(|s| f.skills.level(*s).min(10)).sum();
        let areas: std::collections::BTreeSet<&String> = self.bundles.iter().map(|b| &b.area).collect();
        let museum_total = self.items.objects.values().filter(|o| matches!(o.data.kind.as_str(), "Arch" | "Minerals")).count();
        vec![
            ("Items shipped", 15.0, ratio(shipped, shippable.len())),
            ("Fish caught", 10.0, ratio(fish_caught, fish_total)),
            ("Dishes cooked", 10.0, ratio(cooked, self.cooking.len())),
            ("Items crafted", 10.0, ratio(crafted, self.recipes.len())),
            ("Close friends", 11.0, ratio(close, villagers)),
            ("Skill levels", 10.0, levels as f32 / 50.0),
            ("Museum", 10.0, ratio(f.bundles.museum.len(), museum_total)),
            ("Bundles", 24.0, ratio(f.bundles.areas_completed.len(), areas.len())),
        ]
    }

    /// Overall perfection in percent (0..=100).
    pub fn perfection(&self) -> f32 {
        let parts = self.perfection_parts();
        let total: f32 = parts.iter().map(|p| p.1).sum();
        parts.iter().map(|(_, w, d)| w * d).sum::<f32>() / total * 100.0
    }
}
