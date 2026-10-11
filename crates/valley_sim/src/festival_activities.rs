//! Festival activities. The original runs these from code, so each
//! festival key gets its activity by default (spring13 egg hunt, spring24
//! dance, summer11 soup, fall16 grange display, winter8 ice fishing,
//! winter25 gift exchange); a festival's data can pick one with
//! `Port.Activity` (EggHunt, Dance, Soup, Grange, IceFishing, Gifts) and
//! name a prize with `Port.Prize` (qualified item id). Scores and rewards
//! are approximations.

use engine::Rng;

use crate::events::EventProp;
use crate::farmer::Farmer;
use crate::inventory::Inventory;
use crate::item::Item;
use crate::world::{World, WorldEvent};
use crate::TilePos;

/// Grange display container (chest menus address it by this tile).
pub const GRANGE_TILE: TilePos = TilePos { x: -3, y: -3 };
pub const GRANGE_SLOTS: usize = 9;
pub const EGG_HUNT_MS: u32 = 50_000;
pub const EGGS: usize = 24;
pub const EGGS_TO_WIN: i32 = 9;
pub const DANCE_HEARTS: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    EggHunt,
    Dance,
    Soup,
    Grange,
    IceFishing,
    Gifts,
}

pub fn activity_for(key: &str, data: &std::collections::HashMap<String, String>) -> Option<Activity> {
    let named = data.get("Port.Activity").map(|s| s.trim().to_ascii_lowercase());
    let by_name = |n: &str| match n {
        "egghunt" => Some(Activity::EggHunt),
        "dance" => Some(Activity::Dance),
        "soup" => Some(Activity::Soup),
        "grange" => Some(Activity::Grange),
        "icefishing" => Some(Activity::IceFishing),
        "gifts" => Some(Activity::Gifts),
        _ => None,
    };
    if let Some(n) = named {
        return by_name(&n);
    }
    match key {
        "spring13" => Some(Activity::EggHunt),
        "spring24" => Some(Activity::Dance),
        "summer11" => Some(Activity::Soup),
        "fall16" => Some(Activity::Grange),
        "winter8" => Some(Activity::IceFishing),
        "winter25" => Some(Activity::Gifts),
        _ => None,
    }
}

/// Per-festival activity state.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ActivityState {
    pub soup: Option<Item>,
    pub grange: Option<Inventory>,
    pub hunt_ms: u32,
    pub eggs_found: i32,
    pub hunting: bool,
    pub partner: Option<String>,
    pub gift_given: bool,
}

/// Grange score (approx.): a base, points per item by quality and value,
/// a bonus per distinct category, and one per filled slot.
pub fn grange_score(world: &World, items: &[Item]) -> i32 {
    let mut score = 14;
    let mut cats = std::collections::BTreeSet::new();
    for it in items {
        let price = world.items.sell_price(it);
        score += match price {
            p if p >= 300 => 4,
            p if p >= 100 => 3,
            p if p >= 30 => 2,
            _ => 1,
        };
        score += match it.quality() {
            crate::item::Quality::Iridium => 3,
            crate::item::Quality::Gold => 2,
            crate::item::Quality::Silver => 1,
            crate::item::Quality::Normal => 0,
        };
        if let Some(o) = it.object_id().and_then(|id| world.items.object(id)) {
            cats.insert(o.data.category);
        }
    }
    score + 5 * cats.len().min(6) as i32 + items.len().min(GRANGE_SLOTS) as i32
}

impl World {
    pub(crate) fn festival_activity(&self) -> Option<Activity> {
        let f = self.event.as_ref()?.festival.as_ref()?;
        activity_for(&f.key, &f.dialogue)
    }

    fn festival_prize(&self) -> Option<Item> {
        let f = self.event.as_ref()?.festival.as_ref()?;
        let id = f.dialogue.get("Port.Prize")?;
        self.items.create(id, 1)
    }

    /// Talking to the host: offer the activity before the main event.
    pub(crate) fn festival_host_talk(&mut self, festival: &str) {
        let mut options = vec![("festival:main".to_string(), "Start the main event".to_string())];
        match self.festival_activity() {
            Some(Activity::Soup) if self.activity.soup.is_none() => options.insert(0, ("festival:soup".into(), "Add something to the soup".into())),
            Some(Activity::Grange) => options.insert(0, ("festival:grange".into(), "Set up your grange display".into())),
            _ => {}
        }
        if options.len() == 1 {
            self.emit(WorldEvent::FestivalHost { festival: festival.to_string() });
        } else {
            self.emit(WorldEvent::Choice { text: format!("Welcome to the {festival}!"), options });
        }
    }

