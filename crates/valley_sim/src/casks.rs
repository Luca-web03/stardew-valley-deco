//! Casks: age drinks and cheeses to higher quality over days. A cask is a
//! big craftable with id `163` or the `Port.Cask` custom field; an object
//! ages at its `Port.AgingRate` (else a well-known rate, approx.). Quality
//! thresholds are in "base days" (rate 1.0 = one per day).

use crate::item::Quality;
use crate::objects::ObjKind;
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Base days needed for silver, gold and iridium quality.
pub const AGE_SILVER: f32 = 14.0;
pub const AGE_GOLD: f32 = 28.0;
pub const AGE_IRIDIUM: f32 = 56.0;

/// Well-known agable object ids and their rates (approx.).
const RATES: &[(&str, f32)] = &[("348", 1.0), ("303", 1.66), ("346", 2.0), ("459", 2.0), ("424", 4.0), ("426", 4.0)];

/// Quality reached after `aged` base days.
pub fn aged_quality(aged: f32) -> Quality {
    match aged {
        a if a >= AGE_IRIDIUM => Quality::Iridium,
        a if a >= AGE_GOLD => Quality::Gold,
        a if a >= AGE_SILVER => Quality::Silver,
        _ => Quality::Normal,
    }
}

fn quality_age(q: Quality) -> f32 {
    match q {
        Quality::Normal => 0.0,
        Quality::Silver => AGE_SILVER,
        Quality::Gold => AGE_GOLD,
        Quality::Iridium => AGE_IRIDIUM,
    }
}

impl World {
    pub fn is_cask(&self, big_id: &str) -> bool {
        big_id == "163" || self.items.big(big_id).is_some_and(|b| b.data.custom_fields.as_ref().is_some_and(|f| f.contains_key("Port.Cask")))
    }

    /// How fast an object ages in a cask, if at all.
    pub fn aging_rate(&self, id: &str) -> Option<f32> {
        if let Some(r) = self.items.object(id).and_then(|o| o.data.custom_fields.as_ref()).and_then(|f| f.get("Port.AgingRate")).and_then(|v| v.parse().ok()) {
            return Some(r);
        }
        RATES.iter().find(|(k, _)| *k == id).map(|(_, r)| *r)
    }

    /// Put the held item in an empty cask, or take the aged item out.
    pub(crate) fn cask_action(&mut self, t: TilePos) -> bool {
        let Some(ObjKind::Cask { item, aged }) = self.location().object(t).map(|o| o.kind.clone()) else { return false };
        match item {
            Some(it) => {
                let q = aged_quality(aged);
                let base = it.quality();
                if q <= base && aged < AGE_IRIDIUM {
                    let left = (quality_age(next_quality(base)) - aged).max(0.0);
                    let rate = it.object_id().and_then(|id| self.aging_rate(id)).unwrap_or(1.0);
                    self.emit(WorldEvent::Message(format!("Still aging. About {} more days for the next quality.", (left / rate).ceil() as i32)));
                    return true;
                }
                let mut out = it;
                out.set_quality(q.max(base));
                if !self.farmer.inventory.can_fit(&out) {
                    self.hud("Inventory full");
                    return true;
                }
                self.farmer.inventory.add(out.clone());
                if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                    o.kind = ObjKind::Cask { item: None, aged: 0.0 };
                }
                self.sound("coin");
                self.emit(WorldEvent::Pickup(out));
                true
            }
            None => {
                let slot = self.farmer.current_slot;
                let Some(held) = self.farmer.inventory.get(slot).cloned() else { return false };
                let Some(id) = held.object_id() else { return false };
                if self.aging_rate(id).is_none() {
                    self.emit(WorldEvent::Message("That won't age in a cask.".into()));
                    return true;
                }
                let mut one = held.clone();
                one.set_stack(1);
                // Already-better items start further along.
                let start = quality_age(one.quality());
                self.farmer.inventory.reduce(slot, 1);
                if let Some(o) = self.location_mut().state.objects.get_mut(&t) {
                    o.kind = ObjKind::Cask { item: Some(one), aged: start };
                }
                self.sound("Ship");
                true
            }
        }
    }

    /// Night: every filled cask ages by its item's rate (stops at iridium).
    pub(crate) fn casks_overnight(&mut self) {
        let rates: std::collections::BTreeMap<String, f32> = self
            .locations
            .values()
            .flat_map(|l| l.state.objects.values())
            .filter_map(|o| match &o.kind {
                ObjKind::Cask { item: Some(it), .. } => it.object_id().map(String::from),
                _ => None,
            })
            .filter_map(|id| self.aging_rate(&id).map(|r| (id, r)))
            .collect();
        for loc in self.locations.values_mut() {
            for o in loc.state.objects.values_mut() {
                if let ObjKind::Cask { item: Some(it), aged } = &mut o.kind {
                    let rate = it.object_id().and_then(|id| rates.get(id)).copied().unwrap_or(1.0);
                    *aged = (*aged + rate).min(AGE_IRIDIUM);
                }
            }
        }
    }
}

fn next_quality(q: Quality) -> Quality {
    match q {
        Quality::Normal => Quality::Silver,
        Quality::Silver => Quality::Gold,
        _ => Quality::Iridium,
    }
}
