//! Incoming calls (`Data/IncomingPhoneCalls`, public format): each morning
//! one call may be picked among those whose `TriggerCondition` passes and
//! that haven't reached `MaxCalls` (default 1); unless `IgnoreBaseChance`,
//! a base chance per day (approx.) applies. It rings at a random time
//! during the day while `RingCondition` holds, if the farmer owns a
//! telephone; answering plays `Dialogue` (pages split on
//! `SimpleDialogueSplitBy` if given) from `FromNpc` (or `FromDisplayName`).
//! Unanswered calls stop ringing after two hours.

use std::collections::BTreeMap;

use engine::Rng;
use serde::Deserialize;

use crate::dialogue::{DialoguePage, Emotion, Substitutions};
use crate::world::{World, WorldEvent};

/// Chance per day that someone calls (approx.).
pub const BASE_CALL_CHANCE: f64 = 0.15;
/// Minutes a call keeps ringing.
pub const RING_MINUTES: i32 = 120;

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct IncomingCall {
    pub trigger_condition: Option<String>,
    pub ring_condition: Option<String>,
    pub from_npc: Option<String>,
    pub from_portrait: Option<String>,
    pub from_display_name: Option<String>,
    pub dialogue: String,
    pub ignore_base_chance: bool,
    pub simple_dialogue_split_by: Option<String>,
    pub max_calls: i32,
}

impl Default for IncomingCall {
    fn default() -> Self {
        Self {
            trigger_condition: None,
            ring_condition: None,
            from_npc: None,
            from_portrait: None,
            from_display_name: None,
            dialogue: String::new(),
            ignore_base_chance: false,
            simple_dialogue_split_by: None,
            max_calls: 1,
        }
    }
}

pub fn load(cm: &mut valley_content::ContentManager) -> BTreeMap<String, IncomingCall> {
    cm.load_dict("Data/IncomingPhoneCalls")
}

/// Today's call: (id, ring time, ringing now).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PendingCall {
    pub id: String,
    pub ring_at: i32,
    pub ringing: bool,
}

fn calls_key(id: &str) -> String {
    format!("Port.PhoneCall.{id}")
}

impl World {
    fn owns_phone(&self) -> bool {
        self.locations.values().any(|l| l.state.objects.values().any(|o| o.big && self.is_telephone(&o.item_id)))
    }

    /// Morning: maybe line up a call for today.
    pub(crate) fn phone_calls_morning(&mut self) {
        self.pending_call = None;
        if self.incoming_calls.is_empty() || !self.owns_phone() {
            return;
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.days_played() as i64, 2140]));
        let base = rng.chance(BASE_CALL_CHANCE);
        let calls = self.incoming_calls.clone();
        let mut options = Vec::new();
        for (id, c) in &calls {
            let made = self.farmer.counters.get(&calls_key(id)).copied().unwrap_or(0);
            if c.max_calls >= 0 && made >= c.max_calls {
                continue;
            }
            if !c.ignore_base_chance && !base {
                continue;
            }
            if !self.check_query(c.trigger_condition.as_deref(), None, None, &mut rng.clone()) {
                continue;
            }
            options.push(id.clone());
        }
        let Some(id) = rng.pick(&options).cloned() else { return };
        let ring_at = crate::time::Clock::add_minutes(900, rng.range(0, 48) * 10);
        self.pending_call = Some(PendingCall { id, ring_at, ringing: false });
    }

    /// Every ten minutes: start or stop ringing.
    pub(crate) fn phone_calls_tick(&mut self) {
        let Some(mut p) = self.pending_call.clone() else { return };
        let time = self.clock.time;
        if time >= crate::time::Clock::add_minutes(p.ring_at, RING_MINUTES) {
            self.pending_call = None;
            return;
        }
        if !p.ringing && time >= p.ring_at {
            let ring_ok = self.incoming_calls.get(&p.id).map(|c| c.ring_condition.clone()).is_some_and(|cond| {
                let mut rng = self.day_rng(2141);
                self.check_query(cond.as_deref(), None, None, &mut rng)
            });
            if ring_ok {
                p.ringing = true;
                let here = self.location().state.objects.values().any(|o| o.big && self.is_telephone(&o.item_id));
                if here {
                    self.sound("phone");
                    self.hud("The phone is ringing.");
                }
            }
        }
        self.pending_call = Some(p);
    }

    /// `phone_calls_tick` for tools and tests.
    pub fn phone_calls_tick_pub(&mut self) {
        self.phone_calls_tick();
    }

    pub fn phone_ringing(&self) -> bool {
        self.pending_call.as_ref().is_some_and(|p| p.ringing)
    }

    /// Pick up a ringing phone. True if a call was answered.
    pub(crate) fn answer_call(&mut self) -> bool {
        let Some(p) = self.pending_call.clone().filter(|p| p.ringing) else { return false };
        self.pending_call = None;
        let Some(c) = self.incoming_calls.get(&p.id).cloned() else { return false };
        *self.farmer.counters.entry(calls_key(&p.id)).or_insert(0) += 1;
        let text = self.items.expand_text(&c.dialogue);
        let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
        let pages: Vec<DialoguePage> = match c.simple_dialogue_split_by.as_deref().filter(|s| !s.is_empty()) {
            Some(sep) => text.split(sep).map(|t| DialoguePage { text: t.trim().to_string(), emotion: Emotion::Neutral }).collect(),
            None => crate::dialogue::parse(&text, &subs),
        };
        let speaker = c.from_npc.clone().or(c.from_display_name.clone().map(|n| self.items.expand_text(&n)));
        self.sound("telephone_buttonPush");
        self.emit(WorldEvent::Dialogue { npc: speaker, pages });
        true
    }
}
