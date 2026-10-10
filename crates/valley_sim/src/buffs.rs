//! Timed buffs from food and drink (`Buffs` field of object data).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BuffAttrs {
    pub farming: i32,
    pub fishing: i32,
    pub mining: i32,
    pub foraging: i32,
    pub combat: i32,
    pub luck: i32,
    pub speed: i32,
    pub max_stamina: i32,
    pub attack: i32,
    pub defense: i32,
    pub magnetic_radius: i32,
}

impl BuffAttrs {
    pub fn add(&mut self, o: &BuffAttrs) {
        self.farming += o.farming;
        self.fishing += o.fishing;
        self.mining += o.mining;
        self.foraging += o.foraging;
        self.combat += o.combat;
        self.luck += o.luck;
        self.speed += o.speed;
        self.max_stamina += o.max_stamina;
        self.attack += o.attack;
        self.defense += o.defense;
        self.magnetic_radius += o.magnetic_radius;
    }

    pub fn describe(&self) -> Vec<String> {
        let mut v = Vec::new();
        for (n, x) in [
            ("Farming", self.farming),
            ("Fishing", self.fishing),
            ("Mining", self.mining),
            ("Foraging", self.foraging),
            ("Combat", self.combat),
            ("Luck", self.luck),
            ("Speed", self.speed),
            ("Max Energy", self.max_stamina),
            ("Attack", self.attack),
            ("Defense", self.defense),
            ("Magnetism", self.magnetic_radius),
        ] {
            if x != 0 {
                v.push(format!("{}{} {n}", if x > 0 { "+" } else { "" }, x));
            }
        }
        v
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Buff {
    pub id: String,
    pub source: String,
    /// Remaining game minutes.
    pub minutes_left: i32,
    pub attrs: BuffAttrs,
}

/// Parse an object's `Buffs` JSON list.
pub fn parse_buffs(v: &serde_json::Value, source: &str) -> Vec<Buff> {
    let mut out = Vec::new();
    let Some(list) = v.as_array() else { return out };
    for b in list {
        let id = b.get("Id").and_then(|x| x.as_str()).or_else(|| b.get("BuffId").and_then(|x| x.as_str())).unwrap_or("food").to_string();
        let minutes = b.get("Duration").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        let mut attrs = BuffAttrs::default();
        if let Some(ca) = b.get("CustomAttributes").and_then(|x| x.as_object()) {
            let g = |k: &str| ca.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0) as i32;
            attrs = BuffAttrs {
                farming: g("FarmingLevel"),
                fishing: g("FishingLevel"),
                mining: g("MiningLevel"),
                foraging: g("ForagingLevel"),
                combat: g("CombatLevel"),
                luck: g("LuckLevel"),
                speed: g("Speed"),
                max_stamina: g("MaxStamina"),
                attack: g("Attack"),
                defense: g("Defense"),
                magnetic_radius: g("MagneticRadius"),
            };
        }
        if minutes != 0 && attrs != BuffAttrs::default() {
            out.push(Buff { id, source: source.to_string(), minutes_left: if minutes < 0 { 24 * 60 } else { minutes }, attrs });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_custom_attributes() {
        let v: serde_json::Value = serde_json::from_str(r#"[{"Id":"food","Duration":120,"CustomAttributes":{"Speed":1,"FarmingLevel":2}}]"#).unwrap();
        let b = parse_buffs(&v, "Dish");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].attrs.speed, 1);
        assert_eq!(b[0].attrs.farming, 2);
        assert_eq!(b[0].attrs.describe(), vec!["+2 Farming", "+1 Speed"]);
    }
}
