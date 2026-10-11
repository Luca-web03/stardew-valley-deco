//! Crop growth. Rules follow the documented data format: each night a
//! watered crop advances one day; `DaysInPhase` gives the length of each
//! growth phase; after the last phase the crop is harvestable; crops with
//! `RegrowDays` > 0 return to the harvestable state after that many days.

use engine::Rng;
use serde::{Deserialize, Serialize};
use valley_content::data::CropData;

use crate::item::{Item, Quality};
use crate::time::Season;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    pub seed_id: String,
    /// Index into `DaysInPhase`; == len means fully grown.
    pub phase: usize,
    pub day_in_phase: i32,
    /// Days left until a regrowing crop is harvestable again (0 = ready).
    pub regrow_left: i32,
    /// True once harvested at least once (regrowing crops show their regrow sprite).
    pub harvested_once: bool,
    pub dead: bool,
    /// Sprite variation (mirrored drawing).
    pub flip: bool,
    /// Seed-phase sprite variant (0 or 1).
    pub variant: u8,
    /// Per-crop phase lengths when modified (speed fertilizer); None = data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_days: Option<Vec<i32>>,
}

impl Crop {
    pub fn new(seed_id: &str, rng: &mut Rng) -> Self {
        Self {
            seed_id: seed_id.to_string(),
            phase: 0,
            day_in_phase: 0,
            regrow_left: 0,
            harvested_once: false,
            dead: false,
            flip: rng.chance(0.5),
            variant: rng.range(0, 2) as u8,
            phase_days: None,
        }
    }

    /// Effective phase lengths.
    pub fn phases<'a>(&'a self, data: &'a CropData) -> &'a [i32] {
        self.phase_days.as_deref().unwrap_or(&data.days_in_phase)
    }

    /// Apply a growth speed bonus (e.g. 0.1 = 10% faster) to a crop that
    /// hasn't started growing: removes ceil(total * bonus) days, taking
    /// from the longest phases first and never shortening a phase below 1.
    pub fn apply_speed_bonus(&mut self, data: &CropData, bonus: f32) {
        let mut days: Vec<i32> = data.days_in_phase.clone();
        let total: i32 = days.iter().sum();
        let mut remove = (total as f32 * bonus).ceil() as i32;
        while remove > 0 {
            let Some((i, _)) = days.iter().enumerate().filter(|(_, d)| **d > 1).max_by_key(|(i, d)| (**d, -(*i as i32))) else { break };
            days[i] -= 1;
            remove -= 1;
        }
        self.phase_days = Some(days);
    }

    /// Jump straight to harvestable.
    pub fn grow_fully(&mut self, data: &CropData) {
        if self.dead {
            return;
        }
        self.phase = self.phases(data).len();
        self.day_in_phase = 0;
        self.regrow_left = 0;
    }

    pub fn grows_in(data: &CropData, season: Season) -> bool {
        data.seasons.iter().any(|s| Season::parse(s) == Some(season))
    }

    pub fn is_ready(&self, data: &CropData) -> bool {
        !self.dead && self.phase >= self.phases(data).len() && self.regrow_left <= 0
    }

    /// Total growth days remaining until first harvest.
    pub fn days_until_ready(&self, data: &CropData) -> i32 {
        if self.dead {
            return i32::MAX;
        }
        if self.phase >= self.phases(data).len() {
            return self.regrow_left.max(0);
        }
        let mut d = self.phases(data)[self.phase] - self.day_in_phase;
        for p in self.phases(data).iter().skip(self.phase + 1) {
            d += p;
        }
        d.max(0)
    }

    fn skip_empty_phases(&mut self, data: &CropData) {
        while self.phase < self.phases(data).len() && self.phases(data)[self.phase] <= 0 {
            self.phase += 1;
        }
    }

    /// Advance one night. `watered` covers rain and sprinklers too.
    pub fn new_day(&mut self, data: &CropData, watered: bool, season: Season, outdoors: bool) {
        if self.dead {
            return;
        }
        if outdoors && !Self::grows_in(data, season) {
            self.dead = true;
            return;
        }
        if !watered && data.needs_watering {
            return;
        }
        if self.phase >= self.phases(data).len() {
            if self.regrow_left > 0 {
                self.regrow_left -= 1;
            }
            return;
        }
        self.day_in_phase += 1;
        if self.day_in_phase >= self.phases(data)[self.phase] {
            self.phase += 1;
            self.day_in_phase = 0;
            self.skip_empty_phases(data);
        }
    }

