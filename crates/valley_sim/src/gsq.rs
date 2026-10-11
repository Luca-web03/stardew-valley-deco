//! Game state queries: the condition strings used throughout data assets
//! ("SEASON spring, !WEATHER Here Rain"), per the public query syntax:
//! comma-separated queries that must all pass, `!` to negate, quoted
//! arguments, `ANY "<query>" "<query>"...`.
//!
//! Date, weather, location, world, player, randomization and item queries
//! are implemented; player queries read the current player (any target
//! player argument). Unknown queries fail and are reported once.

use std::collections::HashSet;
use std::sync::Mutex;

use engine::Rng;

use crate::farmer::Skill;
use crate::item::Item;
use crate::time::{Clock, Season, Weather, WEEKDAYS};
use crate::world::World;

pub struct QueryCtx<'a> {
    pub clock: &'a Clock,
    pub weather: Weather,
    pub mail: &'a [String],
    pub events_seen: &'a [String],
    pub location: &'a str,
    pub rng: &'a mut Rng,
    /// The world, for world/player queries (None: those fail).
    pub world: Option<&'a World>,
}

/// Extra context for queries about the world, the player and items.
#[derive(Clone, Copy, Default)]
pub struct QueryWorld<'a> {
    pub world: Option<&'a World>,
    /// The `Target` item (e.g. an item being sold or a machine output).
    pub target: Option<&'a Item>,
    /// The `Input` item (e.g. what's placed in a machine).
    pub input: Option<&'a Item>,
}

static UNKNOWN: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn report_unknown(name: &str) {
    if let Ok(mut g) = UNKNOWN.lock() {
        let set = g.get_or_insert_with(HashSet::new);
        if set.insert(name.to_string()) {
            eprintln!("game state query not supported: {name}");
        }
    }
}

/// Split on commas that aren't inside quotes.
fn split_conditions(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c)
            }
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Whitespace tokens, keeping quoted arguments whole (quotes removed).
pub(crate) fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut had = false;
    for c in s.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                had = true;
            }
            c if c.is_whitespace() && !quoted => {
                if had || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                had = false;
            }
            c => cur.push(c),
        }
    }
    if had || !cur.is_empty() {
        out.push(cur);
    }
    out
}

pub fn check(cond: Option<&str>, ctx: &mut QueryCtx) -> bool {
    check_with(cond, ctx, QueryWorld::default())
}

/// Check with world/player/item context.
pub fn check_with(cond: Option<&str>, ctx: &mut QueryCtx, qw: QueryWorld) -> bool {
    let Some(cond) = cond else { return true };
    let cond = cond.trim();
    if cond.is_empty() || cond.eq_ignore_ascii_case("TRUE") {
        return true;
    }
    if cond.eq_ignore_ascii_case("FALSE") {
        return false;
    }
    split_conditions(cond).iter().all(|c| check_one(c, ctx, qw))
}

fn num(args: &[String], i: usize, d: i64) -> i64 {
    args.get(i).and_then(|v| v.parse::<f64>().ok()).map(|v| v as i64).unwrap_or(d)
}

fn in_range(v: i64, args: &[String], min_i: usize) -> bool {
    v >= num(args, min_i, i64::MIN) && v <= num(args, min_i + 1, i64::MAX)
}

fn hash_key(s: &str) -> i64 {
    s.bytes().fold(5381i64, |h, b| h.wrapping_mul(33) ^ b as i64)
}

fn skill_of(name: &str) -> Option<Skill> {
    Some(match name {
        "PLAYER_FARMING_LEVEL" => Skill::Farming,
        "PLAYER_FISHING_LEVEL" => Skill::Fishing,
        "PLAYER_FORAGING_LEVEL" => Skill::Foraging,
        "PLAYER_MINING_LEVEL" => Skill::Mining,
        "PLAYER_COMBAT_LEVEL" => Skill::Combat,
        _ => return None,
    })
}

fn item_for<'a>(qw: &QueryWorld<'a>, target: &str) -> Option<&'a Item> {
    if target.eq_ignore_ascii_case("Input") {
        qw.input
    } else {
        qw.target
    }
}

