//! Island-style progression, as generic data-driven mechanics:
//!
//! * Collectibles: objects with `Port.Collectible` ("<counter> [amount]")
//!   go to a named counter instead of the inventory (golden walnuts, id
//!   `73`, count as `GoldenWalnut`).
//! * Unlocks: tile action `Port.Unlock <counter> <cost> <flag> [text]`
//!   spends from a counter to set a flag (a mail flag), once.
//! * Cleared tiles: Buildings-layer tiles with the `Port.ClearedBy <flag>`
//!   property disappear (become walkable) once the farmer has that flag,
//!   e.g. a repaired bridge or a cleared rockslide.

use crate::item::Item;
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Well-known collectible objects: (object id, counter).
const COLLECTIBLES: &[(&str, &str)] = &[("73", "GoldenWalnut")];

impl World {
    /// (counter, amount) if `item` is a collectible.
    pub fn collectible(&self, item: &Item) -> Option<(String, i32)> {
        let id = item.object_id()?;
        let spec = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.Collectible")).cloned();
        let (counter, per) = match spec {
            Some(s) => {
                let t: Vec<&str> = s.split_whitespace().collect();
                (t.first()?.to_string(), t.get(1).and_then(|v| v.parse().ok()).unwrap_or(1))
            }
            None => (COLLECTIBLES.iter().find(|(k, _)| *k == id)?.1.to_string(), 1),
        };
        Some((counter, per * item.stack()))
    }

    pub fn counter(&self, name: &str) -> i32 {
        self.farmer.counters.get(name).copied().unwrap_or(0)
    }

    /// If `item` is a collectible, count it and return true.
    pub(crate) fn collect_counter(&mut self, item: &Item) -> bool {
        let Some((counter, n)) = self.collectible(item) else { return false };
        *self.farmer.counters.entry(counter.clone()).or_insert(0) += n;
        let name = self.items.display_name(item);
        self.sound("newArtifact");
        self.emit(WorldEvent::Hud(format!("{name} found! ({} total)", self.counter(&counter))));
        true
    }

    /// `Port.Unlock <counter> <cost> <flag> [text...]`.
    pub(crate) fn unlock_action(&mut self, toks: &[&str]) -> bool {
        let (Some(counter), Some(cost), Some(flag)) = (toks.get(1), toks.get(2).and_then(|c| c.parse::<i32>().ok()), toks.get(3)) else { return false };
        let text = if toks.len() > 4 { toks[4..].join(" ") } else { String::new() };
        if self.farmer.mail_received.iter().any(|m| m == flag) {
            self.emit(WorldEvent::Message("This has already been done.".into()));
            return true;
        }
        let have = self.counter(counter);
        if have < cost {
            let msg = if text.is_empty() { format!("Needs {cost} {counter} (you have {have}).") } else { format!("{text} (needs {cost} {counter}; you have {have})") };
            self.emit(WorldEvent::Message(msg));
            return true;
        }
        *self.farmer.counters.entry(counter.to_string()).or_insert(0) -= cost;
        self.farmer.mail_received.push(flag.to_string());
        self.apply_cleared_tiles();
        self.sound("newRecipe");
        self.emit(WorldEvent::Hud(if text.is_empty() { "Unlocked!".into() } else { format!("{text} Done!") }));
        true
    }

    /// Remove `Port.ClearedBy` tiles whose flag the farmer has.
    pub(crate) fn apply_cleared_tiles(&mut self) {
        let flags = self.farmer.mail_received.clone();
        let mut changed = false;
        for loc in self.locations.values_mut() {
            let Some(layer) = loc.map.layer("Buildings") else { continue };
            let mut clear = Vec::new();
            for y in 0..layer.height {
                for x in 0..layer.width {
                    let Some(t) = layer.get(x, y) else { continue };
                    let Some(flag) = t.props.get("Port.ClearedBy").map(|v| v.as_string()) else { continue };
                    if flags.iter().any(|f| *f == flag) {
                        clear.push(TilePos::new(x, y));
                    }
                }
            }
            if clear.is_empty() {
                continue;
            }
            if let Some(layer) = loc.map.layer_mut("Buildings") {
                for p in clear {
                    layer.set(p.x, p.y, None);
                }
            }
            changed = true;
        }
        if changed {
            self.build_location_edges_pub();
        }
    }
}
