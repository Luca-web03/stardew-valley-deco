//! Social settings from `Data/Characters` (public format): `CanSocialize`
//! and `CanReceiveGifts`, `SocialTab` and `Calendar` visibility,
//! `DumpsterDiveFriendshipEffect`/`DumpsterDiveEmote` (villagers within 7
//! tiles who see you rummage through a trash can; default -25 and an angry
//! emote, children don't mind), `ItemDeliveryQuests`, `PerfectionScore`,
//! `SpouseGiftJealousy` (+ `SpouseGiftJealousyFriendshipChange`, default
//! -30), `SpouseWantsChildren`, `FlowerDanceCanDance` (default: romanceable)
//! and `WinterStarParticipant`. Conditions are game state queries.

use crate::npc::Npc;
use crate::world::World;

/// How far villagers notice trash can diving (tiles).
pub const DUMPSTER_SIGHT: i32 = 7;
/// Emote ids (rows of `TileSheets/emotes`).
pub const EMOTE_ANGRY: i32 = 12;
pub const EMOTE_HAPPY: i32 = 32;

impl World {
    fn npc_query(&self, cond: Option<&str>, salt: i64) -> bool {
        let mut rng = self.day_rng(salt);
        self.check_query(cond, None, None, &mut rng)
    }

    pub fn can_socialize(&self, n: &Npc) -> bool {
        self.npc_query(n.data.can_socialize.as_deref(), 801)
    }

    pub fn can_receive_gifts(&self, n: &Npc) -> bool {
        n.data.can_receive_gifts && self.can_socialize(n)
    }

    /// The social tab line: `None` hides the villager, else the name shown
    /// ("???" until met for the default `UnknownUntilMet`).
    pub fn social_tab_name(&self, n: &Npc) -> Option<String> {
        let met = self.farmer.friendships.get(&n.name).is_some_and(|f| f.met);
        if !self.can_socialize(n) {
            return None;
        }
        match n.data.social_tab.as_deref().unwrap_or("UnknownUntilMet") {
            "HiddenAlways" => None,
            "HiddenUntilMet" if !met => None,
            "UnknownUntilMet" if !met => Some("???".into()),
            _ => Some(n.display_name.clone()),
        }
    }

    /// Whether the calendar shows this villager's birthday.
    pub fn calendar_shows(&self, n: &Npc) -> bool {
        let met = self.farmer.friendships.get(&n.name).is_some_and(|f| f.met);
        match n.data.calendar.as_deref().unwrap_or("AlwaysShown") {
            "HiddenAlways" => false,
            "HiddenUntilMet" => met,
            _ => true,
        }
    }

    pub fn counts_for_perfection(&self, n: &Npc) -> bool {
        n.data.perfection_score.unwrap_or(true)
    }

    pub fn takes_item_delivery_quests(&self, n: &Npc) -> bool {
        self.can_socialize(n) && self.npc_query(n.data.item_delivery_quests.as_deref(), 802)
    }

    pub fn can_flower_dance(&self, n: &Npc) -> bool {
        n.data.flower_dance_can_dance.unwrap_or(n.can_romance)
    }

    pub fn winter_star_participant(&self, n: &Npc) -> bool {
        self.can_socialize(n) && self.npc_query(n.data.winter_star_participant.as_deref(), 803)
    }

    pub fn spouse_wants_children(&self, spouse: &str) -> bool {
        self.npcs.iter().find(|n| n.name == spouse).map_or(true, |n| self.npc_query(n.data.spouse_wants_children.as_deref(), 804))
    }

    /// (gets jealous, friendship change) for the spouse.
    pub fn spouse_jealousy(&self, spouse: &str) -> (bool, i32) {
        match self.npcs.iter().find(|n| n.name == spouse) {
            Some(n) => (self.npc_query(n.data.spouse_gift_jealousy.as_deref(), 805), n.data.spouse_gift_jealousy_friendship_change.unwrap_or(-30)),
            None => (true, -30),
        }
    }

    /// Villagers nearby react to the farmer digging through a trash can.
    pub(crate) fn dumpster_witnesses(&mut self) {
        let me = self.farmer.tile();
        let here = self.current.clone();
        let witnesses: Vec<usize> = (0..self.npcs.len())
            .filter(|&i| {
                let n = &self.npcs[i];
                let t = n.tile();
                n.location == here && (t.x - me.x).pow(2) + (t.y - me.y).pow(2) <= DUMPSTER_SIGHT * DUMPSTER_SIGHT && self.can_socialize(n)
            })
            .collect();
        for i in witnesses {
            let n = &self.npcs[i];
            let child = n.data.age.eq_ignore_ascii_case("Child");
            let effect = n.data.dumpster_dive_friendship_effect.unwrap_or(if child { 0 } else { -25 });
            let emote = n.data.dumpster_dive_emote.unwrap_or(if effect >= 0 { EMOTE_HAPPY } else { EMOTE_ANGRY });
            let name = n.name.clone();
            let romanceable = n.can_romance;
            if effect != 0 {
                let f = self.farmer.friendship(&name);
                f.points = (f.points + effect).clamp(0, f.max_points(romanceable));
            }
            let text = if effect < 0 { "Ugh... really?" } else { "Find anything good in there?" };
            self.npcs[i].bubble = Some((text.to_string(), 2500));
            self.npcs[i].emote = Some((emote, 0));
        }
    }
}
