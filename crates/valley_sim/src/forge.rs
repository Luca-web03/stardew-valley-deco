//! The forge (public behaviour; shard costs and effect sizes are approx.):
//! an item on the left is combined with an ingredient on the right for
//! cinder shards.
//!
//! - weapon + gem: up to three gem forges (ruby: damage, aquamarine: crit
//!   chance, jade: crit power, amethyst: knockback, topaz: defense,
//!   emerald: speed); a diamond fills the free forges with random gems;
//! - weapon or tool + prismatic shard: a random enchantment that fits it
//!   (rerolled when it already has one);
//! - weapon + dragon tooth: a random innate enchantment;
//! - weapon + weapon of the same type: the left one takes the right one's
//!   look;
//! - ring + ring: a combined ring with both effects;
//! - galaxy weapon + galaxy soul: three souls turn it into its infinity
//!   counterpart.
//!
//! Unforging strips forges and enchantments (refunding half the shards) or
//! splits a combined ring. Gems and ingredients are recognised by their
//! well-known ids or a `Port.Forge` custom field naming the kind.

use engine::Rng;

use crate::item::{Item, ToolKind, WeaponMods};
use crate::world::{World, WorldEvent};

/// Shards for the 1st, 2nd and 3rd gem forge.
pub const GEM_COSTS: [i32; 3] = [10, 15, 20];
pub const ENCHANT_COST: i32 = 20;
pub const INNATE_COST: i32 = 10;
pub const APPEARANCE_COST: i32 = 10;
pub const RING_COST: i32 = 20;
pub const SOUL_COST: i32 = 20;
pub const SOULS_NEEDED: i32 = 3;
pub const MAX_GEMS: usize = 3;

pub const GEM_KINDS: &[&str] = &["Emerald", "Aquamarine", "Ruby", "Amethyst", "Topaz", "Jade"];
pub const WEAPON_ENCHANTS: &[&str] = &["Artful", "BugKiller", "Crusader", "Haymaker", "Vampiric"];
pub const INNATE_KINDS: &[&str] = &["Attack", "Crit", "CritPower", "Defense", "Speed", "Weight", "SlimeSlayer", "SlimeGatherer"];

/// Chance a shavings axe chips off a piece of wood per hit (approx.).
pub const SHAVINGS_CHANCE: f64 = 0.25;
/// Chance a vampiric weapon heals on a kill (approx.).
pub const VAMPIRIC_CHANCE: f64 = 0.09;

/// Monster families that enchantments care about (by name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonsterClass {
    Bug,
    Undead,
    Slime,
    Other,
}

pub fn monster_class(name: &str) -> MonsterClass {
    let n = name.to_ascii_lowercase();
    let has = |k: &str| n.contains(k);
    if has("bug") || has("fly") || has("grub") || has("moth") || has("mosquito") {
        MonsterClass::Bug
    } else if has("mummy") || has("skeleton") || has("ghost") {
        MonsterClass::Undead
    } else if has("slime") || has("blob") || has("jelly") {
        MonsterClass::Slime
    } else {
        MonsterClass::Other
    }
}

/// Galaxy weapon -> infinity weapon (well-known ids).
const INFINITY: &[(&str, &str)] = &[("4", "62"), ("23", "64"), ("29", "63")];

/// What an ingredient does at the forge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ingredient {
    Gem(String),
    Diamond,
    Prismatic,
    DragonTooth,
    GalaxySoul,
}

fn well_known(id: &str) -> Option<Ingredient> {
    Some(match id {
        "60" => Ingredient::Gem("Emerald".into()),
        "62" => Ingredient::Gem("Aquamarine".into()),
        "64" => Ingredient::Gem("Ruby".into()),
        "66" => Ingredient::Gem("Amethyst".into()),
        "68" => Ingredient::Gem("Topaz".into()),
        "70" => Ingredient::Gem("Jade".into()),
        "72" => Ingredient::Diamond,
        "74" => Ingredient::Prismatic,
        "852" => Ingredient::DragonTooth,
        "896" => Ingredient::GalaxySoul,
        _ => return None,
    })
}