fn check_one(q: &str, ctx: &mut QueryCtx, qw: QueryWorld) -> bool {
    let (neg, q) = match q.strip_prefix('!') {
        Some(r) => (true, r.trim()),
        None => (false, q),
    };
    let toks = tokens(q);
    let Some(name) = toks.first().map(|n| n.to_ascii_uppercase()) else { return true };
    let args = &toks[1..];
    let w = qw.world.or(ctx.world);
    let f = w.map(|w| &w.farmer);
    let r = match name.as_str() {
        "TRUE" => true,
        "FALSE" => false,
        "ANY" => args.iter().any(|a| check_with(Some(a), ctx, qw)),

        // Date and time.
        "SEASON" => args.iter().any(|s| Season::parse(s) == Some(ctx.clock.season)),
        "LOCATION_SEASON" => args.iter().skip(1).any(|s| Season::parse(s) == Some(ctx.clock.season)),
        "DAY_OF_WEEK" => args.iter().any(|d| {
            let d = d.to_ascii_lowercase();
            let wd = ctx.clock.weekday().to_ascii_lowercase();
            d.starts_with(&wd[..3.min(wd.len())]) || d.parse::<i32>().ok() == Some((ctx.clock.weekday_index() + 1) % 7)
        }),
        "DAY_OF_MONTH" => args.iter().any(|d| match d.as_str() {
            "even" => ctx.clock.day % 2 == 0,
            "odd" => ctx.clock.day % 2 == 1,
            n => n.parse::<i32>().ok() == Some(ctx.clock.day),
        }),
        "SEASON_DAY" => args.chunks(2).any(|c| c.len() == 2 && Season::parse(&c[0]) == Some(ctx.clock.season) && c[1].parse::<i32>().ok() == Some(ctx.clock.day)),
        "YEAR" => in_range(ctx.clock.year as i64, args, 0),
        "TIME" => in_range(ctx.clock.time as i64, args, 0),
        "DAYS_PLAYED" => in_range(ctx.clock.days_played() as i64, args, 0),
        "IS_GREEN_RAIN_DAY" => ctx.weather == Weather::GreenRain,
        "IS_FESTIVAL_DAY" => {
            let offset = num(args, 1, 0) as i32;
            w.is_some_and(|w| w.festival_on(ctx.clock.days_played() + offset))
        }
        "IS_PASSIVE_FESTIVAL_TODAY" | "IS_PASSIVE_FESTIVAL_OPEN" => w.is_some_and(|w| args.first().is_some_and(|id| w.passive_festival_today(id))),

        // Weather and locations.
        "WEATHER" => args.iter().skip(1).any(|wn| {
            let wn = wn.to_ascii_lowercase();
            match ctx.weather {
                Weather::Sun => wn == "sun",
                Weather::Rain => wn == "rain",
                Weather::Storm => wn == "storm" || wn == "rain",
                Weather::Snow => wn == "snow",
                Weather::Wind => wn == "wind",
                Weather::GreenRain => wn == "greenrain" || wn == "rain",
            }
        }),
        "IS_RAINING" | "LOCATION_IS_RAINING" => ctx.weather.is_raining(),
        "LOCATION_NAME" | "LOCATION_UNIQUE_NAME" => args.iter().skip(1).any(|l| l.eq_ignore_ascii_case(ctx.location)),
        "LOCATION_IS_INDOORS" | "LOCATION_IS_OUTDOORS" => {
            let outdoors = w.and_then(|w| w.locations.get(ctx.location)).is_some_and(|l| l.outdoors);
            outdoors == (name == "LOCATION_IS_OUTDOORS")
        }
        "LOCATION_IS_MINES" => ctx.location == "Mine" || crate::mines::level_of(ctx.location).is_some_and(|l| l < 121),
        "LOCATION_IS_SKULL_CAVE" => ctx.location == "SkullCave" || crate::mines::level_of(ctx.location).is_some_and(|l| l > 120),
        "LOCATION_ACCESSIBLE" => w.is_some_and(|w| args.first().is_some_and(|l| w.locations.contains_key(l.as_str()))),
        "LOCATION_CONTEXT" => {
            let ctx_name = if ctx.location.contains("Island") || ctx.location.contains("Volcano") { "Island" } else if ctx.location.contains("Desert") || ctx.location.contains("SkullCave") { "Desert" } else { "Default" };
            args.iter().skip(1).any(|c| c.eq_ignore_ascii_case(ctx_name))
        }

        // World.
        "FARM_TYPE" => f.is_some_and(|f| args.iter().any(|t| t.eq_ignore_ascii_case(&f.farm_type) || crate::farm_types::farm_type_alias(t).is_some_and(|a| a.eq_ignore_ascii_case(&f.farm_type)))),
        "IS_CUSTOM_FARM_TYPE" => f.is_some_and(|f| crate::farm_types::FARM_TYPES.iter().all(|t| !t.0.eq_ignore_ascii_case(&f.farm_type))),
        "FARM_NAME" => f.is_some_and(|f| args.join(" ").eq_ignore_ascii_case(&f.farm_name)),
        "FARM_CAVE" => f.is_some_and(|f| args.iter().any(|c| f.cave_choice.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(c)) || (c.eq_ignore_ascii_case("None") && f.cave_choice.is_none()))),
        "IS_COMMUNITY_CENTER_COMPLETE" => f.is_some_and(|f| f.mail_received.iter().any(|m| m == "ccIsComplete")),
        "IS_JOJA_MART_COMPLETE" => f.is_some_and(|f| f.mail_received.iter().any(|m| m == "jojaComplete" || m == "JojaComplete")),
        "IS_HOST" => true,
        "IS_MULTIPLAYER" => w.is_some_and(|w| !w.farmhands.is_empty()),
        "CAN_BUILD_CABIN" | "CAN_BUILD_FOR_CABINS" => true,
        "BUILDINGS_CONSTRUCTED" => w.is_some_and(|w| {
            let kind = args.get(1).filter(|k| !k.eq_ignore_ascii_case("All"));
            let n = w.buildings.iter().filter(|b| b.days_left == 0 && kind.map_or(true, |k| b.kind.eq_ignore_ascii_case(k))).count() as i64;
            in_range(n, args, 2) && (args.len() > 2 || n > 0)
        }),
        "MUSEUM_DONATIONS" => f.is_some_and(|f| in_range(f.bundles.museum.len() as i64, args, 0)),
        "WORLD_STATE_ID" | "WORLD_STATE_FIELD" => f.is_some_and(|f| args.first().is_some_and(|id| f.mail_received.iter().any(|m| m.eq_ignore_ascii_case(id)))),
        "FOUND_ALL_LOST_BOOKS" => f.is_some_and(|f| f.counters.get("Port.LostBooksFound").copied().unwrap_or(0) >= 21),

        // The player.
        n if skill_of(n).is_some() => f.is_some_and(|f| in_range(f.skills.level(skill_of(n).unwrap()) as i64, args, 1)),
        "PLAYER_LUCK_LEVEL" => f.is_some_and(|_| in_range(0, args, 1)),
        "PLAYER_CURRENT_MONEY" => f.is_some_and(|f| in_range(f.money as i64, args, 1)),
        "PLAYER_MONEY_EARNED" => f.is_some_and(|f| in_range(f.total_earned, args, 1)),
        "PLAYER_FARMHOUSE_UPGRADE" => f.is_some_and(|f| in_range(f.house_level as i64, args, 1)),
        "PLAYER_GENDER" => f.is_some_and(|f| args.get(1).is_some_and(|g| g.eq_ignore_ascii_case(if f.male { "Male" } else { "Female" }))),
        "PLAYER_HAS_ACHIEVEMENT" => f.is_some_and(|f| args.get(1).is_some_and(|a| f.achievements.contains(a))),
        "PLAYER_HAS_CAUGHT_FISH" => f.is_some_and(|f| args.get(1).is_some_and(|id| f.fish_caught.contains_key(crate::item::split_qualified(id).1))),
        "PLAYER_HAS_CONVERSATION_TOPIC" => f.is_some_and(|f| args.get(1).is_some_and(|t| f.conversation_topics.contains_key(t))),
        "PLAYER_HAS_CRAFTING_RECIPE" => f.is_some_and(|f| f.crafting_recipes.contains_key(&args[1..].join(" "))),
        "PLAYER_HAS_COOKING_RECIPE" => f.is_some_and(|f| f.cooking_recipes.contains_key(&args[1..].join(" "))),
        "PLAYER_HAS_DIALOGUE_ANSWER" => f.is_some_and(|f| args.get(1).is_some_and(|a| f.dialogue_answers.contains(a))),
        "PLAYER_HAS_FLAG" => args.get(1).is_some_and(|m| ctx.mail.iter().any(|x| x == m)),
        "PLAYER_HAS_MAIL" => {
            let id = args.get(1);
            let kind = args.get(2).map(|k| k.to_ascii_lowercase()).unwrap_or_else(|| "received".into());
            id.is_some_and(|m| {
                let received = ctx.mail.iter().any(|x| x == m);
                let in_box = f.is_some_and(|f| f.mailbox.contains(m));
                match kind.as_str() {
                    "mailbox" => in_box,
                    "any" => received || in_box,
                    "tomorrow" => false,
                    _ => received,
                }
            })
        }
        "PLAYER_HAS_ITEM" => f.is_some_and(|f| {
            let id = args.get(1).map(|i| crate::item::split_qualified(i).1.to_string()).unwrap_or_default();
            let n = f.inventory.slots.iter().flatten().filter(|i| i.object_id() == Some(id.as_str()) || i.qualified_id().ends_with(&format!("){id}"))).map(|i| i.stack() as i64).sum::<i64>();
            n >= num(args, 2, 1) && n <= num(args, 3, i64::MAX)
        }),
        "PLAYER_HAS_PROFESSION" => f.is_some_and(|f| args.get(1).is_some_and(|p| f.has_profession(p) || p.parse::<i32>().ok().and_then(crate::professions::name_for_original_id).is_some_and(|n| f.has_profession(n)))),
        "PLAYER_HAS_RUN_TRIGGER_ACTION" => f.is_some_and(|f| args.get(1).is_some_and(|a| f.trigger_actions_run.contains(a))),
        "PLAYER_HAS_SECRET_NOTE" => f.is_some_and(|f| args.get(1).is_some_and(|n| f.notes_read.contains(n))),
        "PLAYER_HAS_SEEN_EVENT" => args.get(1).is_some_and(|m| ctx.events_seen.iter().any(|x| x == m)),
        "PLAYER_HAS_TOWN_KEY" => f.is_some_and(|f| f.mail_received.iter().any(|m| m == "HasTownKey")),
        "PLAYER_HAS_TRASH_CAN_LEVEL" => f.is_some_and(|f| in_range(f.counters.get("TrashCanLevel").copied().unwrap_or(0) as i64, args, 1)),
        "PLAYER_LOCATION_NAME" | "PLAYER_LOCATION_UNIQUE_NAME" => w.is_some_and(|w| args.iter().skip(1).any(|l| l.eq_ignore_ascii_case(&w.current))),
        "PLAYER_SHIPPED_BASIC_ITEM" => f.is_some_and(|f| {
            let id = args.get(1).map(|i| crate::item::split_qualified(i).1.to_string()).unwrap_or_default();
            let n = f.shipped.get(&id).copied().unwrap_or(0) as i64;
            n >= num(args, 2, 1) && n <= num(args, 3, i64::MAX)
        }),
        "PLAYER_SPECIAL_ORDER_ACTIVE" => f.is_some_and(|f| args.get(1).is_some_and(|id| f.special_order.as_ref().is_some_and(|o| &o.id == id))),
        "PLAYER_SPECIAL_ORDER_COMPLETE" => f.is_some_and(|f| args.get(1).is_some_and(|id| f.orders_completed.contains(id))),
        "PLAYER_SPECIAL_ORDER_RULE_ACTIVE" => false,
        "PLAYER_STAT" => w.is_some_and(|w| args.get(1).is_some_and(|s| in_range(w.player_stat(s), args, 2))),
        "PLAYER_VISITED_LOCATION" => f.is_some_and(|f| args.iter().skip(1).any(|l| f.counters.contains_key(&format!("Port.Visited.{l}")))),
        "PLAYER_FRIENDSHIP_POINTS" | "PLAYER_HEARTS" => f.is_some_and(|f| {
            let points = |npc: &str| f.friendships.get(npc).map(|x| x.points).unwrap_or(0) as i64;
            let scale = if name == "PLAYER_HEARTS" { crate::farmer::Friendship::POINTS_PER_HEART as i64 } else { 1 };
            let npc = args.get(1).map(String::as_str).unwrap_or("Any");
            let ok = |v: i64| in_range(v / scale, args, 2);
            match npc {
                "Any" | "AnyDateable" => f.friendships.keys().any(|n| ok(points(n))),
                n => ok(points(n)),
            }
        }),
        "PLAYER_HAS_CHILDREN" => f.is_some_and(|f| in_range(f.children.len() as i64, args, 1) && (args.len() > 1 || !f.children.is_empty())),
        "PLAYER_HAS_PET" => w.is_some_and(|w| w.pet.is_some()),
        "PLAYER_IS_DATING" | "PLAYER_IS_ENGAGED" | "PLAYER_IS_MARRIED" | "PLAYER_IS_DIVORCED" | "PLAYER_IS_ROOMMATE" => f.is_some_and(|f| {
            use crate::farmer::RelationStatus as R;
            let want = |s: &R| match name.as_str() {
                "PLAYER_IS_DATING" => matches!(s, R::Dating),
                "PLAYER_IS_ENGAGED" => matches!(s, R::Engaged { .. }),
                "PLAYER_IS_MARRIED" => matches!(s, R::Married),
                "PLAYER_IS_DIVORCED" => matches!(s, R::Divorced),
                _ => false,
            };
            let npc = args.get(1).map(String::as_str).unwrap_or("Any");
            f.friendships.iter().any(|(n, x)| (npc == "Any" || n == npc) && want(&x.status))
        }),

        // Randomization.
        "RANDOM" => {
            let mut p: f64 = args.first().and_then(|v| v.parse().ok()).unwrap_or(1.0);
            if args.iter().any(|a| a == "@addDailyLuck") {
                p += w.map(|w| w.daily_luck).unwrap_or(0.0);
            }
            ctx.rng.chance(p)
        }
        "SYNCED_RANDOM" | "SYNCED_DAY_RANDOM" => {
            let (interval, key, chance) = if name == "SYNCED_DAY_RANDOM" { ("day", args.first(), args.get(1)) } else { (args.first().map(String::as_str).unwrap_or("day"), args.get(1), args.get(2)) };
            let mut p: f64 = chance.and_then(|v| v.parse().ok()).unwrap_or(0.0);
            if args.iter().any(|a| a == "@addDailyLuck") {
                p += w.map(|w| w.daily_luck).unwrap_or(0.0);
            }
            let period = match interval.to_ascii_lowercase().as_str() {
                "tick" | "day" => ctx.clock.days_played() as i64,
                "season" => (ctx.clock.days_played() / 28) as i64,
                "year" => ctx.clock.year as i64,
                _ => ctx.clock.days_played() as i64,
            };
            let seed = w.map(|w| w.seed).unwrap_or(0) as i64;
            Rng::new(Rng::seed_from(&[seed, period, hash_key(key.map(String::as_str).unwrap_or(""))])).chance(p)
        }
        "SYNCED_CHOICE" => {
            let period = match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
                Some("season") => (ctx.clock.days_played() / 28) as i64,
                Some("year") => ctx.clock.year as i64,
                _ => ctx.clock.days_played() as i64,
            };
            let seed = w.map(|w| w.seed).unwrap_or(0) as i64;
            let (lo, hi) = (num(args, 2, 0), num(args, 3, 0));
            let pick = Rng::new(Rng::seed_from(&[seed, period, hash_key(args.get(1).map(String::as_str).unwrap_or(""))])).range(lo as i32, hi as i32 + 1);
            args.iter().skip(4).any(|c| c.parse::<i32>().ok() == Some(pick))
        }
        "SYNCED_SUMMER_RAIN_RANDOM" => {
            let base: f64 = args.first().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let per_day: f64 = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let p = base + per_day * ctx.clock.day as f64;
            let seed = w.map(|w| w.seed).unwrap_or(0) as i64;
            Rng::new(Rng::seed_from(&[seed, ctx.clock.days_played() as i64, 7713])).chance(p)
        }

        // Items.
        n if n.starts_with("ITEM_") => {
            let Some(item) = args.first().and_then(|t| item_for(&qw, t)) else { return neg };
            let db = w.map(|w| &w.items);
            let id = item.object_id().map(String::from).unwrap_or_else(|| crate::item::split_qualified(&item.qualified_id()).1.to_string());
            let obj = db.and_then(|db| item.object_id().and_then(|i| db.object(i)));
            match n {
                "ITEM_ID" => args.iter().skip(1).any(|a| a == &item.qualified_id() || crate::item::split_qualified(a).1 == id),
                "ITEM_ID_PREFIX" => args.iter().skip(1).any(|a| item.qualified_id().starts_with(a.as_str()) || id.starts_with(a.as_str())),
                "ITEM_NUMERIC_ID" => id.parse::<i64>().is_ok_and(|v| in_range(v, args, 1)),
                "ITEM_CATEGORY" => {
                    let cat = obj.map(|o| o.data.category as i64).unwrap_or(0);
                    if args.len() < 2 {
                        cat != 0
                    } else {
                        args.iter().skip(1).any(|c| c.parse::<i64>().ok() == Some(cat))
                    }
                }
                "ITEM_HAS_EXPLICIT_OBJECT_CATEGORY" => obj.is_some_and(|o| o.data.category != 0),
                "ITEM_OBJECT_TYPE" => obj.is_some_and(|o| args.iter().skip(1).any(|t| t.eq_ignore_ascii_case(&o.data.kind))),
                "ITEM_TYPE" => {
                    let q = item.qualified_id();
                    let ty = crate::item::split_qualified(&q).0.to_string();
                    args.iter().skip(1).any(|t| t.trim_matches(|c| c == '(' || c == ')').eq_ignore_ascii_case(&ty))
                }
                "ITEM_PRICE" => db.is_some_and(|db| in_range(db.sell_price(item) as i64, args, 1)),
                "ITEM_QUALITY" => in_range(item.quality() as i32 as i64, args, 1),
                "ITEM_STACK" => in_range(item.stack() as i64, args, 1),
                "ITEM_EDIBILITY" => obj.is_some_and(|o| in_range(o.data.edibility as i64, args, 1)),
                "ITEM_CONTEXT_TAG" | "ITEM_HAS_TAG" => db.is_some_and(|db| {
                    let tags: Vec<String> = args[1..].to_vec();
                    db.matches_tags(item, &tags)
                }),
                _ => {
                    report_unknown(n);
                    false
                }
            }
        }
        other => {
            report_unknown(other);
            false
        }
    };
    r != neg
}

