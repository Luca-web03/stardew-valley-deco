//! Dating, engagement, weddings and spouse life.

use crate::dialogue::{self, DialoguePage, Emotion, Substitutions};
use crate::farmer::RelationStatus;
use crate::item::Item;
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Well-known bouquet / pendant ids, with placeholders last.
const BOUQUET: &[&str] = &["458", "Dev_Bouquet"];
const PENDANT: &[&str] = &["460", "Dev_Pendant"];

impl World {
    fn say(&mut self, npc: &str, text: &str, emotion: Emotion) {
        self.emit(WorldEvent::Dialogue { npc: Some(npc.to_string()), pages: vec![DialoguePage { text: text.to_string(), emotion }] });
    }

    /// Bouquet / pendant handling. Returns true if the item was a romance item.
    pub(crate) fn try_romance_item(&mut self, i: usize, item_id: &str) -> bool {
        let is_bouquet = BOUQUET.contains(&item_id);
        let is_pendant = PENDANT.contains(&item_id);
        if !is_bouquet && !is_pendant {
            return false;
        }
        let name = self.npcs[i].name.clone();
        let romanceable = self.npcs[i].can_romance;
        let slot = self.farmer.current_slot;
        let f = self.farmer.friendship(&name).clone();
        if !romanceable {
            self.say(&name, "That's sweet, but I don't think of you that way.", Emotion::Neutral);
            return true;
        }
        if f.status == RelationStatus::Divorced {
            self.say(&name, "After everything that happened? No.", Emotion::Angry);
            return true;
        }
        if is_bouquet {
            match f.status {
                RelationStatus::Friendly if f.hearts() >= 8 => {
                    self.farmer.inventory.reduce(slot, 1);
                    self.farmer.friendship(&name).status = RelationStatus::Dating;
                    self.farmer.friendship(&name).points += 25;
                    self.say(&name, "For me? ...Yes. I'd like that very much.", Emotion::Love);
                }
                RelationStatus::Friendly => self.say(&name, "Oh... I don't know you well enough for that yet.", Emotion::Sad),
                _ => self.say(&name, "You already gave me one. I kept it!", Emotion::Happy),
            }
            return true;
        }
        // Pendant.
        if self.farmer.spouse.is_some() || self.farmer.friendships.values().any(|x| matches!(x.status, RelationStatus::Engaged { .. })) {
            self.say(&name, "Aren't you already spoken for?", Emotion::Angry);
            return true;
        }
        match f.status {
            RelationStatus::Dating if f.hearts() >= 10 => {
                self.farmer.inventory.reduce(slot, 1);
                let day = self.clock.days_played() + 3;
                self.farmer.friendship(&name).status = RelationStatus::Engaged { wedding_day: day };
                self.say(&name, "...Yes! A thousand times yes! Let's have the wedding in three days.", Emotion::Love);
            }
            RelationStatus::Dating => self.say(&name, "I'm happy with how things are, but I'm not ready for that.", Emotion::Neutral),
            _ => self.say(&name, "What? We aren't even dating!", Emotion::Sad),
        }
        true
    }

    /// Morning: hold the wedding if it's the day.
    pub(crate) fn check_wedding(&mut self) {
        let today = self.clock.days_played();
        let due: Option<String> = self
            .farmer
            .friendships
            .iter()
            .find(|(_, f)| matches!(f.status, RelationStatus::Engaged { wedding_day } if wedding_day <= today))
            .map(|(k, _)| k.clone());
        let Some(name) = due else { return };
        self.farmer.friendship(&name).status = RelationStatus::Married;
        self.farmer.spouse = Some(name.clone());
        if self.play_wedding(&name) {
            return;
        }
        let display = self.npcs.iter().find(|n| n.name == name).map(|n| n.display_name.clone()).unwrap_or(name.clone());
        self.emit(WorldEvent::Message(format!("Friends gather in town as you and {display} exchange vows. {display} moves into your farmhouse.")));
    }