    /// Harvest quality from farming level (and fertilizer quality level 0..3),
    /// using the publicly documented chance formula.
    pub fn roll_quality(rng: &mut Rng, farming_level: i32, fertilizer: i32, data: &CropData) -> Quality {
        let fl = farming_level as f64;
        let fert = fertilizer as f64;
        let gold = 0.2 * (fl / 10.0) + 0.2 * fert * ((fl + 2.0) / 12.0) + 0.01;
        let silver = (gold * 2.0).min(0.75);
        let q = if fert >= 3.0 && rng.chance(gold / 2.0) {
            4
        } else if rng.chance(gold) {
            2
        } else if rng.chance(silver) || fert >= 3.0 {
            1
        } else {
            0
        };
        let q = q.max(data.harvest_min_quality);
        let q = match data.harvest_max_quality {
            Some(m) => q.min(m),
            None => q,
        };
        Quality::from_level(q)
    }

    /// Harvest the crop. Returns the items produced and whether the crop
    /// should be removed from the soil.
    pub fn harvest(&mut self, data: &CropData, rng: &mut Rng, farming_level: i32, fertilizer: i32) -> Option<(Vec<Item>, bool)> {
        if !self.is_ready(data) {
            return None;
        }
        let quality = Self::roll_quality(rng, farming_level, fertilizer, data);
        let max_bonus = (data.harvest_max_increase_per_farming_level * farming_level as f32) as i32;
        let max = data.harvest_max_stack.max(data.harvest_min_stack) + max_bonus;
        let mut count = rng.range(data.harvest_min_stack.max(1), max + 1);
        while data.extra_harvest_chance > 0.0 && rng.chance(data.extra_harvest_chance.min(0.9)) {
            count += 1;
        }
        let mut items = vec![Item::object_q(&data.harvest_item_id, 1, quality)];
        if count > 1 {
            items.push(Item::object(&data.harvest_item_id, count - 1));
        }
        if data.regrow_days > 0 {
            self.regrow_left = data.regrow_days;
            self.harvested_once = true;
            Some((items, false))
        } else {
            Some((items, true))
        }
    }

    /// Frame index in the 8-frame crop strip.
    pub fn sprite_frame(&self, data: &CropData) -> i32 {
        let phases = self.phases(data).len() as i32;
        if self.phase == 0 && self.day_in_phase == 0 && !self.harvested_once {
            return self.variant as i32;
        }
        if (self.phase as i32) < phases {
            return (self.phase as i32 + 1).min(phases);
        }
        if self.regrow_left > 0 {
            // Regrowing: shown as the last growth phase.
            return phases;
        }
        (phases + 1).min(7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(phases: &[i32], regrow: i32) -> CropData {
        CropData {
            seasons: vec!["Spring".into()],
            days_in_phase: phases.to_vec(),
            regrow_days: regrow,
            harvest_item_id: "X".into(),
            ..Default::default()
        }
    }

    #[test]
    fn grows_only_when_watered_and_in_season() {
        let d = data(&[1, 1, 1, 1], -1);
        let mut rng = Rng::new(1);
        let mut c = Crop::new("S", &mut rng);
        assert_eq!(c.days_until_ready(&d), 4);
        c.new_day(&d, false, Season::Spring, true);
        assert_eq!(c.days_until_ready(&d), 4);
        for _ in 0..4 {
            c.new_day(&d, true, Season::Spring, true);
        }
        assert!(c.is_ready(&d));
        let (items, remove) = c.harvest(&d, &mut rng, 0, 0).unwrap();
        assert!(remove);
        assert_eq!(items[0].object_id(), Some("X"));
        let mut c2 = Crop::new("S", &mut rng);
        c2.new_day(&d, true, Season::Summer, true);
        assert!(c2.dead);
    }

    #[test]
    fn regrows() {
        let d = data(&[1, 1], 3);
        let mut rng = Rng::new(2);
        let mut c = Crop::new("S", &mut rng);
        c.new_day(&d, true, Season::Spring, true);
        c.new_day(&d, true, Season::Spring, true);
        assert!(c.is_ready(&d));
        let (_, remove) = c.harvest(&d, &mut rng, 0, 0).unwrap();
        assert!(!remove);
        assert!(!c.is_ready(&d));
        for _ in 0..3 {
            c.new_day(&d, true, Season::Spring, true);
        }
        assert!(c.is_ready(&d));
    }

    #[test]
    fn zero_length_phases_are_skipped() {
        let d = data(&[1, 0, 1], -1);
        let mut rng = Rng::new(3);
        let mut c = Crop::new("S", &mut rng);
        c.new_day(&d, true, Season::Spring, true);
        assert_eq!(c.phase, 2);
        c.new_day(&d, true, Season::Spring, true);
        assert!(c.is_ready(&d));
    }
}
