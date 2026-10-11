//! Dating, engagement, weddings and spouse life.

use crate::dialogue::{self, DialoguePage, Emotion, Substitutions};
use crate::farmer::RelationStatus;
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
        if !nearby {
            return;
        }
        let f = self.farmer.friendship(&spouse);
        f.points = (f.points - JEALOUSY_POINTS).max(0);
        self.say(&spouse, "...I saw that.", Emotion::Angry);
    }
}

/// Filing fee for a divorce (approx.).
pub const DIVORCE_COST: i32 = 50_000;
/// Friendship the spouse loses when they see a gift to a rival (approx.).
pub const JEALOUSY_POINTS: i32 = 30;
pub const JEALOUSY_RANGE: i32 = 10;
