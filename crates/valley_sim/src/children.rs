//! Children: after marriage the spouse may ask about a baby; the child
//! arrives two weeks later and grows through stages. Chances and stage
//! lengths are engine approximations kept as constants.

use engine::{Rng, Vec2};
use serde::{Deserialize, Serialize};

use crate::collision::{box_blocked, tile_passable};
use crate::dialogue::{Choice, Question, Reaction};
use crate::farm::FarmAnimal;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos, TILE_F};

pub const MAX_CHILDREN: usize = 2;
pub const DAYS_UNTIL_BIRTH: i32 = 14;
/// Spouse friendship needed before they bring it up.
pub const ASK_FRIENDSHIP: i32 = 2500;
/// Nightly chance the spouse asks (approx.).
pub const ASK_CHANCE: f64 = 0.05;
/// A new baby only once the youngest is at least this old.
pub const MIN_GAP_DAYS: i32 = 56;
const BABY_DAYS: i32 = 13;
const SITTER_DAYS: i32 = 27;
const CRAWLER_DAYS: i32 = 55;
/// Invented placeholder names, picked when a child is born.
const NAMES: &[&str] = &["Pip", "Wren", "Juniper", "Bram", "Clover", "Rowan", "Sorrel", "Tamsin"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildStage {
    Newborn,
    Sitter,
    Crawler,
    Toddler,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Child {
    pub name: String,
    /// `days_played` on the day of birth.
    pub born: i32,
    /// Appearance variant (row block in the sheet).
    pub variant: i32,
    pub pos: Vec2,
    pub facing: Dir,
    pub friendship: i32,
    pub talked_today: bool,
    #[serde(skip)]
    pub moving: bool,
    #[serde(skip)]
    pub walk_ms: u32,
    #[serde(skip)]
    pub target: Option<Vec2>,
    #[serde(skip)]
    pub wait_ms: u32,
}

impl Child {
    pub fn age(&self, today: i32) -> i32 {
        today - self.born
    }

    pub fn stage(&self, today: i32) -> ChildStage {
        match self.age(today) {
            a if a <= BABY_DAYS => ChildStage::Newborn,
            a if a <= SITTER_DAYS => ChildStage::Sitter,
            a if a <= CRAWLER_DAYS => ChildStage::Crawler,
            _ => ChildStage::Toddler,
        }
    }
}

impl World {
    /// House level that has room for a crib: 2, or the best available map.
    fn nursery_level(&self) -> i32 {
        let best = self.house_maps.keys().copied().max().unwrap_or(0);
        2.min(best.max(1))
    }

    /// Where the crib is: the `CribTile` farmhouse map property, else a
    /// free tile a few steps from the bed.
    pub fn crib_tile(&self) -> TilePos {
        let house = self.locations.get("FarmHouse");
        if let Some(t) = house.and_then(|h| h.map_prop_tile("CribTile")) {
            return t;
        }
        let bed = self.bed_tile;
        let Some(h) = house else { return bed };
        (0..h.height())
            .flat_map(|y| (0..h.width()).map(move |x| TilePos::new(x, y)))
            .filter(|p| tile_passable(h, *p) && p.manhattan(bed) >= 3 && h.tile_prop("Back", *p, "Bed").is_none())
            .min_by_key(|p| ((p.x - bed.x - 3).abs() + (p.y - bed.y).abs(), p.y, p.x))
            .unwrap_or(bed)
    }

    fn can_have_child(&self) -> bool {
        let Some(spouse) = &self.farmer.spouse else { return false };
        let points = self.farmer.friendships.get(spouse).map(|f| f.points).unwrap_or(0);
        let today = self.clock.days_played();
        self.farmer.house_level >= self.nursery_level()
            && points >= ASK_FRIENDSHIP
            && self.farmer.children.len() < MAX_CHILDREN
            && self.farmer.baby_due.is_none()
            && self.farmer.children.iter().all(|c| c.age(today) >= MIN_GAP_DAYS)
    }

    /// Night: maybe the spouse brings it up tomorrow.
    pub(crate) fn children_night(&mut self) {
        let mut rng = self.day_rng(41);
        self.child_question_pending = self.can_have_child() && rng.chance(ASK_CHANCE);
    }

    /// Talking to the spouse on an asking day poses the question.
    pub(crate) fn maybe_ask_about_child(&mut self, spouse: &str) -> bool {
        if !self.child_question_pending || self.farmer.spouse.as_deref() != Some(spouse) {
            return false;
        }
        self.child_question_pending = false;
        let q = Question {
            prompt: "I've been thinking... what would you say to us having a baby?".into(),
            choices: vec![
                Choice { id: "__child_yes".into(), text: "Let's do it.".into(), friendship: 0, reaction: Reaction::Text("Really? Oh, I'm so happy!$h".into()) },
                Choice { id: "__child_no".into(), text: "Not right now.".into(), friendship: 0, reaction: Reaction::Text("Okay. Maybe some other time.".into()) },
            ],
        };
        self.pending_question = Some((spouse.to_string(), q.clone()));
        self.emit(WorldEvent::DialogueQuestion { npc: spouse.to_string(), prompt: q.prompt, choices: q.choices.iter().map(|c| c.text.clone()).collect() });
        true
    }

    /// Answer hook for the special choice ids.
    pub(crate) fn child_answer(&mut self, id: &str) {
        if id == "__child_yes" && self.farmer.baby_due.is_none() {
            self.farmer.baby_due = Some(self.clock.days_played() + DAYS_UNTIL_BIRTH);
        }
    }

    /// Morning: births, daily resets and placement.
    pub(crate) fn children_morning(&mut self) {
        let today = self.clock.days_played();
        if self.farmer.baby_due.is_some_and(|d| today >= d) {
            self.farmer.baby_due = None;
            let used: Vec<String> = self.farmer.children.iter().map(|c| c.name.clone()).collect();
            let mut rng = self.day_rng(43);
            let free: Vec<&str> = NAMES.iter().copied().filter(|n| !used.iter().any(|u| u == n)).collect();
            let name = rng.pick(&free).copied().unwrap_or("Baby").to_string();
            let variant = rng.range(0, 2);
            self.farmer.children.push(Child {
                name: name.clone(),
                born: today,
                variant,
                pos: Vec2::ZERO,
                facing: Dir::Down,
                friendship: 0,
                talked_today: false,
                moving: false,
                walk_ms: 0,
                target: None,
                wait_ms: 0,
            });
            self.emit(WorldEvent::Message(format!("Your baby has arrived! You named them {name}.")));
        }
        let crib = self.crib_tile();
        let mut taken: Vec<TilePos> = Vec::new();
        let house = self.locations.get("FarmHouse");
        for c in self.farmer.children.iter_mut() {
            c.talked_today = false;
            c.target = None;
            // Babies stay in the crib; older children start on open floor near it.
            let spot = match (c.stage(today), house) {
                (ChildStage::Newborn | ChildStage::Sitter, _) | (_, None) => crib,
                (_, Some(h)) => (0..h.height())
                    .flat_map(|y| (0..h.width()).map(move |x| TilePos::new(x, y)))
                    .filter(|p| *p != crib && !taken.contains(p) && tile_passable(h, *p))
                    .min_by_key(|p| (p.manhattan(TilePos::new(crib.x, crib.y + 1)), p.y, p.x))
                    .unwrap_or(crib),
            };
            taken.push(spot);
            c.pos = Vec2::new(spot.x as f32 * TILE_F, spot.y as f32 * TILE_F - 16.0);
        }
    }

    /// Debug/console: add a child that is `age` days old.
    pub fn debug_add_child(&mut self, age: i32) -> String {
        if self.farmer.children.len() >= MAX_CHILDREN {
            return "no room for more children".into();
        }
        self.farmer.baby_due = Some(self.clock.days_played());
        self.children_morning();
        if let Some(c) = self.farmer.children.last_mut() {
            c.born -= age.max(0);
        }
        self.children_morning();
        let c = &self.farmer.children[self.farmer.children.len() - 1];
        format!("{} ({} days old)", c.name, age)
    }

    /// Talk to (or pick up) a child.
    pub(crate) fn interact_child(&mut self, t: TilePos) -> bool {
        if self.current != "FarmHouse" {
            return false;
        }
        let today = self.clock.days_played();
        let me = self.farmer.tile();
        let Some(i) = self.farmer.children.iter().position(|c| {
            let ct = TilePos::from_world(FarmAnimal::bbox_at(c.pos).center());
            ct == t || (ct.manhattan(me) <= 1 && ct.manhattan(t) <= 1)
        }) else {
            return false;
        };
        let c = &mut self.farmer.children[i];
        let line = match c.stage(today) {
            ChildStage::Newborn => format!("{} is fast asleep.", c.name),
            ChildStage::Sitter => format!("{} giggles and reaches for you.", c.name),
            ChildStage::Crawler => format!("{} babbles happily.", c.name),
            ChildStage::Toddler => format!("{}: \"Play with me!\"", c.name),
        };
        if !c.talked_today {
            c.talked_today = true;
            c.friendship = (c.friendship + 20).min(2500);
        }
        self.emit(WorldEvent::Message(line));
        true
    }

    /// Crawlers and toddlers wander the house.
    pub(crate) fn update_children(&mut self, dt: u32) {
        if self.farmer.children.is_empty() {
            return;
        }
        let today = self.clock.days_played();
        let Some(house) = self.locations.get("FarmHouse") else { return };
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.game_ms as i64, 317]));
        for c in self.farmer.children.iter_mut() {
            c.moving = false;
            let speed = match c.stage(today) {
                ChildStage::Crawler => 0.8,
                ChildStage::Toddler => 1.6,
                _ => continue,
            };
            c.wait_ms = c.wait_ms.saturating_sub(dt);
            if c.target.is_none() && c.wait_ms == 0 {
                let d = Vec2::new(rng.range(-3, 4) as f32, rng.range(-3, 4) as f32) * TILE_F;
                c.target = Some(c.pos + d);
                c.wait_ms = rng.range(1500, 5000) as u32;
            }
            let Some(t) = c.target else { continue };
            let d = t - c.pos;
            if d.length() < 2.0 {
                c.target = None;
                continue;
            }
            let v = d.normalized() * speed;
            let np = c.pos + v;
            if box_blocked(house, FarmAnimal::bbox_at(np)) {
                c.target = None;
            } else {
                c.pos = np;
                c.moving = true;
                c.walk_ms += dt;
                c.facing = Dir::toward((v.x * 10.0) as i32, (v.y * 10.0) as i32);
            }
        }
    }
}
