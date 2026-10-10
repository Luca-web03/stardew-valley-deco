//! Special orders from `Data/SpecialOrders` (public format, subset): the
//! town board offers two orders each week; one can be active at a time.
//! Objectives: Collect, Ship, Deliver, Fish, Gift, Slay, ReachMineFloor.
//! Rewards: Money, Friendship, Mail. Board timing is approximate.

use std::collections::BTreeMap;

use engine::Rng;
use serde::{Deserialize, Serialize};

use crate::item::Item;
use crate::world::{World, WorldEvent};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct OrderObjectiveData {
    #[serde(rename = "Type")]
    pub kind: String,
    pub text: String,
    pub required_count: String,
    pub data: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct OrderRewardData {
    #[serde(rename = "Type")]
    pub kind: String,
    pub data: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct SpecialOrderData {
    pub name: String,
    pub requester: String,
    pub duration: String,
    pub repeatable: bool,
    pub required_tags: String,
    pub condition: Option<String>,
    pub order_type: String,
    pub text: String,
    pub objectives: Vec<OrderObjectiveData>,
    pub rewards: Vec<OrderRewardData>,
}

const SUPPORTED: &[&str] = &["Collect", "Ship", "Deliver", "Fish", "Gift", "Slay", "ReachMineFloor"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrderObjective {
    pub kind: String,
    pub text: String,
    pub required: i32,
    pub progress: i32,
    pub data: BTreeMap<String, String>,
}

impl OrderObjective {
    pub fn done(&self) -> bool {
        self.progress >= self.required
    }

    fn tags(&self) -> Vec<String> {
        self.data.get("AcceptedContextTags").map(|t| vec![t.clone()]).unwrap_or_default()
    }

    fn target(&self) -> Option<&str> {
        self.data.get("TargetName").map(String::as_str).filter(|s| !s.is_empty())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveOrder {
    pub id: String,
    pub name: String,
    pub requester: String,
    pub text: String,
    pub days_left: i32,
    pub objectives: Vec<OrderObjective>,
}

impl ActiveOrder {
    pub fn done(&self) -> bool {
        self.objectives.iter().all(OrderObjective::done)
    }
}

/// Days an order lasts.
pub fn duration_days(d: &str) -> i32 {
    match d {
        "Day" => 1,
        "ThreeDays" => 3,
        "TwoWeeks" => 14,
        "Month" => 28,
        _ => 7,
    }
}

/// Orders shown on the board this week.
pub const OFFERS: usize = 2;

impl World {
    fn order_tags_ok(&self, tags: &str) -> bool {
        tags.split(',').map(str::trim).filter(|t| !t.is_empty()).all(|tag| {
            let (neg, t) = match tag.strip_prefix('!') {
                Some(t) => (true, t),
                None => (false, tag),
            };
            let known = if let Some(s) = t.strip_prefix("season_") {
                Some(self.clock.season.key().eq_ignore_ascii_case(s))
            } else if let Some(m) = t.strip_prefix("mail_") {
                Some(self.farmer.mail_received.iter().any(|x| x == m))
            } else if let Some(e) = t.strip_prefix("event_") {
                Some(self.farmer.events_seen.iter().any(|x| x == e))
            } else {
                t.strip_prefix("knows_").map(|n| self.farmer.friendships.contains_key(n))
            };
            match known {
                Some(v) => v != neg,
                // Unknown requirements: only satisfied when negated.
                None => neg,
            }
        })
    }

    fn order_available(&self, id: &str, d: &SpecialOrderData) -> bool {
        if !(d.order_type.is_empty() || d.order_type == "Town") {
            return false;
        }
        if !d.objectives.iter().all(|o| SUPPORTED.contains(&o.kind.as_str())) || d.objectives.is_empty() {
            return false;
        }
        if !d.repeatable && self.farmer.orders_completed.iter().any(|c| c == id) {
            return false;
        }
        if self.farmer.special_order.as_ref().is_some_and(|o| o.id == id) {
            return false;
        }
        let mut rng = self.day_rng(23);
        let mail = self.farmer.mail_received.clone();
        let seen = self.farmer.events_seen.clone();
        let mut ctx = crate::gsq::QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng };
        self.order_tags_ok(&d.required_tags) && crate::gsq::check(d.condition.as_deref(), &mut ctx)
    }

    /// Monday morning (or first look): pick this week's board offers.
    pub(crate) fn refresh_order_board(&mut self) {
        let week = self.clock.days_played() / 7;
        let mut ids: Vec<String> = self.special_orders.iter().filter(|(id, d)| self.order_available(id, d)).map(|(id, _)| id.clone()).collect();
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, week as i64, 29]));
        for i in (1..ids.len()).rev() {
            let j = rng.range(0, i as i32 + 1) as usize;
            ids.swap(i, j);
        }
        ids.truncate(OFFERS);
        self.order_offers = ids;
    }

    /// The board action: show offers (or the active order).
    pub fn open_order_board(&mut self) -> bool {
        if self.order_offers.is_empty() && self.farmer.special_order.is_none() {
            self.refresh_order_board();
        }
        self.emit(WorldEvent::OpenOrderBoard);
        true
    }

    /// Display text for an order id (title, description).
    pub fn order_text(&self, id: &str) -> Option<(String, String)> {
        let d = self.special_orders.get(id)?;
        Some((self.items.expand_text(&d.name), self.items.expand_text(&d.text)))
    }

    pub fn accept_order(&mut self, id: &str) -> Result<(), String> {
        if self.farmer.special_order.is_some() {
            return Err("You already have an order in progress.".into());
        }
        let d = self.special_orders.get(id).cloned().ok_or("No such order.")?;
        let objectives = d
            .objectives
            .iter()
            .map(|o| OrderObjective {
                kind: o.kind.clone(),
                text: self.items.expand_text(&o.text),
                required: o.required_count.trim().parse().unwrap_or(1).max(1),
                progress: 0,
                data: o.data.clone(),
            })
            .collect();
        self.farmer.special_order = Some(ActiveOrder {
            id: id.to_string(),
            name: self.items.expand_text(&d.name),
            requester: d.requester.clone(),
            text: self.items.expand_text(&d.text),
            days_left: duration_days(&d.duration),
            objectives,
        });
        self.order_offers.retain(|o| o != id);
        self.sound("newArtifact");
        self.hud("Special order accepted. (F: journal)");
        Ok(())
    }

    /// Add `f(index, objective)` progress to each unfinished objective.
    fn advance_objectives(&mut self, mut f: impl FnMut(usize, &OrderObjective) -> i32) {
        let Some(order) = self.farmer.special_order.as_mut() else { return };
        let mut changed = false;
        for (i, o) in order.objectives.iter_mut().enumerate().filter(|(_, o)| !o.done()) {
            let n = f(i, o);
            if n > 0 {
                o.progress = (o.progress + n).min(o.required);
                changed = true;
            }
        }
        if changed && order.done() {
            self.complete_order();
        }
    }

    fn item_matches(&self, o: &OrderObjective, item: &Item) -> bool {
        let tags = o.tags();
        !tags.is_empty() && self.items.matches_tags(item, &tags)
    }

    pub(crate) fn order_obtained(&mut self, item: &Item) {
        let Some(order) = self.farmer.special_order.clone() else { return };
        let counts: Vec<i32> = order.objectives.iter().map(|o| if o.kind == "Collect" && self.item_matches(o, item) { item.stack() } else { 0 }).collect();
        self.apply_counts(counts);
    }

    pub(crate) fn order_shipped(&mut self, item: &Item) {
        let Some(order) = self.farmer.special_order.clone() else { return };
        let counts: Vec<i32> = order.objectives.iter().map(|o| if o.kind == "Ship" && self.item_matches(o, item) { item.stack() } else { 0 }).collect();
        self.apply_counts(counts);
    }

    pub(crate) fn order_fish(&mut self, item: &Item) {
        let Some(order) = self.farmer.special_order.clone() else { return };
        let counts: Vec<i32> = order
            .objectives
            .iter()
            .map(|o| if o.kind == "Fish" && (o.tags().is_empty() || self.item_matches(o, item)) { 1 } else { 0 })
            .collect();
        self.apply_counts(counts);
    }

    pub(crate) fn order_gift(&mut self, npc: &str, item: &Item) {
        let Some(order) = self.farmer.special_order.clone() else { return };
        let counts: Vec<i32> = order
            .objectives
            .iter()
            .map(|o| {
                let who = o.target().map_or(true, |t| t == npc);
                if o.kind == "Gift" && who && (o.tags().is_empty() || self.item_matches(o, item)) { 1 } else { 0 }
            })
            .collect();
        self.apply_counts(counts);
    }

    pub(crate) fn order_slay(&mut self, monster: &str) {
        self.advance_objectives(|_, o| {
            let any = o.target().map_or(true, |t| t.split(',').any(|n| n.trim().eq_ignore_ascii_case(monster)));
            if o.kind == "Slay" && any { 1 } else { 0 }
        });
    }

    pub(crate) fn order_mine_floor(&mut self, level: i32) {
        let Some(order) = self.farmer.special_order.as_mut() else { return };
        let mut changed = false;
        for o in order.objectives.iter_mut().filter(|o| o.kind == "ReachMineFloor" && !o.done()) {
            if level > o.progress {
                o.progress = level.min(o.required);
                changed = true;
            }
        }
        if changed && order.done() {
            self.complete_order();
        }
    }

    fn apply_counts(&mut self, counts: Vec<i32>) {
        self.advance_objectives(|i, _| counts.get(i).copied().unwrap_or(0));
    }

    /// Hand the held item to `npc` for a Deliver objective.
    pub(crate) fn try_order_delivery(&mut self, npc: &str) -> bool {
        let slot = self.farmer.current_slot;
        let Some(held) = self.farmer.inventory.get(slot).cloned() else { return false };
        let Some(order) = self.farmer.special_order.clone() else { return false };
        let Some(k) = order.objectives.iter().position(|o| o.kind == "Deliver" && !o.done() && o.target() == Some(npc) && self.item_matches(o, &held)) else {
            return false;
        };
        let need = order.objectives[k].required - order.objectives[k].progress;
        let give = need.min(held.stack());
        self.farmer.inventory.reduce(slot, give);
        let mut counts = vec![0; order.objectives.len()];
        counts[k] = give;
        self.sound("give_gift");
        self.hud(format!("Delivered {give} to {npc}."));
        // Count only the targeted objective, not every matching one.
        self.apply_counts(counts);
        true
    }

    fn complete_order(&mut self) {
        let Some(order) = self.farmer.special_order.take() else { return };
        let rewards = self.special_orders.get(&order.id).map(|d| d.rewards.clone()).unwrap_or_default();
        for r in rewards {
            match r.kind.as_str() {
                "Money" => {
                    if let Some(n) = r.data.get("Amount").and_then(|a| a.trim().parse::<i32>().ok()) {
                        self.farmer.money += n;
                        self.farmer.total_earned += n.max(0) as i64;
                    }
                }
                "Friendship" => {
                    let who = r.data.get("TargetName").cloned().unwrap_or_else(|| order.requester.clone());
                    let amount = r.data.get("Amount").and_then(|a| a.trim().parse().ok()).unwrap_or(250);
                    self.farmer.friendship(&who).points += amount;
                }
                "Mail" => {
                    for m in r.data.get("MailReceived").map(|m| m.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>()).unwrap_or_default() {
                        if !m.is_empty() && !self.farmer.mail_received.contains(&m) {
                            self.farmer.mail_received.push(m);
                        }
                    }
                }
                _ => {}
            }
        }
        self.farmer.orders_completed.push(order.id.clone());
        self.sound("reward");
        self.emit(WorldEvent::Hud(format!("Special order complete: {}", order.name)));
    }

    /// Night: count down the active order; refresh the board on Mondays.
    pub(crate) fn tick_orders(&mut self) {
        if let Some(o) = self.farmer.special_order.as_mut() {
            o.days_left -= 1;
            if o.days_left <= 0 {
                let name = o.name.clone();
                self.farmer.special_order = None;
                self.emit(WorldEvent::Hud(format!("Special order expired: {name}")));
            }
        }
    }

    pub(crate) fn orders_morning(&mut self) {
        if self.clock.weekday() == "Mon" || self.order_offers.is_empty() {
            self.refresh_order_board();
        }
    }
}