pub fn weekday_name(i: i32) -> &'static str {
    WEEKDAYS[(i.rem_euclid(7)) as usize]
}

impl World {
    /// Query context helpers: check a condition about this world, with
    /// optional target/input items.
    pub fn check_query(&self, condition: Option<&str>, target: Option<&Item>, input: Option<&Item>, rng: &mut Rng) -> bool {
        let mail = self.farmer.mail_received.clone();
        let seen = self.farmer.events_seen.clone();
        let mut ctx = QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng, world: Some(self) };
        check_with(condition, &mut ctx, QueryWorld { world: Some(self), target, input })
    }

    /// A `PLAYER_STAT` value.
    pub fn player_stat(&self, name: &str) -> i64 {
        let f = &self.farmer;
        match name.to_ascii_lowercase().as_str() {
            "daysplayed" => self.clock.days_played() as i64,
            "itemsshipped" => f.shipped.values().map(|v| *v as i64).sum(),
            "fishcaught" => f.fish_caught.values().map(|v| v.0 as i64).sum(),
            "monsterskilled" => self.monsters_killed as i64,
            "questscompleted" => f.quests_completed as i64,
            "itemscrafted" => f.crafting_recipes.values().map(|v| *v as i64).sum(),
            "itemscooked" => f.cooking_recipes.values().map(|v| *v as i64).sum(),
            "timesfished" | "stepstaken" | "geodescracked" | "giftsgiven" => f.counters.get(name).copied().unwrap_or(0) as i64,
            _ => f.counters.get(name).copied().unwrap_or(0) as i64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_queries() {
        let clock = Clock::default();
        let mut rng = Rng::new(1);
        let mail = vec!["intro".to_string()];
        let mut ctx = QueryCtx { clock: &clock, weather: Weather::Rain, mail: &mail, events_seen: &[], location: "Town", rng: &mut rng, world: None };
        assert!(check(Some("SEASON Spring Summer"), &mut ctx));
        assert!(!check(Some("SEASON Fall"), &mut ctx));
        assert!(check(Some("!SEASON Fall, WEATHER Here Rain"), &mut ctx));
        assert!(check(Some("DAY_OF_WEEK Monday"), &mut ctx));
        assert!(check(Some("PLAYER_HAS_MAIL Current intro"), &mut ctx));
        assert!(!check(Some("YEAR 2"), &mut ctx));
        assert!(check(None, &mut ctx));
        assert!(check(Some(r#"ANY "SEASON Fall" "WEATHER Here Rain""#), &mut ctx));
        assert!(!check(Some(r#"ANY "SEASON Fall" "YEAR 3""#), &mut ctx));
        assert!(!check(Some("NO_SUCH_QUERY"), &mut ctx), "unknown queries fail");
        assert!(check(Some("SEASON_DAY Spring 1 Fall 3"), &mut ctx));
    }

    #[test]
    fn tokens_keep_quotes() {
        assert_eq!(tokens(r#"ANY "A b" "C""#), vec!["ANY", "A b", "C"]);
    }
}
