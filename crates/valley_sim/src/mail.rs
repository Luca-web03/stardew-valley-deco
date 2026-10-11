//! Letters (`Data/mail`): text with `^` line breaks, an optional `[#]Title`
//! suffix and `%item ... %%` attachments, per the public mail format.

use crate::item::{Item, ItemDb};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Letter {
    pub text: String,
    pub title: Option<String>,
    pub items: Vec<Item>,
    pub money: i32,
    pub recipe: Option<String>,
    /// `%item quest <id>`: a quest the letter starts.
    pub quest: Option<String>,
    /// `%item conversationTopic <id> <days>`.
    pub topic: Option<(String, i32)>,
}

pub fn parse(raw: &str, db: &ItemDb, farmer_name: &str) -> Letter {
    let mut l = Letter::default();
    let (body, title) = match raw.split_once("[#]") {
        Some((b, t)) => (b.to_string(), Some(t.trim().to_string())),
        None => (raw.to_string(), None),
    };
    l.title = title;
    let mut text = String::new();
    let mut rest = body.as_str();
    while let Some(start) = rest.find("%item") {
        text.push_str(&rest[..start]);
        let after = &rest[start + 5..];
        let end = after.find("%%").unwrap_or(after.len());
        let spec: Vec<&str> = after[..end].split_whitespace().collect();
        match spec.as_slice() {
            ["object", pairs @ ..] | ["id", pairs @ ..] => {
                for c in pairs.chunks(2) {
                    let id = c[0];
                    let n = c.get(1).and_then(|n| n.parse().ok()).unwrap_or(1);
                    if let Some(item) = db.create(id, n) {
                        l.items.push(item);
                    }
                }
            }
            ["bigobject", id, ..] | ["BigObject", id, ..] => {
                if let Some(item) = db.create(&format!("(BC){id}"), 1) {
                    l.items.push(item);
                }
            }
            ["furniture", id, ..] => {
                if let Some(item) = db.create(&format!("(F){id}"), 1) {
                    l.items.push(item);
                }
            }
            ["tools", kind, ..] => {
                if let Some(item) = db.create(&format!("(T){kind}"), 1) {
                    l.items.push(item);
                }
            }
            ["quest", id, ..] => l.quest = Some(id.to_string()),
            ["conversationTopic", id, days @ ..] => {
                l.topic = Some((id.to_string(), days.first().and_then(|d| d.parse().ok()).unwrap_or(4)));
            }
            ["money", a, b @ ..] => {
                let lo: i32 = a.parse().unwrap_or(0);
                let hi: i32 = b.first().and_then(|x| x.parse().ok()).unwrap_or(lo);
                l.money = (lo + hi) / 2;
            }
            ["cookingRecipe", name @ ..] | ["craftingRecipe", name @ ..] => {
                if !name.is_empty() {
                    l.recipe = Some(name.join(" "));
                }
            }
            _ => {}
        }
        rest = if end + 2 <= after.len() { &after[end + 2..] } else { "" };
    }
    text.push_str(rest);
    l.text = text.replace('@', farmer_name).replace('^', "\n").trim().to_string();
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_attachments_and_title() {
        let db = ItemDb::default();
        let l = parse("Hi @,^here's some cash.%item money 100 200 %%[#]Gift", &db, "Kai");
        assert_eq!(l.text, "Hi Kai,\nhere's some cash.");
        assert_eq!(l.money, 150);
        assert_eq!(l.title.as_deref(), Some("Gift"));
    }
}
