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

/// A buff definition from `Data/Buffs` (`Duration` in milliseconds, -2 =
/// the rest of the day; `Effects` like an object's `CustomAttributes`).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct BuffDef {
    pub display_name: String,
    pub description: Option<String>,
    pub is_debuff: bool,
    pub duration: i32,
    pub effects: Option<serde_json::Value>,
}

/// Real milliseconds per game minute.
const MS_PER_GAME_MINUTE: i32 = 700;

fn attrs_of(ca: &serde_json::Map<String, serde_json::Value>) -> BuffAttrs {
    let g = |k: &str| ca.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0) as i32;
    BuffAttrs {
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
    }
}

/// A `Data/Buffs` buff as an active buff.
pub fn from_def(id: &str, def: &BuffDef, source: &str) -> Buff {
    let attrs = def.effects.as_ref().and_then(|e| e.as_object()).map(attrs_of).unwrap_or_default();
    let minutes = if def.duration < 0 { 24 * 60 } else { (def.duration / MS_PER_GAME_MINUTE).max(1) };
    Buff { id: id.to_string(), source: source.to_string(), minutes_left: minutes, attrs }
}

/// Parse an object's `Buffs` JSON list.
pub fn parse_buffs(v: &serde_json::Value, source: &str) -> Vec<Buff> {
    parse_buffs_with(v, source, &std::collections::BTreeMap::new())
}

/// Parse an object's `Buffs` list; a `BuffId` defined in `Data/Buffs`
/// starts from that buff, with the object's own attributes added.
pub fn parse_buffs_with(v: &serde_json::Value, source: &str, defs: &std::collections::BTreeMap<String, BuffDef>) -> Vec<Buff> {
    let mut out = Vec::new();
    let Some(list) = v.as_array() else { return out };
    for b in list {
        let buff_id = b.get("BuffId").and_then(|x| x.as_str());
        let id = b.get("Id").and_then(|x| x.as_str()).or(buff_id).unwrap_or("food").to_string();
        let base = buff_id.and_then(|bid| defs.get(bid)).map(|d| from_def(&id, d, source));
        let minutes = b.get("Duration").and_then(|x| x.as_i64()).map(|m| m as i32).or(base.as_ref().map(|b| b.minutes_left)).unwrap_or(0);
        let mut attrs = base.as_ref().map(|b| b.attrs.clone()).unwrap_or_default();
        if let Some(ca) = b.get("CustomAttributes").and_then(|x| x.as_object()) {
            attrs.add(&attrs_of(ca));
        }
        if minutes != 0 && (attrs != BuffAttrs::default() || base.is_some()) {
            out.push(Buff { id, source: source.to_string(), minutes_left: if minutes < 0 { 24 * 60 } else { minutes }, attrs });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buff_ids_start_from_data_buffs() {
        let mut defs = std::collections::BTreeMap::new();
        defs.insert("28".to_string(), BuffDef { display_name: "Inked".into(), duration: 7000, effects: Some(serde_json::json!({ "Defense": 2 })), ..Default::default() });
        let v: serde_json::Value = serde_json::from_str(r#"[{"BuffId":"28","CustomAttributes":{"Speed":1}}]"#).unwrap();
        let b = parse_buffs_with(&v, "Ravioli", &defs);
        assert_eq!(b.len(), 1);
        assert_eq!((b[0].attrs.defense, b[0].attrs.speed, b[0].minutes_left), (2, 1, 10));
    }

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