/// Enchantments a tool can take.
pub fn tool_enchants(kind: ToolKind, level: i32) -> Vec<&'static str> {
    let mut v = match kind {
        ToolKind::Axe => vec!["Efficient", "Powerful", "Shavings", "Swift"],
        ToolKind::Pickaxe => vec!["Efficient", "Powerful", "Swift"],
        ToolKind::Hoe => vec!["Archaeologist", "Efficient", "Generous", "Swift"],
        ToolKind::WateringCan => vec!["Bottomless", "Efficient"],
        ToolKind::FishingRod => vec!["AutoHook", "Master", "Preserving"],
        _ => vec![],
    };
    if level >= 4 && matches!(kind, ToolKind::Hoe | ToolKind::WateringCan) {
        v.push("Reaching");
    }
    v
}

/// Display name of an enchantment.
pub fn enchantment_title(e: &str) -> Option<&'static str> {
    Some(match e {
        "Artful" => "Artful",
        "BugKiller" => "Bug Killer",
        "Crusader" => "Crusader",
        "Haymaker" => "Haymaker",
        "Vampiric" => "Vampiric",
        "Efficient" => "Efficient",
        "Powerful" => "Powerful",
        "Shavings" => "Shavings",
        "Swift" => "Swift",
        "Archaeologist" => "Archaeologist",
        "Generous" => "Generous",
        "Bottomless" => "Bottomless",
        "Reaching" => "Reaching",
        "AutoHook" => "Auto-Hook",
        "Master" => "Master",
        "Preserving" => "Preserving",
        "Attack" => "Attack",
        "Crit" => "Crit. Chance",
        "CritPower" => "Crit. Power",
        "Defense" => "Defense",
        "Speed" => "Speed",
        "Weight" => "Weight",
        "SlimeSlayer" => "Slime Slayer",
        "SlimeGatherer" => "Slime Gatherer",
        _ => return None,
    })
}

/// A weapon's effective stats with its forges applied.
#[derive(Clone, Debug, PartialEq)]
pub struct WeaponStats {
    pub min: i32,
    pub max: i32,
    pub knockback: f32,
    pub speed: i32,
    pub crit: f32,
    pub crit_mult: f32,
    pub defense: i32,
    pub aoe: i32,
    pub kind: i32,
}

/// Effect of `n` levels of each forge kind (approx. sizes).
pub fn apply_mods(s: &mut WeaponStats, m: &WeaponMods) {
    let add = |kind: &str, n: i32, s: &mut WeaponStats| {
        let f = n as f32;
        match kind {
            "Ruby" | "Attack" => {
                let bonus = |v: i32| (v as f32 * 0.1 * f).round().max(f) as i32;
                s.min += bonus(s.min);
                s.max += bonus(s.max);
            }
            "Aquamarine" | "Crit" => s.crit += 0.046 * f,
            "Jade" | "CritPower" => s.crit_mult += 0.1 * f,
            "Amethyst" | "Weight" => s.knockback += 0.1 * f,
            "Topaz" | "Defense" => s.defense += n,
            "Emerald" | "Speed" => s.speed += n,
            _ => {}
        }
    };
    for g in &m.gems {
        add(g, 1, s);
    }
    for (k, n) in &m.innate {
        add(k, *n, s);
    }
}

impl World {
    pub fn forge_ingredient(&self, item: &Item) -> Option<Ingredient> {
        let id = item.object_id()?;
        let custom = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.Forge")).cloned();
        match custom.as_deref() {
            Some("Diamond") => Some(Ingredient::Diamond),
            Some("Prismatic") => Some(Ingredient::Prismatic),
            Some("DragonTooth") => Some(Ingredient::DragonTooth),
            Some("GalaxySoul") => Some(Ingredient::GalaxySoul),
            Some(g) if GEM_KINDS.contains(&g) => Some(Ingredient::Gem(g.to_string())),
            _ => well_known(id).filter(|_| self.items.objects.contains_key(id)),
        }
    }