    /// Roaming: talking to someone during a dance or gift festival.
    /// Returns true when the talk was used by the activity.
    pub(crate) fn festival_villager_talk(&mut self, name: &str) -> bool {
        match self.festival_activity() {
            Some(Activity::Dance) if self.activity.partner.is_none() => {
                let romanceable = self.npcs.iter().any(|n| n.name == name && self.can_flower_dance(n));
                if romanceable {
                    let display = self.npc_display(name);
                    self.emit(WorldEvent::Choice { text: format!("Ask {display} to dance?"), options: vec![(format!("festival:dance:{name}"), "Ask".into())] });
                    return true;
                }
                false
            }
            Some(Activity::Gifts) if !self.activity.gift_given && self.farmer.secret_friend.as_deref() == Some(name) => {
                let slot = self.farmer.current_slot;
                let Some(Item::Object { id, .. }) = self.farmer.inventory.get(slot).cloned() else { return false };
                self.farmer.inventory.reduce(slot, 1);
                let taste = self.gift_taste(name, &id);
                let points = match taste {
                    0 => 160,
                    1 => 90,
                    2 => -40,
                    3 => -80,
                    _ => 40,
                };
                let f = self.farmer.friendship(name);
                f.points = (f.points + points).clamp(0, 2500);
                self.activity.gift_given = true;
                let display = self.npc_display(name);
                let line = if taste <= 1 { "Oh! You're my secret friend? This is wonderful, thank you!" } else { "Oh... you shouldn't have. Really." };
                self.emit(WorldEvent::Dialogue { npc: Some(name.to_string()), pages: vec![crate::dialogue::DialoguePage { text: format!("{line}"), emotion: if taste <= 1 { crate::dialogue::Emotion::Happy } else { crate::dialogue::Emotion::Neutral } }] });
                self.hud(format!("You gave {display} your secret gift."));
                true
            }
            _ => false,
        }
    }

    fn npc_display(&self, name: &str) -> String {
        self.npcs.iter().find(|n| n.name == name).map(|n| n.display_name.clone()).unwrap_or(name.to_string())
    }

    /// Festival choices (`festival:*` ids from [`World::choose`]).
    pub(crate) fn choose_festival(&mut self, what: &str) {
        match what {
            "main" => self.festival_begin_main(),
            "soup" => {
                let slot = self.farmer.current_slot;
                match self.farmer.inventory.get(slot).cloned() {
                    Some(it @ Item::Object { .. }) => {
                        self.farmer.inventory.reduce(slot, 1);
                        let mut one = it.clone();
                        one.set_stack(1);
                        let name = self.items.display_name(&one);
                        self.activity.soup = Some(one);
                        self.hud(format!("You added {name} to the soup."));
                        self.sound("dropItemInWater");
                    }
                    _ => self.emit(WorldEvent::Message("Hold the item you want to add to the soup.".into())),
                }
            }
            "grange" => {
                self.activity.grange.get_or_insert_with(|| Inventory::new(GRANGE_SLOTS));
                self.emit(WorldEvent::OpenChest(GRANGE_TILE));
            }
            other => {
                if let Some(name) = other.strip_prefix("dance:") {
                    let hearts = self.farmer.friendships.get(name).map(|f| f.hearts()).unwrap_or(0);
                    let display = self.npc_display(name);
                    if hearts >= DANCE_HEARTS {
                        self.activity.partner = Some(name.to_string());
                        self.emit(WorldEvent::Dialogue { npc: Some(name.to_string()), pages: vec![crate::dialogue::DialoguePage { text: "I'd love to! See you on the dance floor.".into(), emotion: crate::dialogue::Emotion::Happy }] });
                    } else {
                        self.emit(WorldEvent::Dialogue { npc: Some(name.to_string()), pages: vec![crate::dialogue::DialoguePage { text: "Sorry, I don't think so.".into(), emotion: crate::dialogue::Emotion::Sad }] });
                        self.hud(format!("{display} turned you down."));
                    }
                }
            }
        }
    }

