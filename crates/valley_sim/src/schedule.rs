//! NPC schedule strings ("900 Town 20 14 2/1200 ...") and key selection,
//! following the publicly documented schedule data format.

use std::collections::HashMap;

use crate::time::Clock;
use crate::{Dir, TilePos};

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleEntry {
    pub time: i32,
    pub location: String,
    pub tile: TilePos,
    pub facing: Dir,
    /// Optional end-of-route behaviour / animation name.
    pub behavior: Option<String>,
    /// Optional dialogue key shown on arrival ("\"Strings\\...\"").
    pub message: Option<String>,
    /// `a<time>`: the time to arrive by (the NPC leaves early enough).
    pub arrive: bool,
}

/// Parse a schedule script. `GOTO key` redirects are resolved through `all`;
/// conditional prefixes like "NOT friendship ..." are treated as passed.
pub fn parse(script: &str, all: &HashMap<String, String>, depth: u32) -> Vec<ScheduleEntry> {
    parse_with(script, all, depth, &|_| 0)
}

/// Like [`parse`], with the player's hearts per NPC for
/// `NOT friendship <npc> <hearts> [...]`: when the player has at least that
/// many hearts with every listed NPC, the default schedule is used instead.
pub fn parse_with(script: &str, all: &HashMap<String, String>, depth: u32, hearts: &dyn Fn(&str) -> i32) -> Vec<ScheduleEntry> {
    let mut out = Vec::new();
    let parts: Vec<&str> = script.split('/').map(str::trim).filter(|s| !s.is_empty()).collect();
    for (i, part) in parts.iter().enumerate() {
        let toks: Vec<&str> = part.split_whitespace().collect();
        if toks.is_empty() {
            continue;
        }
        if toks[0] == "GOTO" && depth < 4 {
            if let Some(target) = toks.get(1).and_then(|k| all.get(*k)) {
                return parse_with(target, all, depth + 1, hearts);
            }
            continue;
        }
        if i == 0 && toks[0] == "NOT" && toks.get(1) == Some(&"friendship") {
            let blocked = toks[2..].chunks(2).all(|c| c.len() == 2 && hearts(c[0]) >= c[1].parse().unwrap_or(99));
            if blocked && depth < 4 {
                let fallback = all.get("default").or_else(|| all.get("spring"));
                return fallback.map(|s| parse_with(s, all, depth + 1, hearts)).unwrap_or_default();
            }
            continue;
        }
        if i == 0 && (toks[0] == "NOT" || toks[0] == "MAIL") {
            // Other condition prefixes are treated as passed.
            continue;
        }
        let Some(time) = parse_time(toks[0]) else { continue };
        if toks.len() < 4 {
            continue;
        }
        // Location may be omitted (then it's "the previous location").
        let (location, rest) = if toks[1].parse::<i32>().is_ok() {
            (out.last().map(|e: &ScheduleEntry| e.location.clone()).unwrap_or_default(), &toks[1..])
        } else {
            (toks[1].to_string(), &toks[2..])
        };
        if rest.len() < 2 {
            continue;
        }
        let (Ok(x), Ok(y)) = (rest[0].parse(), rest[1].parse()) else { continue };
        let facing = rest.get(2).and_then(|d| d.parse::<i32>().ok()).map(Dir::from_index).unwrap_or(Dir::Down);
        let mut behavior = None;
        let mut message = None;
        for t in rest.iter().skip(3) {
            if t.starts_with('"') {
                message = Some(t.trim_matches('"').to_string());
            } else if behavior.is_none() {
                behavior = Some(t.to_string());
            }
        }
        out.push(ScheduleEntry { time, location, tile: TilePos::new(x, y), facing, behavior, message, arrive: toks[0].starts_with('a') });
    }
    out
}

fn parse_time(s: &str) -> Option<i32> {
    let s = s.strip_prefix('a').unwrap_or(s);
    s.parse().ok()
}

/// Choose which schedule key applies today (most specific first). On rainy
/// days `rain2` is considered when `rain2_roll` (a 50% roll) is true.
pub fn pick_key(sched: &HashMap<String, String>, clock: &Clock, hearts: i32, raining: bool) -> Option<String> {
    pick_key_with(sched, clock, hearts, raining, true)
}

pub fn pick_key_with(sched: &HashMap<String, String>, clock: &Clock, hearts: i32, raining: bool, rain2_roll: bool) -> Option<String> {
    let season = clock.season.key();
    let dow = clock.weekday();
    let mut keys = vec![format!("{season}_{}", clock.day)];
    for h in (1..=13).rev() {
        if hearts >= h {
            keys.push(format!("{}_{h}", clock.day));
        }
    }
    keys.push(clock.day.to_string());
    if raining {
        if rain2_roll {
            keys.push("rain2".into());
        }
        keys.push("rain".into());
    }
    for h in (1..=13).rev() {
        if hearts >= h {
            keys.push(format!("{season}_{dow}_{h}"));
        }
    }
    keys.push(format!("{season}_{dow}"));
    keys.push(dow.to_string());
    keys.push(season.to_string());
    keys.push(format!("spring_{dow}"));
    keys.push("spring".into());
    keys.push("default".into());
    keys.into_iter().find(|k| sched.contains_key(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_and_goto() {
        let mut all = HashMap::new();
        all.insert("default".to_string(), "800 Town 20 14 2/1200 30 20 3 sit \"Strings\\x\"".to_string());
        all.insert("Tue".to_string(), "GOTO default".to_string());
        let e = parse(&all["Tue"], &all, 0);
        assert_eq!(e.len(), 2);
        assert_eq!(e[1].location, "Town");
        assert_eq!(e[1].facing, Dir::Left);
        assert_eq!(e[1].behavior.as_deref(), Some("sit"));
        assert_eq!(e[1].message.as_deref(), Some("Strings\\x"));
        let c = Clock::default();
        assert_eq!(pick_key(&all, &c, 0, false).as_deref(), Some("default"));
    }

    #[test]
    fn not_friendship_falls_back_and_arrival_times_are_marked() {
        let mut all = HashMap::new();
        all.insert("default".to_string(), "900 Town 1 1 2".to_string());
        all.insert("Fri".to_string(), "NOT friendship Mara 6/a1300 Beach 5 5 2".to_string());
        let low = parse_with(&all["Fri"], &all, 0, &|_| 2);
        assert_eq!((low[0].location.as_str(), low[0].time, low[0].arrive), ("Beach", 1300, true));
        let high = parse_with(&all["Fri"], &all, 0, &|n| if n == "Mara" { 8 } else { 0 });
        assert_eq!(high[0].location, "Town");
    }

    #[test]
    fn key_order_follows_the_documented_fallbacks() {
        let mut all = HashMap::new();
        all.insert("spring".to_string(), "x".to_string());
        all.insert("spring_Mon".to_string(), "y".to_string());
        all.insert("rain2".to_string(), "z".to_string());
        all.insert("rain".to_string(), "w".to_string());
        let mut c = Clock::default();
        c.season = crate::time::Season::Summer;
        assert_eq!(pick_key_with(&all, &c, 0, false, true).as_deref(), Some("spring_Mon"));
        assert_eq!(pick_key_with(&all, &c, 0, true, false).as_deref(), Some("rain"));
        assert_eq!(pick_key_with(&all, &c, 0, true, true).as_deref(), Some("rain2"));
    }
}