    pub fn weapon_stats(&self, item: &Item) -> Option<WeaponStats> {
        let Item::Weapon { id, mods } = item else { return None };
        let w = self.items.weapons.get(id)?;
        let legacy = self.farmer.weapon_forge.get(id).copied().unwrap_or(0) * crate::volcano::FORGE_BONUS;
        let mut s = WeaponStats {
            min: w.min_damage + legacy,
            max: w.max_damage + legacy,
            knockback: w.knockback,
            speed: w.speed,
            crit: w.crit_chance,
            crit_mult: w.crit_multiplier,
            defense: w.defense,
            aoe: w.area_of_effect,
            kind: w.kind,
        };
        if let Some(m) = mods {
            apply_mods(&mut s, m);
        }
        Some(s)
    }

    /// The held weapon's forge state.
    pub fn held_weapon_mods(&self) -> Option<&WeaponMods> {
        self.farmer.current_item()?.weapon_mods()
    }

    /// Does the held weapon carry this enchantment (prismatic or innate)?
    pub fn held_weapon_has(&self, e: &str) -> bool {
        self.held_weapon_mods().is_some_and(|m| m.enchantment.as_deref() == Some(e) || m.innate.contains_key(e))
    }

    /// Enchantment on the tool in `slot`.
    pub fn tool_enchantment(&self, slot: usize) -> Option<&str> {
        self.farmer.inventory.get(slot)?.tool()?.enchantment.as_deref()
    }

    pub fn tool_has(&self, slot: usize, e: &str) -> bool {
        self.tool_enchantment(slot) == Some(e)
    }

    /// Is this enchantment on the farmer's fishing rod (current item)?
    pub fn rod_has(&self, e: &str) -> bool {
        self.tool_has(self.farmer.current_slot, e)
    }

