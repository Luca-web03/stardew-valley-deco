//! Grandpa's evaluation and the Joja route.
//!
//! Evaluation: on the first morning of year 3 the farm is scored (the
//! publicly documented 21-point table: earnings, skills, collections,
//! family, friends, pet, community, keys) and up to four candles are lit on
//! the shrine (`Port.Shrine` tile action). Placing a diamond on the shrine
//! asks for a new evaluation the next morning; four candles earn the
//! statue of perfection (big craftable `160`).
//!
//! Joja route: `Port.JojaMembership` sells a membership (5,000g) that
//! closes the community bundles; `Port.JojaForm` sells one community
//! project per bundle area. Projects come from `Port/JojaProjects` (list of
//! `Id`, `Area`, `DisplayName`, `Price`, `Flag`) or the original areas.
//! Finishing an area either way sets `ccDone_<area>` and the area's
//! well-known flag (`ccPantry`, `ccVault`, ...); all areas set
//! `ccIsComplete`.

use serde::{Deserialize, Serialize};

use crate::farmer::Skill;
use crate::world::{World, WorldEvent};

pub const EVALUATED: &str = "Port_GrandpaEvaluated";
pub const REEVALUATE: &str = "Port_GrandpaReevaluate";
pub const STATUE_GIVEN: &str = "Port_GrandpaStatue";
pub const JOJA_MEMBER: &str = "JojaMember";
pub const JOJA_MEMBERSHIP_PRICE: i32 = 5_000;
const DIAMOND: &str = "72";
const STATUE_OF_PERFECTION: &str = "160";

