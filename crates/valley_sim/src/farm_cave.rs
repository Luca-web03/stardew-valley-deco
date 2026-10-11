//! The farm cave: once the farm has earned 25,000g, entering the cave
//! (location `FarmCave`) offers a one-time choice between mushrooms and
//! fruit bats.
//!
//! * Mushrooms: mushroom boxes (big craftable `128`, or one with the
//!   `Port.MushroomBox` custom field) are placed on free cave tiles; they
//!   produce through their `Data/Machines` rules (`DayUpdate`).
//! * Fruit bats: each morning a few fruits (category -79) appear on the
//!   cave floor (approx. 0-3 a day).

use engine::Rng;

use crate::objects::{ObjKind, PlacedObject};
use crate::world::{World, WorldEvent};
use crate::TilePos;

pub const CAVE: &str = "FarmCave";
pub const CAVE_EARNINGS: i64 = 25_000;
pub const MUSHROOM_BOXES: usize = 6;

impl World {
    /// Entering the cave: offer the choice once it's due.
    pub(crate) fn check_cave_choice(&mut self) {
        if self.current != CAVE || self.farmer.cave_choice.is_some() || self.farmer.total_earned < CAVE_EARNINGS || self.cave_offered {
            return;
        }
        self.cave_offered = true;
        self.emit(WorldEvent::Choice {
            text: "A naturalist from town would like to study your cave, and offers to bring something to live in it. What should it be?".into(),
            options: vec![("cave:mushrooms".into(), "Mushrooms".into()), ("cave:bats".into(), "Fruit bats".into())],
        });
    }

    pub fn choose_cave(&mut self, choice: &str) {
        if self.farmer.cave_choice.is_some() || !matches!(choice, "mushrooms" | "bats") {
            return;
        }
        self.farmer.cave_choice = Some(choice.to_string());
        if choice == "mushrooms" {
            self.place_mushroom_boxes();
        }
        self.sound("newArtifact");
        self.emit(WorldEvent::Hud(if choice == "mushrooms" { "Mushrooms will grow in the cave." } else { "Fruit bats have moved into the cave." }.into()));
    }

    fn mushroom_box_id(&self) -> Option<String> {
        if self.items.big("128").is_some() {
            return Some("128".into());
        }
        self.items.bigs.iter().find(|(_, b)| b.data.custom_fields.as_ref().is_some_and(|f| f.contains_key("Port.MushroomBox"))).map(|(k, _)| k.clone())
    }

    fn place_mushroom_boxes(&mut self) {
        let Some(id) = self.mushroom_box_id() else { return };
        let machine = self.items.machines.contains_key(&format!("(BC){id}"));
        let Some(cave) = self.locations.get_mut(CAVE) else { return };
        let mut free: Vec<TilePos> = (1..cave.height() - 1)
            .flat_map(|y| (1..cave.width() - 1).map(move |x| TilePos::new(x, y)))
            .filter(|p| cave.is_clear(*p) && crate::collision::tile_passable(cave, *p))
            .collect();
        // Leave the entrance row free.
        free.retain(|p| p.y < cave.height() - 3);
        let spots: Vec<TilePos> = free.into_iter().step_by(2).take(MUSHROOM_BOXES).collect();
        for p in &spots {
            let kind = if machine { ObjKind::Machine { output: None, minutes_left: 0, ready: false } } else { ObjKind::Placed };
            cave.state.objects.insert(*p, PlacedObject::new(&id, true, kind));
        }
        for p in spots {
            self.machine_auto_trigger(CAVE, p, "MachinePutDown", None);
        }
    }

    /// Morning: fruit bats drop fruit.
    pub(crate) fn cave_morning(&mut self) {
        if self.farmer.cave_choice.as_deref() != Some("bats") {
            return;
        }
        let mut rng: Rng = self.day_rng(149);
        let fruits: Vec<String> = self.items.objects.iter().filter(|(_, o)| o.data.category == -79 && o.data.price > 0).map(|(k, _)| k.clone()).collect();
        if fruits.is_empty() {
            return;
        }
        let n = rng.range(0, 4);
        let Some(cave) = self.locations.get_mut(CAVE) else { return };
        for _ in 0..n {
            let Some(p) = cave.free_area(1, 1, |len| rng.range(0, len as i32) as usize).or_else(|| {
                let free: Vec<TilePos> = (0..cave.height()).flat_map(|y| (0..cave.width()).map(move |x| TilePos::new(x, y))).filter(|p| cave.is_clear(*p) && crate::collision::tile_passable(cave, *p)).collect();
                (!free.is_empty()).then(|| free[rng.range(0, free.len() as i32) as usize])
            }) else { break };
            let fruit = &fruits[rng.range(0, fruits.len() as i32) as usize];
            cave.state.objects.insert(p, PlacedObject::new(fruit, false, ObjKind::Forage));
        }
    }
}
