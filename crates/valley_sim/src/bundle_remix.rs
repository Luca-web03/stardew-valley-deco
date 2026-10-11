//! Remixed community bundles (`Data/RandomBundles`, public format subset):
//! per area, a list of bundle keys, alternative bundle sets and a pool of
//! extra bundles. A new game with remixed bundles picks one set per area,
//! fills the remaining slots from the pool, picks each bundle's items and
//! writes the result in the `Data/Bundles` string format, kept in the save.
//!
//! Item lists are comma-separated "count [quality] name-or-id" entries
//! (quality NQ/SQ/GQ/IQ); "Money" stands for gold. Rewards use the same
//! form or the bundle reward format ("O 465 20").

use std::collections::{BTreeMap, HashMap};

use engine::Rng;
use serde::Deserialize;

use crate::item::ItemDb;
use crate::world::World;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct RandomBundle {
    pub name: String,
    pub index: i32,
    pub sprite: Option<String>,
    pub color: Option<String>,
    pub items: String,
    pub pick: i32,
    pub required_items: i32,
    pub reward: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct BundleSet {
    pub id: Option<String>,
    pub bundles: Vec<RandomBundle>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct RandomBundleArea {
    pub area_name: String,
    pub keys: String,
    pub bundle_sets: Vec<BundleSet>,
    pub bundles: Vec<RandomBundle>,
}

const COLORS: &[&str] = &["green", "purple", "orange", "yellow", "red", "blue", "teal"];

fn color_index(c: Option<&str>) -> i32 {
    let c = c.unwrap_or("").trim();
    c.parse().ok().or_else(|| COLORS.iter().position(|n| n.eq_ignore_ascii_case(c)).map(|i| i as i32)).unwrap_or(0)
}

/// Object id for a name or id ("Parsnip", "24", "(O)24").
fn resolve(db: &ItemDb, name: &str) -> Option<String> {
    let (_, id) = crate::item::split_qualified(name);
    if db.objects.contains_key(id) {
        return Some(id.to_string());
    }
    db.objects
        .iter()
        .find(|(_, o)| o.data.name.eq_ignore_ascii_case(name) || o.display_name.eq_ignore_ascii_case(name))
        .map(|(k, _)| k.clone())
}

/// One "count [quality] name" entry as a bundle requirement
/// ("id count quality"), or None when the item is unknown.
fn requirement(db: &ItemDb, entry: &str) -> Option<String> {
    let mut toks: Vec<&str> = entry.split_whitespace().collect();
    let count: i32 = match toks.first().and_then(|t| t.parse().ok()) {
        Some(n) => {
            toks.remove(0);
            n
        }
        None => 1,
    };
    let quality = match toks.first().copied() {
        Some("NQ") => Some(0),
        Some("SQ") => Some(1),
        Some("GQ") => Some(2),
        Some("IQ") => Some(4),
        _ => None,
    };
    if quality.is_some() {
        toks.remove(0);
    }
    let name = toks.join(" ");
    if name.eq_ignore_ascii_case("money") {
        return Some(format!("-1 {count} {count}"));
    }
    Some(format!("{} {count} {}", resolve(db, &name)?, quality.unwrap_or(0)))
}

/// A reward as "O id n" / "BO id n".
fn reward(db: &ItemDb, s: &str) -> String {
    let toks: Vec<&str> = s.split_whitespace().collect();
    if toks.first().is_some_and(|t| t.parse::<i32>().is_err()) && toks.len() >= 2 && toks[0].len() <= 2 {
        return s.to_string();
    }
    let (count, rest) = match toks.first().and_then(|t| t.parse::<i32>().ok()) {
        Some(n) => (n, &toks[1..]),
        None => (1, &toks[..]),
    };
    // "1 BO 10": a typed id after the count.
    if let [ty @ ("O" | "BO"), id] = rest {
        return format!("{ty} {id} {count}");
    }
    let name = rest.join(" ");
    if let Some(id) = resolve(db, &name) {
        return format!("O {id} {count}");
    }
    let (_, id) = crate::item::split_qualified(&name);
    if db.bigs.contains_key(id) {
        return format!("BO {id} {count}");
    }
    if let Some((k, _)) = db.bigs.iter().find(|(_, b)| b.data.name.eq_ignore_ascii_case(&name)) {
        return format!("BO {k} {count}");
    }
    String::new()
}

fn bundle_string(db: &ItemDb, b: &RandomBundle, rng: &mut Rng) -> String {
    let mut entries: Vec<&str> = b.items.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    if b.pick > 0 && (b.pick as usize) < entries.len() {
        for i in (1..entries.len()).rev() {
            let j = rng.range(0, i as i32 + 1) as usize;
            entries.swap(i, j);
        }
        entries.truncate(b.pick as usize);
    }
    let reqs: Vec<String> = entries.iter().filter_map(|e| requirement(db, e)).collect();
    let required = if b.required_items > 0 { (b.required_items as usize).min(reqs.len()) } else { reqs.len() };
    format!(
        "{}/{}/{}/{}/{}/{}/{}",
        b.name,
        b.reward.as_deref().map(|r| reward(db, r)).unwrap_or_default(),
        reqs.join(" "),
        color_index(b.color.as_deref()),
        required,
        b.sprite.clone().unwrap_or_default(),
        b.name
    )
}

/// Generate remixed bundle data ("Area/key" -> bundle string).
pub fn generate(areas: &[RandomBundleArea], db: &ItemDb, rng: &mut Rng) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for area in areas {
        let keys: Vec<&str> = area.keys.split_whitespace().collect();
        let mut slots: Vec<Option<RandomBundle>> = vec![None; keys.len()];
        if let Some(set) = rng.pick(&area.bundle_sets).cloned() {
            for b in set.bundles {
                if let Some(s) = slots.get_mut(b.index.max(0) as usize).filter(|s| s.is_none()) {
                    *s = Some(b);
                }
            }
        }
        let mut pool = area.bundles.clone();
        for (i, slot) in slots.iter_mut().enumerate() {
            if slot.is_some() {
                continue;
            }
            // Prefer pool bundles made for this slot.
            let matching: Vec<usize> = (0..pool.len()).filter(|&j| pool[j].index == i as i32).collect();
            let choices: Vec<usize> = if matching.is_empty() { (0..pool.len()).collect() } else { matching };
            if let Some(&j) = rng.pick(&choices) {
                *slot = Some(pool.remove(j));
            }
        }
        for (key, b) in keys.iter().zip(slots) {
            if let Some(b) = b {
                out.insert(format!("{}/{key}", area.area_name), bundle_string(db, &b, rng));
            }
        }
    }
    out
}

impl World {
    /// Use bundle data kept in the save (remixed bundles), if any.
    pub fn apply_bundle_data(&mut self) {
        if self.farmer.bundle_data.is_empty() {
            return;
        }
        let raw: HashMap<String, String> = self.farmer.bundle_data.clone().into_iter().collect();
        self.bundles = crate::bundles::parse_bundles(&raw, &self.items);
    }

    /// New game: roll remixed bundles from `Data/RandomBundles`.
    pub(crate) fn remix_bundles(&mut self, cm: &mut valley_content::ContentManager) {
        let areas: Vec<RandomBundleArea> = cm.load_json("Data/RandomBundles").unwrap_or_default();
        if areas.is_empty() {
            return;
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, 4747]));
        let data = generate(&areas, &self.items, &mut rng);
        // Areas the remix doesn't cover keep their standard bundles.
        let mut all: BTreeMap<String, String> = cm.load_string_dict("Data/Bundles").into_iter().filter(|(k, _)| !areas.iter().any(|a| k.split('/').next() == Some(a.area_name.as_str()))).collect();
        all.extend(data);
        self.farmer.bundle_data = all;
        self.apply_bundle_data();
    }
}