    fn forge_rng(&self) -> Rng {
        let n = self.farmer.counters.get("Forge.Count").copied().unwrap_or(0);
        Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, n as i64, 919]))
    }

    fn shard_count(&self) -> i32 {
        self.shard_item().map(|s| self.farmer.inventory.count(s)).unwrap_or(0)
    }

    /// What forging `left` with `right` makes, and its shard cost.
    pub fn forge_preview(&self, left: &Item, right: &Item) -> Result<(Item, i32), String> {
        let mut rng = self.forge_rng();
        self.forge_result(left, right, &mut rng)
    }

    fn forge_result(&self, left: &Item, right: &Item, rng: &mut Rng) -> Result<(Item, i32), String> {
        // Ring + ring.
        if self.is_ring(left) && self.is_ring(right) {
            let parts = |r: &Item| -> Vec<String> {
                let c = r.extra().map(|e| e.combined.clone()).unwrap_or_default();
                if c.is_empty() {
                    vec![r.object_id().unwrap_or("").to_string()]
                } else {
                    c
                }
            };
            if parts(left).len() > 1 || parts(right).len() > 1 {
                return Err("Combined rings can't be combined again.".into());
            }
            let (a, b) = (parts(left).remove(0), parts(right).remove(0));
            if a == b {
                return Err("Those rings are the same.".into());
            }
            let id = if self.items.objects.contains_key("880") { "880".to_string() } else { a.clone() };
            let na = self.items.display_name(&Item::object(&a, 1));
            let nb = self.items.display_name(&Item::object(&b, 1));
            let mut out = Item::object(&id, 1);
            if let Item::Object { extra, .. } = &mut out {
                *extra = Some(Box::new(crate::item::ItemExtra { display_name: Some(format!("Combined Ring ({na} + {nb})")), combined: vec![a, b], ..Default::default() }));
            }
            return Ok((out, RING_COST));
        }
        // Tool + prismatic shard.
        if let Item::Tool(t) = left {
            if self.forge_ingredient(right) != Some(Ingredient::Prismatic) {
                return Err("Tools take a prismatic shard.".into());
            }
            let options: Vec<&str> = tool_enchants(t.kind, t.level).into_iter().filter(|e| t.enchantment.as_deref() != Some(*e)).collect();
            let e = rng.pick(&options).copied().ok_or("This tool can't be enchanted.")?;
            let mut out = left.clone();
            if let Some(tm) = out.tool_mut() {
                tm.enchantment = Some(e.to_string());
            }
            return Ok((out, ENCHANT_COST));
        }
        let Item::Weapon { id, mods } = left else { return Err("That can't be forged.".into()) };
        let mods = mods.as_deref().cloned().unwrap_or_default();
        let wkind = self.items.weapons.get(id).map(|w| w.kind).unwrap_or(-1);
        if wkind == crate::arsenal::TYPE_SLINGSHOT {
            return Err("That can't be forged.".into());
        }
        let with = |m: WeaponMods| Item::Weapon { id: id.clone(), mods: (!m.is_empty()).then(|| Box::new(m)) };
        // Weapon + weapon: transmogrify.
        if let Item::Weapon { id: other, .. } = right {
            let okind = self.items.weapons.get(other).map(|w| w.kind).unwrap_or(-2);
            if okind != wkind || other == id {
                return Err("The weapons must be of the same type.".into());
            }
            let mut m = mods;
            m.appearance = Some(other.clone());
            return Ok((with(m), APPEARANCE_COST));
        }
        let ing = self.forge_ingredient(right).ok_or("That can't be used here.")?;
        let mut m = mods;
        match ing {
            Ingredient::Gem(g) => {
                let n = m.gems.len();
                let cost = *GEM_COSTS.get(n).ok_or("This weapon can't be forged any further.")?;
                m.gems.push(g);
                Ok((with(m), cost))
            }
            Ingredient::Diamond => {
                let n = m.gems.len();
                let cost = *GEM_COSTS.get(n).ok_or("This weapon can't be forged any further.")?;
                while m.gems.len() < MAX_GEMS {
                    m.gems.push(rng.pick(GEM_KINDS).unwrap().to_string());
                }
                Ok((with(m), cost))
            }
            Ingredient::Prismatic => {
                let options: Vec<&&str> = WEAPON_ENCHANTS.iter().filter(|e| m.enchantment.as_deref() != Some(**e)).collect();
                m.enchantment = Some(rng.pick(&options).unwrap().to_string());
                Ok((with(m), ENCHANT_COST))
            }
            Ingredient::DragonTooth => {
                // Replaces the innate enchantment with a fresh random one.
                m.innate.clear();
                let k = rng.pick(INNATE_KINDS).unwrap().to_string();
                let level = if matches!(k.as_str(), "SlimeSlayer" | "SlimeGatherer") { 1 } else { rng.range(1, 4) };
                m.innate.insert(k, level);
                Ok((with(m), INNATE_COST))
            }
            Ingredient::GalaxySoul => {
                let target = INFINITY.iter().find(|(g, _)| g == id).map(|(_, i)| *i).filter(|i| self.items.weapons.contains_key(*i));
                let Some(target) = target else { return Err("Only galaxy weapons take a galaxy soul.".into()) };
                m.souls += 1;
                if m.souls >= SOULS_NEEDED {
                    m.souls = 0;
                    return Ok((Item::Weapon { id: target.to_string(), mods: (!m.is_empty()).then(|| Box::new(m)) }, SOUL_COST));
                }
                Ok((with(m), SOUL_COST))
            }
        }
    }

    /// Forge inventory slots `left` and `right`; the result replaces the
    /// left item.
    pub fn forge(&mut self, left: usize, right: usize) -> Result<Item, String> {
        if left == right {
            return Err("Pick two different items.".into());
        }
        let (Some(l), Some(r)) = (self.farmer.inventory.get(left).cloned(), self.farmer.inventory.get(right).cloned()) else {
            return Err("Pick two items.".into());
        };
        let mut rng = self.forge_rng();
        let (out, cost) = self.forge_result(&l, &r, &mut rng)?;
        let shard = self.shard_item().ok_or("There's nothing to forge with.")?;
        if self.shard_count() < cost {
            return Err(format!("You need {cost} cinder shards."));
        }
        self.farmer.inventory.remove_object(shard, cost);
        self.farmer.inventory.reduce(right, 1);
        if self.is_ring(&l) {
            self.farmer.inventory.reduce(left, 1);
            self.give_item(out.clone());
        } else {
            self.farmer.inventory.slots[left] = Some(out.clone());
        }
        // The next forge rolls afresh.
        *self.farmer.counters.entry("Forge.Count".into()).or_insert(0) += 1;
        self.sound("crit");
        Ok(out)
    }

    /// Strip an item's forges and enchantments (half the shards back), or
    /// split a combined ring.
    pub fn unforge(&mut self, slot: usize) -> Result<(), String> {
        let item = self.farmer.inventory.get(slot).cloned().ok_or("Pick an item.")?;
        let shard = self.shard_item();
        let mut refund = 0;
        match &item {
            Item::Weapon { id, mods: Some(m) } => {
                refund += m.gems.iter().enumerate().map(|(i, _)| GEM_COSTS[i.min(2)]).sum::<i32>();
                if m.enchantment.is_some() {
                    refund += ENCHANT_COST;
                }
                if !m.innate.is_empty() {
                    refund += INNATE_COST;
                }
                if m.appearance.is_some() {
                    refund += APPEARANCE_COST;
                }
                refund += m.souls * SOUL_COST;
                self.farmer.inventory.slots[slot] = Some(Item::weapon(id));
            }
            Item::Tool(t) if t.enchantment.is_some() => {
                refund += ENCHANT_COST;
                if let Some(tm) = self.farmer.inventory.slots[slot].as_mut().and_then(|i| i.tool_mut()) {
                    tm.enchantment = None;
                }
            }
            Item::Object { extra: Some(e), .. } if !e.combined.is_empty() => {
                let parts = e.combined.clone();
                self.farmer.inventory.reduce(slot, 1);
                for p in parts {
                    self.give_item(Item::object(&p, 1));
                }
            }
            _ => return Err("There's nothing to unforge.".into()),
        }
        if let (Some(s), true) = (shard, refund > 0) {
            self.give_item(Item::object(s, refund / 2));
        }
        self.sound("debuffHit");
        Ok(())
    }

    pub(crate) fn open_forge(&mut self) -> bool {
        self.emit(WorldEvent::OpenForge);
        true
    }
}

