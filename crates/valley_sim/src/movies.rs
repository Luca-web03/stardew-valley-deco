//! Movie theater (`Data/Movies`, `Data/MoviesReactions`; public format
//! subsets). Tickets come from a shop; giving a ticket to a villager
//! invites them for today. Watching plays the movie's scene texts; an
//! invited villager reacts by the movie's tags and friendship changes.
//! With a guest, a snack from `Data/Concessions` can be bought first; the
//! guest's `Data/ConcessionTastes` (their own entry, then `*`) decide how
//! much they enjoy it. Reaction values are approximations.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::dialogue::{DialoguePage, Emotion};
use crate::world::{World, WorldEvent};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MovieScene {
    pub id: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MovieData {
    pub id: String,
    pub seasons: Vec<String>,
    pub year_modulus: Option<i32>,
    pub year_remainder: Option<i32>,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub scenes: Vec<MovieScene>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MovieReaction {
    pub tag: String,
    pub response: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct NpcMovieReactions {
    #[serde(rename = "NPCName")]
    pub npc_name: String,
    pub reactions: Vec<MovieReaction>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ConcessionData {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub price: i32,
    pub texture: Option<String>,
    pub sprite_index: i32,
    pub item_tags: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ConcessionTaste {
    pub name: String,
    pub loved_tags: Vec<String>,
    pub liked_tags: Vec<String>,
    pub disliked_tags: Vec<String>,
}

/// Friendship change by snack taste (approx.).
pub const CONCESSION_POINTS: &[(&str, i32)] = &[("love", 50), ("like", 25), ("dislike", 0)];

/// Friendship change by reaction (approx.).
pub const REACTION_POINTS: &[(&str, i32)] = &[("love", 200), ("like", 100), ("dislike", 0)];
pub const NEUTRAL_POINTS: i32 = 50;

/// Ticket object ids (well-known, then placeholder).
const TICKETS: &[&str] = &["809", "Dev_Ticket"];

impl World {
    pub fn ticket_item(&self) -> Option<&'static str> {
        TICKETS.iter().copied().find(|id| self.items.objects.contains_key(*id))
    }

    /// The movie showing today (by season and year rotation).
    pub fn movie_today(&self) -> Option<&MovieData> {
        let season = self.clock.season.key();
        self.movies.iter().find(|m| {
            let season_ok = m.seasons.is_empty() || m.seasons.iter().any(|s| s.eq_ignore_ascii_case(season));
            let year_ok = match (m.year_modulus, m.year_remainder) {
                (Some(md), r) if md > 0 => self.clock.year % md == r.unwrap_or(0),
                _ => true,
            };
            season_ok && year_ok
        })
    }

    /// How `npc` feels about a movie: "love", "like", "dislike" or "neutral".
    pub fn movie_reaction(&self, npc: &str, movie: &MovieData) -> String {
        let Some(r) = self.movie_reactions.iter().find(|r| r.npc_name == npc) else { return "neutral".into() };
        r.reactions
            .iter()
            .find(|x| x.tag == "*" || x.tag == movie.id || movie.tags.iter().any(|t| *t == x.tag))
            .map(|x| x.response.to_ascii_lowercase())
            .unwrap_or_else(|| "neutral".into())
    }

    /// Giving a ticket to a villager invites them to today's movie.
    pub(crate) fn try_invite_to_movie(&mut self, npc: &str) -> bool {
        let Some(ticket) = self.ticket_item() else { return false };
        let slot = self.farmer.current_slot;
        if self.farmer.inventory.get(slot).and_then(|i| i.object_id()) != Some(ticket) {
            return false;
        }
        if self.movie_today().is_none() {
            self.emit(WorldEvent::Message("Nothing is showing right now.".into()));
            return true;
        }
        if self.movie_invite.is_some() {
            self.emit(WorldEvent::Message("You've already invited someone today.".into()));
            return true;
        }
        self.farmer.inventory.reduce(slot, 1);
        self.movie_invite = Some(npc.to_string());
        let display = self.npcs.iter().find(|n| n.name == npc).map(|n| n.display_name.clone()).unwrap_or(npc.to_string());
        self.emit(WorldEvent::Dialogue {
            npc: Some(npc.to_string()),
            pages: vec![DialoguePage { text: "A movie? I'd love to! See you at the theater.".into(), emotion: Emotion::Happy }],
        });
        self.hud(format!("{display} will meet you at the theater today."));
        true
    }

    /// How `npc` likes a snack: "love", "like", "dislike" or "neutral".
    pub fn concession_taste(&self, npc: &str, c: &ConcessionData) -> &'static str {
        let has = |tags: &[String]| tags.iter().any(|t| c.item_tags.iter().any(|x| x.eq_ignore_ascii_case(t)) || t.eq_ignore_ascii_case(&c.name));
        for who in [npc, "*"] {
            if let Some(t) = self.concession_tastes.iter().find(|t| t.name == who) {
                if has(&t.loved_tags) {
                    return "love";
                }
                if has(&t.disliked_tags) {
                    return "dislike";
                }
                if has(&t.liked_tags) {
                    return "like";
                }
            }
        }
        "neutral"
    }

    /// Theater door: watch today's movie (with a ticket or an invitation).
    /// With a guest, offer a snack first.
    pub(crate) fn watch_movie(&mut self) -> bool {
        if self.movie_today().is_none() {
            self.emit(WorldEvent::Message("The theater is between showings.".into()));
            return true;
        }
        if let Some(guest) = self.movie_invite.clone() {
            if !self.concessions.is_empty() {
                let display = self.npcs.iter().find(|n| n.name == guest).map(|n| n.display_name.clone()).unwrap_or(guest.clone());
                let mut options: Vec<(String, String)> = self
                    .concessions
                    .iter()
                    .map(|c| {
                        let name = self.items.expand_text(if c.display_name.is_empty() { &c.name } else { &c.display_name });
                        (format!("concession:{}", c.id), format!("{name} - {}g", c.price))
                    })
                    .collect();
                options.push(("concession:".into(), "No snack".into()));
                self.emit(WorldEvent::Choice { text: format!("Get a snack for {display}?"), options });
                return true;
            }
        }
        self.play_movie(None)
    }

    /// The snack choice: buy it (if affordable) and start the movie.
    pub(crate) fn choose_concession(&mut self, id: &str) {
        let snack = self.concessions.iter().find(|c| c.id == id).cloned();
        if let Some(c) = &snack {
            if self.farmer.money < c.price {
                self.emit(WorldEvent::Message("You can't afford that.".into()));
                return;
            }
            self.farmer.money -= c.price;
            self.sound("purchase");
        }
        self.play_movie(snack);
    }

    fn play_movie(&mut self, snack: Option<ConcessionData>) -> bool {
        let Some(movie) = self.movie_today().cloned() else { return false };
        let guest = self.movie_invite.take();
        if guest.is_none() {
            let Some(ticket) = self.ticket_item() else { return false };
            if self.farmer.inventory.count(ticket) == 0 {
                self.emit(WorldEvent::Message("You need a ticket. The box office sells them.".into()));
                return true;
            }
            self.farmer.inventory.remove_object(ticket, 1);
        }
        let title = self.items.expand_text(&movie.title);
        let mut pages = vec![DialoguePage { text: format!("Now showing: {title}"), emotion: Emotion::Neutral }];
        pages.extend(movie.scenes.iter().map(|s| DialoguePage { text: self.items.expand_text(&s.text), emotion: Emotion::Neutral }));
        if let Some(npc) = guest {
            let reaction = self.movie_reaction(&npc, &movie);
            let points = REACTION_POINTS.iter().find(|(r, _)| *r == reaction).map(|(_, p)| *p).unwrap_or(NEUTRAL_POINTS);
            let romanceable = self.npcs.iter().find(|n| n.name == npc).is_some_and(|n| n.can_romance);
            let f = self.farmer.friendship(&npc);
            f.points = (f.points + points).clamp(0, f.max_points(romanceable));
            let display = self.npcs.iter().find(|n| n.name == npc).map(|n| n.display_name.clone()).unwrap_or(npc.clone());
            let line = match reaction.as_str() {
                "love" => format!("{display}: \"That was wonderful! Thank you for bringing me.\""),
                "like" => format!("{display}: \"I enjoyed that. Thanks for inviting me.\""),
                "dislike" => format!("{display}: \"Hmm... not really my kind of movie.\""),
                _ => format!("{display}: \"That was nice. Thanks!\""),
            };
            pages.push(DialoguePage { text: line, emotion: if reaction == "dislike" { Emotion::Sad } else { Emotion::Happy } });
            if let Some(c) = &snack {
                let taste = self.concession_taste(&npc, c);
                let points = CONCESSION_POINTS.iter().find(|(t, _)| *t == taste).map(|(_, p)| *p).unwrap_or(0);
                let f = self.farmer.friendship(&npc);
                f.points = (f.points + points).clamp(0, f.max_points(romanceable));
                let snack_name = self.items.expand_text(if c.display_name.is_empty() { &c.name } else { &c.display_name });
                let line = match taste {
                    "love" => format!("{display}: \"And {snack_name}! You know me so well.\""),
                    "like" => format!("{display}: \"Thanks for the {snack_name}.\""),
                    "dislike" => format!("{display}: \"I'm not really a fan of {snack_name}...\""),
                    _ => format!("{display}: \"Oh, {snack_name}. Thanks.\""),
                };
                pages.push(DialoguePage { text: line, emotion: if taste == "dislike" { Emotion::Sad } else { Emotion::Happy } });
            }
            self.farmer.movies_seen_with.push(npc);
        }
        self.farmer.movies_seen.push(movie.id.clone());
        self.emit(WorldEvent::Dialogue { npc: None, pages });
        true
    }
}

/// Loaders.
pub fn load(cm: &mut valley_content::ContentManager) -> (Vec<MovieData>, Vec<NpcMovieReactions>) {
    let movies: Vec<MovieData> = cm.load_json("Data/Movies").unwrap_or_default();
    let reactions: Vec<NpcMovieReactions> = cm.load_json("Data/MoviesReactions").unwrap_or_default();
    (movies, reactions)
}

pub fn load_concessions(cm: &mut valley_content::ContentManager) -> (Vec<ConcessionData>, Vec<ConcessionTaste>) {
    let list: Vec<ConcessionData> = cm.load_json("Data/Concessions").unwrap_or_default();
    let tastes: Vec<ConcessionTaste> = cm.load_json("Data/ConcessionTastes").unwrap_or_default();
    (list, tastes)
}

/// For callers that want reactions by NPC.
pub fn by_npc(r: &[NpcMovieReactions]) -> BTreeMap<String, NpcMovieReactions> {
    r.iter().map(|x| (x.npc_name.clone(), x.clone())).collect()
}