    /// Daily spouse placement and greeting.
    pub(crate) fn spouse_morning(&mut self) {
        let Some(spouse) = self.farmer.spouse.clone() else { return };
        let Some(i) = self.npcs.iter().position(|n| n.name == spouse) else { return };
        let Some(house) = self.locations.get("FarmHouse") else { return };
        // Stand somewhere open near the kitchen / middle of the house.
        let target = (0..house.height())
            .flat_map(|y| (0..house.width()).map(move |x| TilePos::new(x, y)))
            .filter(|p| crate::collision::tile_passable(house, *p) && house.tile_prop("Back", *p, "Bed").is_none())
            .min_by_key(|p| (p.x - house.width() / 2).abs() + (p.y - 4).abs())
            .unwrap_or(TilePos::new(3, 4));
        let npc = &mut self.npcs[i];
        npc.location = "FarmHouse".into();
        npc.set_tile(target);
        npc.schedule.clear();
        npc.route.clear();
        npc.home.location = "FarmHouse".into();
        npc.home.tile = target;
        // Morning line from MarriageDialogue<Name> (or a generic one).
        let key_candidates = if self.weather.is_raining() { vec!["Rainy_Day_0", "Rainy_Day_1", "Indoor_Day_0"] } else { vec!["Indoor_Day_0", "Indoor_Day_1", "Outdoor_0"] };
        let text = key_candidates.iter().find_map(|k| self.spouse_dialogue.get(&spouse).and_then(|d| d.get(*k)).cloned());
        let text = text.unwrap_or_else(|| "Good morning, dear. I'll keep things running here today.$h".to_string());
        let subs = Substitutions { farmer_name: &self.farmer.name, farm_name: &self.farmer.farm_name, male: self.farmer.male };
        let pages = dialogue::parse(&text, &subs);
        npc.dialogue.insert("__spouse_today".into(), pages.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join("#$b#"));
    }

    /// Paperwork at town hall: end the marriage (approx. cost). The former
    /// spouse moves back to their own home tomorrow.
    pub fn divorce(&mut self) -> Result<(), String> {
        let Some(spouse) = self.farmer.spouse.clone() else { return Err("You aren't married.".into()) };
        if self.farmer.money < DIVORCE_COST {
            return Err(format!("The filing fee is {DIVORCE_COST}g."));
        }
        self.farmer.money -= DIVORCE_COST;
        self.farmer.spouse = None;
        let f = self.farmer.friendship(&spouse);
        f.status = RelationStatus::Divorced;
        f.points = 0;
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.name == spouse) {
            npc.home = npc.original_home.clone();
            npc.dialogue.remove("__spouse_today");
        }
        self.emit(WorldEvent::Message("The papers are filed. Your former spouse will move out tomorrow.".into()));
        Ok(())
    }

    /// Gifts to another romanceable villager in front of the spouse sting.
    pub(crate) fn check_jealousy(&mut self, recipient: &str) {
        let Some(spouse) = self.farmer.spouse.clone() else { return };
        if spouse == recipient || !self.npcs.iter().any(|n| n.name == recipient && n.can_romance) {
            return;
        }
        let me = self.farmer.tile();
        let nearby = self.npcs.iter().any(|n| n.name == spouse && n.location == self.current && n.tile().manhattan(me) <= JEALOUSY_RANGE);
        let (jealous, change) = self.spouse_jealousy(&spouse);
        if !nearby || !jealous {
            return;
        }
        let f = self.farmer.friendship(&spouse);
        f.points = (f.points + change).max(0);
        self.say(&spouse, "...I saw that.", Emotion::Angry);
    }
}

/// Spouse chores (approx.): a spouse at 10+ hearts may, on 40% of mornings,
/// do one of: water the farm's crops (not on rainy days), fill the troughs
/// from the silos, mend the fences, fill the pet's bowl, or make breakfast.
pub const CHORE_MIN_POINTS: i32 = 2500;
pub const CHORE_CHANCE: f64 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chore {
    Water,
    Feed,
    Fences,
    PetBowl,
    Breakfast,
}

