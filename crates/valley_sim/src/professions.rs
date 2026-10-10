//! Skill professions (chosen at levels 5 and 10) with their publicly
//! documented effects. Effects not yet modelled are recorded but inert.

use crate::farmer::Skill;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profession {
    pub id: &'static str,
    pub name: &'static str,
    pub skill: Skill,
    pub level: i32,
    /// For level-10 professions: the level-5 profession it follows.
    pub parent: Option<&'static str>,
    pub description: &'static str,
}

pub const PROFESSIONS: &[Profession] = &[
    Profession { id: "rancher", name: "Rancher", skill: Skill::Farming, level: 5, parent: None, description: "Animal products worth 20% more." },
    Profession { id: "tiller", name: "Tiller", skill: Skill::Farming, level: 5, parent: None, description: "Crops worth 10% more." },
    Profession { id: "coopmaster", name: "Coopmaster", skill: Skill::Farming, level: 10, parent: Some("rancher"), description: "Befriend coop animals quicker." },
    Profession { id: "shepherd", name: "Shepherd", skill: Skill::Farming, level: 10, parent: Some("rancher"), description: "Befriend barn animals quicker." },
    Profession { id: "artisan", name: "Artisan", skill: Skill::Farming, level: 10, parent: Some("tiller"), description: "Artisan goods worth 40% more." },
    Profession { id: "agriculturist", name: "Agriculturist", skill: Skill::Farming, level: 10, parent: Some("tiller"), description: "All crops grow 10% faster." },
    Profession { id: "fisher", name: "Fisher", skill: Skill::Fishing, level: 5, parent: None, description: "Fish worth 25% more." },
    Profession { id: "trapper", name: "Trapper", skill: Skill::Fishing, level: 5, parent: None, description: "Resources required to craft traps are reduced." },
    Profession { id: "angler", name: "Angler", skill: Skill::Fishing, level: 10, parent: Some("fisher"), description: "Fish worth 50% more." },
    Profession { id: "pirate", name: "Pirate", skill: Skill::Fishing, level: 10, parent: Some("fisher"), description: "Chance to find treasure doubled." },
    Profession { id: "mariner", name: "Mariner", skill: Skill::Fishing, level: 10, parent: Some("trapper"), description: "Traps never catch trash." },
    Profession { id: "luremaster", name: "Luremaster", skill: Skill::Fishing, level: 10, parent: Some("trapper"), description: "Traps no longer require bait." },
    Profession { id: "forester", name: "Forester", skill: Skill::Foraging, level: 5, parent: None, description: "Gain 25% more wood when chopping." },
    Profession { id: "gatherer", name: "Gatherer", skill: Skill::Foraging, level: 5, parent: None, description: "Chance for double harvest of forage." },
    Profession { id: "lumberjack", name: "Lumberjack", skill: Skill::Foraging, level: 10, parent: Some("forester"), description: "Felled trees sometimes drop hardwood." },
    Profession { id: "tapper", name: "Tapper", skill: Skill::Foraging, level: 10, parent: Some("forester"), description: "Syrups worth 25% more." },
    Profession { id: "botanist", name: "Botanist", skill: Skill::Foraging, level: 10, parent: Some("gatherer"), description: "Forage is always the best quality." },
    Profession { id: "tracker", name: "Tracker", skill: Skill::Foraging, level: 10, parent: Some("gatherer"), description: "Forage locations are revealed." },
    Profession { id: "miner", name: "Miner", skill: Skill::Mining, level: 5, parent: None, description: "+1 ore per vein." },
    Profession { id: "geologist", name: "Geologist", skill: Skill::Mining, level: 5, parent: None, description: "Chance for gems to appear in pairs." },
    Profession { id: "blacksmith", name: "Blacksmith", skill: Skill::Mining, level: 10, parent: Some("miner"), description: "Metal bars worth 50% more." },
    Profession { id: "prospector", name: "Prospector", skill: Skill::Mining, level: 10, parent: Some("miner"), description: "Chance to find coal doubled." },
    Profession { id: "excavator", name: "Excavator", skill: Skill::Mining, level: 10, parent: Some("geologist"), description: "Chance to find geodes doubled." },
    Profession { id: "gemologist", name: "Gemologist", skill: Skill::Mining, level: 10, parent: Some("geologist"), description: "Gems worth 30% more." },
    Profession { id: "fighter", name: "Fighter", skill: Skill::Combat, level: 5, parent: None, description: "Attacks deal 10% more damage. +15 HP." },
    Profession { id: "scout", name: "Scout", skill: Skill::Combat, level: 5, parent: None, description: "Critical strike chance increased by 50%." },
    Profession { id: "brute", name: "Brute", skill: Skill::Combat, level: 10, parent: Some("fighter"), description: "Deal 15% more damage." },
    Profession { id: "defender", name: "Defender", skill: Skill::Combat, level: 10, parent: Some("fighter"), description: "+25 HP." },
    Profession { id: "acrobat", name: "Acrobat", skill: Skill::Combat, level: 10, parent: Some("scout"), description: "Special move cooldowns halved." },
    Profession { id: "desperado", name: "Desperado", skill: Skill::Combat, level: 10, parent: Some("scout"), description: "Critical strikes are deadly." },
];

pub fn get(id: &str) -> Option<&'static Profession> {
    PROFESSIONS.iter().find(|p| p.id == id)
}

/// The two choices offered when reaching `level` in `skill`.
pub fn choices(skill: Skill, level: i32, owned: &[String]) -> Vec<&'static Profession> {
    PROFESSIONS
        .iter()
        .filter(|p| p.skill == skill && p.level == level)
        .filter(|p| match p.parent {
            None => true,
            Some(parent) => owned.iter().any(|o| o == parent),
        })
        .collect()
}

/// Sell price multiplier for an object category given owned professions.
pub fn price_multiplier(owned: &[String], category: i32) -> f32 {
    let has = |id: &str| owned.iter().any(|o| o == id);
    let mut m = 1.0;
    match category {
        -75 | -79 | -80 if has("tiller") => m *= 1.1,
        _ => {}
    }
    if (category == -5 || category == -6 || category == -18) && has("rancher") {
        m *= 1.2;
    }
    if category == -26 && has("artisan") {
        m *= 1.4;
    }
    if category == -4 {
        if has("angler") {
            m *= 1.5;
        } else if has("fisher") {
            m *= 1.25;
        }
    }
    if category == -27 && has("tapper") {
        m *= 1.25;
    }
    if category == -2 && has("gemologist") {
        m *= 1.3;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ten_choices_follow_parent() {
        assert_eq!(choices(Skill::Farming, 5, &[]).len(), 2);
        let c = choices(Skill::Farming, 10, &["tiller".to_string()]);
        assert_eq!(c.iter().map(|p| p.id).collect::<Vec<_>>(), vec!["artisan", "agriculturist"]);
        assert!((price_multiplier(&["tiller".into()], -75) - 1.1).abs() < 1e-6);
        assert!((price_multiplier(&["artisan".into()], -26) - 1.4).abs() < 1e-6);
    }
}