/// Lines describing an item's forges and enchantments (tooltips).
pub fn forge_lines(item: &Item) -> Vec<String> {
    let mut v = Vec::new();
    match item {
        Item::Weapon { mods: Some(m), .. } => {
            for g in &m.gems {
                v.push(format!("Forged: {g}"));
            }
            for (k, n) in &m.innate {
                let t = enchantment_title(k).unwrap_or(k);
                v.push(if matches!(k.as_str(), "SlimeSlayer" | "SlimeGatherer") { t.to_string() } else { format!("{t} +{n}") });
            }
            if let Some(e) = &m.enchantment {
                v.push(enchantment_title(e).unwrap_or(e).to_string());
            }
            if m.souls > 0 {
                v.push(format!("Galaxy souls: {}/{SOULS_NEEDED}", m.souls));
            }
        }
        Item::Tool(t) => {
            if let Some(e) = &t.enchantment {
                v.push(enchantment_title(e).unwrap_or(e).to_string());
            }
        }
        _ => {}
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gem_forges_stack() {
        let mut s = WeaponStats { min: 10, max: 20, knockback: 1.0, speed: 0, crit: 0.02, crit_mult: 3.0, defense: 0, aoe: 0, kind: 3 };
        let m = WeaponMods { gems: vec!["Ruby".into(), "Ruby".into(), "Topaz".into()], ..Default::default() };
        apply_mods(&mut s, &m);
        assert_eq!((s.min, s.max, s.defense), (12, 24, 1));
    }

    #[test]
    fn reaching_needs_iridium() {
        assert!(!tool_enchants(ToolKind::Hoe, 3).contains(&"Reaching"));
        assert!(tool_enchants(ToolKind::WateringCan, 4).contains(&"Reaching"));
    }
}