    /// The main event begins: resolve the activity (an egg hunt runs
    /// first). Returns true when the script should wait for the UI.
    pub(crate) fn festival_main_activity(&mut self) -> bool {
        let Some(act) = self.festival_activity() else { return false };
        let before = self.events.len();
        let prize = self.festival_prize();
        match act {
            Activity::EggHunt => {
                self.start_egg_hunt();
                return true;
            }
            Activity::Soup => {
                let item = self.activity.soup.take();
                let score = item.as_ref().map(|i| self.items.sell_price(i) / 10 + i.quality() as i32 * 4).unwrap_or(-5);
                let (points, verdict) = match score {
                    s if s >= 25 => (120, "Magnificent! This is the best soup I've had in years!"),
                    s if s >= 10 => (60, "Mmm, a fine soup. Well done, everyone."),
                    s if s >= 0 => (0, "Hm. It's... soup."),
                    _ => (-50, "Blech! What on earth is in this?"),
                };
                let names: Vec<String> = self.npcs.iter().map(|n| n.name.clone()).collect();
                for n in names {
                    let f = self.farmer.friendship(&n);
                    f.points = (f.points + points).clamp(0, 2500);
                }
                self.emit(WorldEvent::Message(format!("The guest of honor takes a long sip.\n\"{verdict}\"")));
            }
            Activity::Grange => {
                let items: Vec<Item> = self.activity.grange.take().map(|g| g.slots.into_iter().flatten().collect()).unwrap_or_default();
                let score = if items.is_empty() { 0 } else { grange_score(self, &items) };
                let (place, tokens) = match score {
                    s if s >= 90 => ("first", 1000),
                    s if s >= 75 => ("second", 500),
                    s if s >= 60 => ("third", 200),
                    _ => ("fourth", 50),
                };
                self.farmer.star_tokens += tokens;
                for it in items {
                    self.give_item(it);
                }
                self.emit(WorldEvent::Message(format!("The judges scored your display {score} points: {place} place! You win {tokens} star tokens.")));
                if place == "first" {
                    if let Some(p) = prize {
                        self.give_item(p);
                    }
                }
            }
            Activity::IceFishing => {
                let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.year as i64, 197]));
                let level = self.farmer.skills.level(crate::farmer::Skill::Fishing);
                let caught = 2 + level / 2 + rng.range(0, 4);
                let best_rival = 4 + rng.range(0, 4);
                let won = caught > best_rival;
                self.emit(WorldEvent::Message(format!("Time's up! You caught {caught} fish; the best of the rest caught {best_rival}.{}", if won { " You win the contest!" } else { "" })));
                if won {
                    if let Some(p) = prize {
                        self.give_item(p);
                    } else {
                        self.farmer.money += 2000;
                    }
                }
            }
            Activity::Gifts => {
                let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.year as i64, 199]));
                let giver = self.npcs.get(rng.range(0, self.npcs.len().max(1) as i32) as usize).map(|n| n.name.clone());
                let pool: Vec<String> = self.items.objects.iter().filter(|(_, o)| o.data.price > 0 && o.data.category != -4 && o.data.kind != "Litter").map(|(k, _)| k.clone()).collect();
                if let (Some(giver), Some(id)) = (giver, rng.pick(&pool).cloned()) {
                    let item = Item::object(&id, 1);
                    let name = self.items.display_name(&item);
                    self.give_item(item);
                    let display = self.npc_display(&giver);
                    self.emit(WorldEvent::Message(format!("{display} was your secret friend! They give you {name}.")));
                }
            }
            Activity::Dance => {
                if let Some(p) = self.activity.partner.clone() {
                    let f = self.farmer.friendship(&p);
                    f.points = (f.points + 100).clamp(0, 2500);
                    let display = self.npc_display(&p);
                    self.emit(WorldEvent::Message(format!("You and {display} dance under the blossoms.")));
                }
            }
        }
        self.events[before..].iter().any(|e| matches!(e, WorldEvent::Message(_)))
    }

    fn start_egg_hunt(&mut self) {
        let egg = ["176", "Dev_Egg"].iter().find(|id| self.items.objects.contains_key(**id)).map(|s| s.to_string()).unwrap_or_else(|| "176".into());
        let Some(e) = self.event.as_mut() else { return };
        let loc_name = e.location.clone();
        let Some(loc) = self.locations.get(&loc_name) else { return };
        let taken: Vec<TilePos> = e.actors.iter().map(|a| TilePos::from_world(Farmer::bbox_at(a.pos).center())).collect();
        let spots: Vec<TilePos> = (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).filter(|p| crate::collision::tile_passable(loc, *p) && !taken.contains(p)).collect();
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.year as i64, 211]));
        for _ in 0..EGGS.min(spots.len()) {
            let p = spots[rng.range(0, spots.len() as i32) as usize];
            if !e.props.iter().any(|x| matches!(x, EventProp::Object { tile, .. } if *tile == p)) {
                e.props.push(EventProp::Object { tile: p, id: egg.clone(), big: false });
            }
        }
        if let Some(f) = e.festival.as_mut() {
            f.roaming = true;
        }
        self.activity.hunting = true;
        self.activity.hunt_ms = EGG_HUNT_MS;
        self.activity.eggs_found = 0;
        self.emit(WorldEvent::Hud("Find as many eggs as you can!".into()));
    }

    pub(crate) fn grange(&self) -> Option<&Inventory> {
        self.activity.grange.as_ref()
    }

    /// While roaming during an egg hunt: pick up eggs, run the clock.
    pub(crate) fn update_egg_hunt(&mut self, dt: u32) {
        if !self.activity.hunting {
            return;
        }
        let Some(e) = self.event.as_mut() else { return };
        let Some(me) = e.actors.iter().find(|a| a.name == "farmer").map(|a| Farmer::bbox_at(a.pos)) else { return };
        let before = e.props.len();
        e.props.retain(|p| match p {
            EventProp::Object { tile, .. } => !me.intersects(&engine::IRect::new(tile.x * crate::TILE + 16, tile.y * crate::TILE + 16, 32, 32)),
            _ => true,
        });
        let found = (before - e.props.len()) as i32;
        if found > 0 {
            self.activity.eggs_found += found;
            self.sound("coin");
        }
        self.activity.hunt_ms = self.activity.hunt_ms.saturating_sub(dt);
        if self.activity.hunt_ms == 0 {
            self.activity.hunting = false;
            let won = self.activity.eggs_found >= EGGS_TO_WIN;
            let found = self.activity.eggs_found;
            if let Some(e) = self.event.as_mut() {
                e.props.retain(|p| !matches!(p, EventProp::Object { .. }));
                if let Some(f) = e.festival.as_mut() {
                    f.roaming = false;
                }
            }
            self.emit(WorldEvent::Message(format!("Time's up! You found {found} eggs.{}", if won { " You win!" } else { "" })));
            if won {
                if let Some(p) = self.festival_prize() {
                    self.give_item(p);
                } else {
                    self.farmer.money += 1000;
                }
            }
            if let Some(e) = self.event.as_mut() {
                e.waiting_ui = true;
            }
        }
    }

    /// Status line for the HUD during a contest.
    pub fn festival_status(&self) -> Option<String> {
        if self.activity.hunting {
            return Some(format!("Eggs: {}   Time: {}", self.activity.eggs_found, (self.activity.hunt_ms + 999) / 1000));
        }
        None
    }

    /// Winter 18: name a secret friend for the gift exchange (by letter).
    pub(crate) fn secret_friend_morning(&mut self) {
        if self.clock.season != crate::time::Season::Winter || self.clock.day != 18 || !self.festivals.contains_key("winter25") {
            return;
        }
        let mut rng = Rng::new(Rng::seed_from(&[self.seed as i64, self.clock.year as i64, 223]));
        let pool: Vec<String> = self.npcs.iter().filter(|n| self.winter_star_participant(n)).map(|n| n.name.clone()).collect();
        let Some(name) = rng.pick(&pool).cloned() else { return };
        self.farmer.secret_friend = Some(name.clone());
        let key = format!("Port_SecretFriend_{}", self.clock.year);
        let display = self.npc_display(&name);
        self.mail_data.insert(key.clone(), format!("Dear @,^This year your secret friend for the winter feast is {display}. Bring them a nice gift!^- The festival committee[#]Secret Friend"));
        self.farmer.mailbox.push(key);
    }
}
