//! Game state queries: the condition strings used throughout data assets
//! ("SEASON spring, !WEATHER Here Rain"), per the public query syntax.
//! A subset is implemented; unknown queries pass (and are reported).

use engine::Rng;

use crate::time::{Clock, Season, Weather, WEEKDAYS};

pub struct QueryCtx<'a> {
    pub clock: &'a Clock,
    pub weather: Weather,
    pub mail: &'a [String],
    pub events_seen: &'a [String],
    pub location: &'a str,
    pub rng: &'a mut Rng,
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

pub fn check(cond: Option<&str>, ctx: &mut QueryCtx) -> bool {
    let Some(cond) = cond else { return true };
    let cond = cond.trim();
    if cond.is_empty() || cond.eq_ignore_ascii_case("TRUE") {
        return true;
    }
    if cond.eq_ignore_ascii_case("FALSE") {
        return false;
    }
    split_conditions(cond).iter().all(|c| check_one(c, ctx))
}

fn check_one(q: &str, ctx: &mut QueryCtx) -> bool {
    let (neg, q) = match q.strip_prefix('!') {
        Some(r) => (true, r.trim()),
        None => (false, q),
    };
    let toks: Vec<&str> = q.split_whitespace().collect();
    let Some(name) = toks.first() else { return true };
    let args = &toks[1..];
    let r = match name.to_ascii_uppercase().as_str() {
        "SEASON" | "LOCATION_SEASON" => {
            let a = if name.eq_ignore_ascii_case("LOCATION_SEASON") && !args.is_empty() { &args[1..] } else { args };
            a.iter().any(|s| Season::parse(s) == Some(ctx.clock.season))
        }
        "DAY_OF_WEEK" => args.iter().any(|d| {
            let d = d.to_ascii_lowercase();
            let wd = ctx.clock.weekday().to_ascii_lowercase();
            d.starts_with(&wd) || d.parse::<i32>().ok() == Some((ctx.clock.weekday_index() + 1) % 7)
        }),
        "DAY_OF_MONTH" => args.iter().any(|d| match *d {
            "even" => ctx.clock.day % 2 == 0,
            "odd" => ctx.clock.day % 2 == 1,
            n => n.parse::<i32>().ok() == Some(ctx.clock.day),
        }),
        "YEAR" => {
            let min = args.first().and_then(|v| v.parse().ok()).unwrap_or(1);
            let max = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(i32::MAX);
            ctx.clock.year >= min && ctx.clock.year <= max
        }
        "TIME" => {
            let min = args.first().and_then(|v| v.parse().ok()).unwrap_or(0);
            let max = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(i32::MAX);
            ctx.clock.time >= min && ctx.clock.time <= max
        }
        "WEATHER" => args.iter().skip(1).any(|w| {
            let w = w.to_ascii_lowercase();
            match ctx.weather {
                Weather::Sun => w == "sun",
                Weather::Rain => w == "rain",
                Weather::Storm => w == "storm" || w == "rain",
                Weather::Snow => w == "snow",
                Weather::Wind => w == "wind",
            }
        }),
        "IS_RAINING" | "LOCATION_IS_RAINING" => ctx.weather.is_raining(),
        "PLAYER_HAS_MAIL" => args.get(1).is_some_and(|m| ctx.mail.iter().any(|x| x == m)),
        "PLAYER_HAS_SEEN_EVENT" => args.get(1).is_some_and(|m| ctx.events_seen.iter().any(|x| x == m)),
        "LOCATION_NAME" => args.iter().skip(1).any(|l| l.eq_ignore_ascii_case(ctx.location)),
        "RANDOM" => {
            let p: f64 = args.first().and_then(|v| v.parse().ok()).unwrap_or(1.0);
            ctx.rng.chance(p)
        }
        "DAYS_PLAYED" => {
            let min = args.first().and_then(|v| v.parse().ok()).unwrap_or(0);
            let max = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(i32::MAX);
            let d = ctx.clock.days_played();
            d >= min && d <= max
        }
        _ => true,
    };
    r != neg
}

pub fn weekday_name(i: i32) -> &'static str {
    WEEKDAYS[(i.rem_euclid(7)) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_queries() {
        let clock = Clock::default();
        let mut rng = Rng::new(1);
        let mail = vec!["intro".to_string()];
        let mut ctx = QueryCtx { clock: &clock, weather: Weather::Rain, mail: &mail, events_seen: &[], location: "Town", rng: &mut rng };
        assert!(check(Some("SEASON Spring Summer"), &mut ctx));
        assert!(!check(Some("SEASON Fall"), &mut ctx));
        assert!(check(Some("!SEASON Fall, WEATHER Here Rain"), &mut ctx));
        assert!(check(Some("DAY_OF_WEEK Monday"), &mut ctx));
        assert!(check(Some("PLAYER_HAS_MAIL Current intro"), &mut ctx));
        assert!(!check(Some("YEAR 2"), &mut ctx));
        assert!(check(None, &mut ctx));
    }
}