/// Well-known flags for the original bundle areas.
pub fn area_flag(area: &str) -> Option<&'static str> {
    Some(match area {
        "Pantry" => "ccPantry",
        "Crafts Room" | "CraftsRoom" => "ccCraftsRoom",
        "Fish Tank" | "FishTank" => "ccFishTank",
        "Boiler Room" | "BoilerRoom" => "ccBoilerRoom",
        "Vault" => "ccVault",
        "Bulletin Board" | "BulletinBoard" => "ccBulletin",
        _ => return None,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct JojaProject {
    pub id: String,
    pub area: String,
    pub display_name: String,
    pub description: String,
    pub price: i32,
    pub flag: String,
}

impl Default for JojaProject {
    fn default() -> Self {
        Self { id: String::new(), area: String::new(), display_name: String::new(), description: String::new(), price: 0, flag: String::new() }
    }
}

/// The original community development projects (public prices).
pub fn default_joja_projects() -> Vec<JojaProject> {
    let p = |id: &str, area: &str, name: &str, desc: &str, price: i32, flag: &str| JojaProject {
        id: id.into(),
        area: area.into(),
        display_name: name.into(),
        description: desc.into(),
        price,
        flag: flag.into(),
    };
    vec![
        p("Bus", "Vault", "Bus Repair", "Get the bus to the desert running again.", 40_000, "jojaVault"),
        p("Minecarts", "Boiler Room", "Minecarts", "Restore the minecart network.", 15_000, "jojaBoilerRoom"),
        p("Bridge", "Crafts Room", "Bridge Repair", "Fix the old bridge on the beach.", 25_000, "jojaCraftsRoom"),
        p("Greenhouse", "Pantry", "Greenhouse", "Rebuild the farm's greenhouse.", 35_000, "jojaPantry"),
        p("Panning", "Fish Tank", "Panning", "Clear the river for panning.", 20_000, "jojaFishTank"),
    ]
}

pub fn load_joja_projects(cm: &mut valley_content::ContentManager) -> Vec<JojaProject> {
    match cm.load_json::<Vec<JojaProject>>("Port/JojaProjects") {
        Some(v) if !v.is_empty() => v,
        _ => default_joja_projects(),
    }
}

/// Candles lit for an evaluation score (0..=21).
pub fn candles_for(score: i32) -> i32 {
    match score {
        i32::MIN..=3 => 1,
        4..=7 => 2,
        8..=11 => 3,
        _ => 4,
    }
}

impl World {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.farmer.mail_received.iter().any(|m| m == flag)
    }

    fn set_flag(&mut self, flag: &str) {
        if !self.has_flag(flag) {
            self.farmer.mail_received.push(flag.to_string());
        }
    }

    /// The evaluation score with each line (label, points).
    pub fn grandpa_score_lines(&self) -> Vec<(&'static str, i32)> {
        let f = &self.farmer;
        let earned = f.total_earned;
        let mut lines = Vec::new();
        let money: i32 = [50_000i64, 100_000, 200_000, 300_000, 500_000].iter().filter(|&&m| earned >= m).count() as i32 + if earned >= 1_000_000 { 2 } else { 0 };
        lines.push(("Earnings", money));
        let levels: i32 = [Skill::Farming, Skill::Mining, Skill::Foraging, Skill::Fishing, Skill::Combat].iter().map(|s| f.skills.level(*s)).sum();
        lines.push(("Skills", (levels >= 30) as i32 + (levels >= 50) as i32));
        let parts = self.perfection_parts();
        let done = |name: &str| parts.iter().any(|(n, _, d)| *n == name && *d >= 1.0);
        lines.push(("Museum collection", done("Museum") as i32));
        lines.push(("Master angler", done("Fish caught") as i32));
        lines.push(("Full shipment", done("Items shipped") as i32));
        lines.push(("Family", (f.spouse.is_some() && f.house_level >= 2) as i32));
        let close = f.friendships.values().filter(|x| x.hearts() >= 8).count();
        lines.push(("Friends", (close >= 5) as i32 + (close >= 10) as i32));
        lines.push(("Pet", self.pet.as_ref().is_some_and(|p| p.friendship >= 1000) as i32));
        let cc = self.has_flag("ccIsComplete");
        let joja = self.has_flag(JOJA_MEMBER);
        lines.push(("Community", if cc && !joja { 3 } else { cc as i32 }));
        lines.push(("Skull key", (f.deepest_mine_level >= 120) as i32));
        lines.push(("Rusty key", (f.bundles.museum.len() >= 60) as i32));
        lines
    }

    pub fn grandpa_score(&self) -> i32 {
        self.grandpa_score_lines().iter().map(|l| l.1).sum()
    }

    /// Morning: the first evaluation (year 3) or a requested re-evaluation.
    pub(crate) fn grandpa_morning(&mut self) {
        let first = self.clock.year >= 3 && !self.has_flag(EVALUATED);
        let again = self.has_flag(REEVALUATE);
        if !first && !again {
            return;
        }
        self.farmer.mail_received.retain(|m| m != REEVALUATE);
        self.set_flag(EVALUATED);
        let score = self.grandpa_score();
        let candles = candles_for(score).max(self.farmer.grandpa_candles);
        self.farmer.grandpa_candles = candles;
        let verdict = match candles {
            1 => "You've barely begun. Don't give up on the land, my child.",
            2 => "You've worked hard, but there's more the farm can give.",
            3 => "I'm proud of you. The farm has come a long way.",
            _ => "This is everything I hoped for. You've made this place shine.",
        };
        self.emit(WorldEvent::Message(format!(
            "A soft light fills the room. Grandpa's voice drifts in from somewhere far away...\n\"{verdict}\"\n({candles} of 4 candles lit on the shrine.)"
        )));
    }

    /// `Port.Shrine`.
    pub(crate) fn shrine_action(&mut self) -> bool {
        if !self.has_flag(EVALUATED) {
            self.emit(WorldEvent::Message("A small shrine with four empty candle holders. Grandpa asked you to wait here... someday.".into()));
            return true;
        }
        let candles = self.farmer.grandpa_candles;
        if candles >= 4 {
            if !self.has_flag(STATUE_GIVEN) {
                let statue = self.items.create(&format!("(BC){STATUE_OF_PERFECTION}"), 1);
                match statue {
                    Some(item) if self.farmer.inventory.can_fit(&item) => {
                        self.give_item(item);
                        self.set_flag(STATUE_GIVEN);
                        self.sound("newArtifact");
                        self.emit(WorldEvent::Message("All four candles burn bright. Something has been left on the shrine for you.".into()));
                    }
                    Some(_) => self.emit(WorldEvent::Message("Make room in your bag first.".into())),
                    None => {
                        self.set_flag(STATUE_GIVEN);
                        self.emit(WorldEvent::Message("All four candles burn bright.".into()));
                    }
                }
            } else {
                self.emit(WorldEvent::Message("All four candles burn bright.".into()));
            }
            return true;
        }
        if self.has_flag(REEVALUATE) {
            self.emit(WorldEvent::Message("The diamond glitters on the shrine. Grandpa will come by tonight.".into()));
            return true;
        }
        if self.farmer.inventory.count(DIAMOND) == 0 {
            self.emit(WorldEvent::Message(format!("{candles} of 4 candles are lit. A diamond on the shrine would call Grandpa back for another look.")));
            return true;
        }
        self.emit(WorldEvent::Choice {
            text: format!("{candles} of 4 candles are lit. Place a diamond on the shrine to be evaluated again?"),
            options: vec![("shrine_diamond".into(), "Place a diamond".into())],
        });
        true
    }

    /// The answer to a [`WorldEvent::Choice`].
    pub fn choose(&mut self, id: &str) {
        if id == "shrine_diamond" {
            if let Some(slot) = (0..self.farmer.inventory.capacity()).find(|&s| self.farmer.inventory.get(s).and_then(|i| i.object_id()) == Some(DIAMOND)) {
                self.farmer.inventory.reduce(slot, 1);
                self.set_flag(REEVALUATE);
                self.sound("newArtifact");
            }
        } else if id == "joja_member" {
            if let Err(e) = self.buy_joja_membership() {
                self.emit(WorldEvent::Message(e));
            }
        } else if let Some(p) = id.strip_prefix("joja_project:") {
            if let Err(e) = self.buy_joja_project(p) {
                self.emit(WorldEvent::Message(e));
            }
        } else if let Some(c) = id.strip_prefix("concession:") {
            self.choose_concession(c);
        } else if let Some(c) = id.strip_prefix("cave:") {
            self.choose_cave(c);
        } else if let Some(f) = id.strip_prefix("festival:") {
            self.choose_festival(f);
        }
    }

    pub fn joja_member(&self) -> bool {
        self.has_flag(JOJA_MEMBER)
    }

    pub fn buy_joja_membership(&mut self) -> Result<(), String> {
        if self.joja_member() {
            return Err("You're already a member.".into());
        }
        if self.farmer.money < JOJA_MEMBERSHIP_PRICE {
            return Err(format!("Membership costs {JOJA_MEMBERSHIP_PRICE}g."));
        }
        self.farmer.money -= JOJA_MEMBERSHIP_PRICE;
        self.set_flag(JOJA_MEMBER);
        self.sound("purchase");
        self.emit(WorldEvent::Hud("Welcome to Joja! The community center has closed its doors.".into()));
        Ok(())
    }

    /// `Port.JojaMembership`.
    pub(crate) fn joja_membership_action(&mut self) -> bool {
        if self.joja_member() {
            self.emit(WorldEvent::Message("Thank you for being a valued member!".into()));
        } else {
            self.emit(WorldEvent::Choice {
                text: format!("Become a member for {JOJA_MEMBERSHIP_PRICE}g? Members fund town projects here instead of filling bundles at the community center."),
                options: vec![("joja_member".into(), "Sign up".into())],
            });
        }
        true
    }

    pub fn joja_project_done(&self, p: &JojaProject) -> bool {
        self.has_flag(&p.flag) || self.farmer.bundles.areas_completed.contains(&p.area)
    }

    /// `Port.JojaForm`: list the projects still open.
    pub(crate) fn joja_form_action(&mut self) -> bool {
        if !self.joja_member() {
            self.emit(WorldEvent::Message("Community development projects are for members only.".into()));
            return true;
        }
        let open: Vec<(String, String)> = self.joja_projects.iter().filter(|p| !self.joja_project_done(p)).map(|p| (format!("joja_project:{}", p.id), format!("{} - {}g", p.display_name, p.price))).collect();
        if open.is_empty() {
            self.emit(WorldEvent::Message("Every project is complete. Thank you for choosing Joja!".into()));
        } else {
            self.emit(WorldEvent::Choice { text: "Community development. Choose a project to fund:".into(), options: open });
        }
        true
    }

    pub fn buy_joja_project(&mut self, id: &str) -> Result<(), String> {
        let p = self.joja_projects.iter().find(|p| p.id == id).cloned().ok_or("No such project.")?;
        if !self.joja_member() {
            return Err("Members only.".into());
        }
        if self.joja_project_done(&p) {
            return Err("That project is already done.".into());
        }
        if self.farmer.money < p.price {
            return Err(format!("{} costs {}g.", p.display_name, p.price));
        }
        self.farmer.money -= p.price;
        self.set_flag(&p.flag);
        self.sound("purchase");
        self.complete_area(&p.area);
        self.emit(WorldEvent::Hud(format!("{} funded! Work starts tonight.", p.display_name)));
        Ok(())
    }

    /// An area is restored (bundles or Joja): flags and the full set.
    pub(crate) fn complete_area(&mut self, area: &str) {
        if !self.farmer.bundles.areas_completed.iter().any(|a| a == area) {
            self.farmer.bundles.areas_completed.push(area.to_string());
        }
        self.set_flag(&format!("ccDone_{area}"));
        if let Some(f) = area_flag(area) {
            self.set_flag(f);
        }
        let mut areas: Vec<String> = self.bundle_areas();
        for p in &self.joja_projects {
            if self.joja_member() && !areas.contains(&p.area) {
                areas.push(p.area.clone());
            }
        }
        let all = !areas.is_empty() && areas.iter().all(|a| self.farmer.bundles.areas_completed.contains(a));
        if all {
            self.set_flag("ccIsComplete");
        }
        self.apply_cleared_tiles();
    }

    /// Start an arcade game.
    pub(crate) fn open_arcade(&mut self, game: &str) -> bool {
        self.sound("coin");
        self.emit(WorldEvent::OpenArcade(game.to_string()));
        true
    }

    /// An arcade game ended: keep the best score and count wins.
    pub fn arcade_finished(&mut self, game: &str, score: i32, won: bool) {
        let best = self.farmer.counters.entry(format!("Arcade.{game}.Best")).or_insert(0);
        let record = score > *best;
        *best = (*best).max(score);
        if won {
            *self.farmer.counters.entry(format!("Arcade.{game}.Wins")).or_insert(0) += 1;
            self.check_achievements();
        }
        if record && score > 0 {
            self.emit(WorldEvent::Hud(format!("New high score: {score}")));
        }
    }
}