impl World {
    /// Morning: a happy spouse helps out. Returns the chore done.
    pub(crate) fn spouse_chores(&mut self) -> Option<Chore> {
        let spouse = self.farmer.spouse.clone()?;
        let points = self.farmer.friendships.get(&spouse).map(|f| f.points).unwrap_or(0);
        if points < CHORE_MIN_POINTS {
            return None;
        }
        let mut rng = self.day_rng(4040);
        if !rng.chance(CHORE_CHANCE) {
            return None;
        }
        let mut options = vec![Chore::Breakfast];
        let farm_has_dry_crops = self.locations.get("Farm").is_some_and(|f| f.state.terrain.values().any(|t| matches!(t, crate::terrain::TerrainFeature::HoeDirt(d) if !d.watered && d.crop.is_some())));
        if !self.weather.is_raining() && farm_has_dry_crops {
            options.push(Chore::Water);
        }
        if self.farmer.hay_storage > 0 && !self.animals.is_empty() {
            options.push(Chore::Feed);
        }
        let worn = self.locations.values().any(|l| {
            l.outdoors && l.state.objects.values().any(|o| matches!(o.kind, crate::objects::ObjKind::Fence { health, .. } if health < self.fences.get(&o.item_id).map_or(1, |d| d.health)))
        });
        if worn {
            options.push(Chore::Fences);
        }
        if self.pet.as_ref().is_some_and(|p| !p.bowl_filled) {
            options.push(Chore::PetBowl);
        }
        let chore = *rng.pick(&options)?;
        let line = match chore {
            Chore::Water => {
                if let Some(f) = self.locations.get_mut("Farm") {
                    for t in f.state.terrain.values_mut() {
                        if let crate::terrain::TerrainFeature::HoeDirt(d) = t {
                            if d.crop.is_some() {
                                d.watered = true;
                            }
                        }
                    }
                }
                "I got up early and watered the crops for you."
            }
            Chore::Feed => {
                let hay = self.hay_id.clone();
                let homes: Vec<String> = self.animals.iter().map(|a| a.home.clone()).collect();
                if let Some(hay) = hay {
                    for home in homes {
                        let Some(loc) = self.locations.get(&home) else { continue };
                        let troughs: Vec<crate::TilePos> = (0..loc.height())
                            .flat_map(|y| (0..loc.width()).map(move |x| crate::TilePos::new(x, y)))
                            .filter(|p| (loc.tile_prop("Back", *p, crate::farm::TROUGH_PROP).is_some() || loc.tile_prop("Buildings", *p, crate::farm::TROUGH_PROP).is_some()) && loc.object(*p).is_none())
                            .collect();
                        for t in troughs {
                            if self.farmer.hay_storage <= 0 {
                                break;
                            }
                            self.farmer.hay_storage -= 1;
                            if let Some(l) = self.locations.get_mut(&home) {
                                l.state.objects.insert(t, crate::objects::PlacedObject::new(&hay, false, crate::objects::ObjKind::Placed));
                            }
                        }
                    }
                }
                "I fed the animals this morning."
            }
            Chore::Fences => {
                let data: std::collections::BTreeMap<String, i32> = self.fences.iter().map(|(k, d)| (k.clone(), d.health.max(1))).collect();
                for loc in self.locations.values_mut().filter(|l| l.outdoors) {
                    for o in loc.state.objects.values_mut() {
                        if let crate::objects::ObjKind::Fence { health, .. } = &mut o.kind {
                            *health = (*health).max(data.get(&o.item_id).copied().unwrap_or(50));
                        }
                    }
                }
                "I mended the fences while you were asleep."
            }
            Chore::PetBowl => {
                if let Some(p) = self.pet.as_mut() {
                    p.bowl_filled = true;
                }
                "I filled the pet's water bowl."
            }
            Chore::Breakfast => {
                let dishes: Vec<String> = self.items.objects.iter().filter(|(_, o)| o.data.category == -7).map(|(k, _)| k.clone()).collect();
                if let Some(d) = rng.pick(&dishes).cloned() {
                    self.give_item(Item::object(&d, 1));
                }
                "I made you breakfast. Eat up!"
            }
        };
        if let Some(npc) = self.npcs.iter_mut().find(|n| n.name == spouse) {
            npc.dialogue.insert("__spouse_today".into(), format!("{line}$h"));
        }
        Some(chore)
    }

    /// Run the morning chores now (tools and tests).
    pub fn spouse_chores_pub(&mut self) -> Option<Chore> {
        self.spouse_chores()
    }
}

/// Filing fee for a divorce (approx.).
pub const DIVORCE_COST: i32 = 50_000;
pub const JEALOUSY_RANGE: i32 = 10;
