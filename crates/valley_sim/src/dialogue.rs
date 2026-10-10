//! Dialogue strings: page splitting, portrait emotions and substitutions,
//! following the publicly documented dialogue command syntax.

use std::collections::HashMap;

use crate::time::Clock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Emotion {
    #[default]
    Neutral,
    Happy,
    Sad,
    Unique,
    Love,
    Angry,
    /// Explicit portrait index ($0..$9 style).
    Index(i32),
}

impl Emotion {
    /// Portrait frame index in a 2-column portrait sheet.
    pub fn portrait_index(self) -> i32 {
        match self {
            Emotion::Neutral => 0,
            Emotion::Happy => 1,
            Emotion::Sad => 2,
            Emotion::Unique => 3,
            Emotion::Love => 4,
            Emotion::Angry => 5,
            Emotion::Index(i) => i,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DialoguePage {
    pub text: String,
    pub emotion: Emotion,
}

pub struct Substitutions<'a> {
    pub farmer_name: &'a str,
    pub farm_name: &'a str,
    pub male: bool,
}

/// Split raw dialogue into displayable pages.
pub fn parse(raw: &str, subs: &Substitutions) -> Vec<DialoguePage> {
    let mut pages = Vec::new();
    // "#$b#" and "#$e#" separate pages; a bare "#" between sentences also
    // separates. "$q"/"$r" question syntax is reduced to its prompt text.
    let normalized = raw.replace("#$b#", "\u{1}").replace("#$e#", "\u{1}");
    for chunk in normalized.split('\u{1}') {
        for part in chunk.split('#') {
            let part = part.trim();
            if part.is_empty() || part.starts_with("$q") || part.starts_with("$r") || part.starts_with("$p") {
                continue;
            }
            let (text, emotion) = strip_emotion(part);
            let text = substitute(&text, subs);
            if !text.is_empty() {
                pages.push(DialoguePage { text, emotion });
            }
        }
    }
    if pages.is_empty() {
        pages.push(DialoguePage { text: "...".into(), emotion: Emotion::Neutral });
    }
    pages
}

fn strip_emotion(s: &str) -> (String, Emotion) {
    if let Some(idx) = s.rfind('$') {
        let tail = &s[idx + 1..];
        let emo = match tail {
            "h" => Some(Emotion::Happy),
            "s" => Some(Emotion::Sad),
            "u" => Some(Emotion::Unique),
            "l" => Some(Emotion::Love),
            "a" => Some(Emotion::Angry),
            "neutral" | "n" => Some(Emotion::Neutral),
            t if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) => t.parse().ok().map(Emotion::Index),
            _ => None,
        };
        if let Some(e) = emo {
            return (s[..idx].trim_end().to_string(), e);
        }
    }
    (s.to_string(), Emotion::Neutral)
}

/// Apply gendered "male^female" splits and @ / farm-name substitutions.
pub fn substitute(s: &str, subs: &Substitutions) -> String {
    let s = if s.contains('^') {
        let mut it = s.splitn(2, '^');
        let m = it.next().unwrap_or("");
        let f = it.next().unwrap_or(m);
        if subs.male { m.to_string() } else { f.to_string() }
    } else {
        s.to_string()
    };
    s.replace('@', subs.farmer_name).replace("%farm", subs.farm_name)
}

/// Choose today's dialogue key for an NPC, most specific first.
pub fn pick_key(dialogue: &HashMap<String, String>, clock: &Clock, hearts: i32, first_meeting: bool, raining: bool) -> Option<String> {
    if first_meeting && dialogue.contains_key("Introduction") {
        return Some("Introduction".into());
    }
    let season = clock.season.key();
    let dow = clock.weekday();
    let mut keys = vec![format!("{season}_{}", clock.day)];
    if clock.year > 1 {
        keys.push(format!("{}_{}_{}", season, clock.day, clock.year));
    }
    if raining {
        keys.push("rain".into());
    }
    for h in (2..=10).rev().step_by(2) {
        if hearts >= h {
            keys.push(format!("{season}_{dow}{h}"));
        }
    }
    keys.push(format!("{season}_{dow}"));
    for h in (2..=10).rev().step_by(2) {
        if hearts >= h {
            keys.push(format!("{dow}{h}"));
        }
    }
    keys.push(dow.to_string());
    keys.into_iter().find(|k| dialogue.contains_key(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subs() -> Substitutions<'static> {
        Substitutions { farmer_name: "Kai", farm_name: "Brook", male: false }
    }

    #[test]
    fn pages_and_emotions() {
        let p = parse("Hi @!$h#$b#Second page.$s", &subs());
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].text, "Hi Kai!");
        assert_eq!(p[0].emotion, Emotion::Happy);
        assert_eq!(p[1].emotion, Emotion::Sad);
        let p = parse("Hello sir^Hello madam", &subs());
        assert_eq!(p[0].text, "Hello madam");
    }

    #[test]
    fn key_priority() {
        let mut d = HashMap::new();
        d.insert("Mon".to_string(), "a".to_string());
        d.insert("spring_Mon".to_string(), "b".to_string());
        d.insert("Introduction".to_string(), "c".to_string());
        let c = Clock::default();
        assert_eq!(pick_key(&d, &c, 0, true, false).as_deref(), Some("Introduction"));
        assert_eq!(pick_key(&d, &c, 0, false, false).as_deref(), Some("spring_Mon"));
        d.remove("spring_Mon");
        assert_eq!(pick_key(&d, &c, 0, false, false).as_deref(), Some("Mon"));
    }
}
