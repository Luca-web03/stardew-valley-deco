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

/// What an answer leads to.
#[derive(Clone, Debug, PartialEq)]
pub enum Reaction {
    /// Another key in the NPC's dialogue table.
    Key(String),
    /// Inline reaction text.
    Text(String),
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub friendship: i32,
    pub reaction: Reaction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub prompt: String,
    pub choices: Vec<Choice>,
}

/// Pages plus an optional question at the end.
#[derive(Clone, Debug, PartialEq)]
pub struct Parsed {
    pub pages: Vec<DialoguePage>,
    pub question: Option<Question>,
}

/// Parse dialogue including `$q`/`$r` questions and `$y` quick questions.
pub fn parse_full(raw: &str, subs: &Substitutions) -> Parsed {
    // Quick question: $y 'Question_Answer1_Reaction1_Answer2_Reaction2'
    if let Some(start) = raw.find("$y") {
        let before = strip_trailing_breaks(&raw[..start]);
        let body = raw[start + 2..].trim().trim_matches('\'');
        let parts: Vec<&str> = body.split('_').collect();
        if parts.len() >= 3 {
            let mut pages = if before.trim().is_empty() { vec![] } else { parse(before, subs) };
            let prompt = substitute(parts[0], subs);
            let choices = parts[1..]
                .chunks(2)
                .enumerate()
                .map(|(i, c)| Choice {
                    id: format!("y{i}"),
                    text: substitute(c[0], subs),
                    friendship: 0,
                    reaction: c.get(1).map(|r| Reaction::Text(substitute(r, subs))).unwrap_or(Reaction::None),
                })
                .collect();
            pages.retain(|p| p.text != "...");
            return Parsed { pages, question: Some(Question { prompt, choices }) };
        }
    }
    let Some(qpos) = raw.find("$q") else {
        return Parsed { pages: parse(raw, subs), question: None };
    };
    let before = strip_trailing_breaks(&raw[..qpos]);
    let mut pages = if before.trim().is_empty() { vec![] } else { parse(before, subs) };
    let rest = &raw[qpos..];
    let parts: Vec<&str> = rest.split('#').map(str::trim).collect();
    // parts[0] = "$q ids fallback", parts[1] = prompt, then pairs of "$r id pts key", answer.
    let prompt = parts.get(1).map(|p| substitute(strip_emotion(p).0.as_str(), subs)).unwrap_or_default();
    let mut choices = Vec::new();
    let mut i = 2;
    while i < parts.len() {
        if let Some(r) = parts[i].strip_prefix("$r") {
            let t: Vec<&str> = r.split_whitespace().collect();
            let text = parts.get(i + 1).map(|a| substitute(a, subs)).unwrap_or_default();
            choices.push(Choice {
                id: t.first().unwrap_or(&"").to_string(),
                text,
                friendship: t.get(1).and_then(|v| v.parse().ok()).unwrap_or(0),
                reaction: t.get(2).map(|k| Reaction::Key(k.to_string())).unwrap_or(Reaction::None),
            });
            i += 2;
        } else {
            i += 1;
        }
    }
    pages.retain(|p| p.text != "...");
    if choices.is_empty() {
        return Parsed { pages: parse(raw, subs), question: None };
    }
    Parsed { pages, question: Some(Question { prompt, choices }) }
}

/// Remove trailing '#', "$b" and "$e" page-break markers.
fn strip_trailing_breaks(mut s: &str) -> &str {
    loop {
        let t = s.trim_end().trim_end_matches('#');
        let t = t.strip_suffix("$b").or_else(|| t.strip_suffix("$e")).unwrap_or(t);
        if t.len() == s.len() {
            return t;
        }
        s = t;
    }
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
    fn questions_parse() {
        let p = parse_full("Morning!#$b#$q 900/901 q_fallback#Do you like rain?#$r 900 20 q_yes#I love it.#$r 901 -10 q_no#Not really.", &subs());
        assert_eq!(p.pages.len(), 1);
        assert_eq!(p.pages[0].text, "Morning!");
        let q = p.question.unwrap();
        assert_eq!(q.prompt, "Do you like rain?");
        assert_eq!(q.choices.len(), 2);
        assert_eq!(q.choices[0].friendship, 20);
        assert_eq!(q.choices[1].reaction, Reaction::Key("q_no".into()));
        let y = parse_full("$y 'Tea or coffee, @?_Tea_Good choice._Coffee_Bold!'", &subs());
        let q = y.question.unwrap();
        assert_eq!(q.prompt, "Tea or coffee, Kai?");
        assert_eq!(q.choices[1].text, "Coffee");
        assert_eq!(q.choices[1].reaction, Reaction::Text("Bold!".into()));
        assert!(y.pages.is_empty());
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
