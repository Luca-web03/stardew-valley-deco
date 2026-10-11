//! The telephone (public behaviour, approx.): a placed telephone (`214`,
//! or a `Port.Telephone` custom field) lists numbers to call. Contacts come
//! from `Port/PhoneContacts` ({ id: { "Name", "Message", "Kind" } }); kinds
//! `ToolUpgrade` (the blacksmith reports on a tool being upgraded),
//! `Construction` (the carpenter on a building going up) and
//! `DishOfTheDay` (today's special) fill in live details, `Text` just says
//! its message. Without the asset, those three services are listed.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::world::{World, WorldEvent};
use crate::TilePos;

const PHONES: &[&str] = &["214", "Dev_Telephone"];

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct PhoneContact {
    pub name: String,
    pub message: String,
    pub kind: String,
}

pub fn default_contacts() -> BTreeMap<String, PhoneContact> {
    let c = |name: &str, kind: &str| PhoneContact { name: name.into(), message: String::new(), kind: kind.into() };
    BTreeMap::from([
        ("Blacksmith".to_string(), c("Blacksmith", "ToolUpgrade")),
        ("Carpenter".to_string(), c("Carpenter", "Construction")),
        ("Saloon".to_string(), c("Saloon", "DishOfTheDay")),
    ])
}

impl World {
    pub fn is_telephone(&self, big_id: &str) -> bool {
        PHONES.contains(&big_id) || self.items.big(big_id).and_then(|b| b.data.custom_fields.as_ref()).is_some_and(|f| f.contains_key("Port.Telephone"))
    }

    /// Action on a telephone: pick someone to call.
    pub(crate) fn use_phone(&mut self, t: TilePos) -> bool {
        let Some(o) = self.location().object(t).filter(|o| o.big) else { return false };
        if !self.is_telephone(&o.item_id) {
            return false;
        }
        if self.answer_call() {
            return true;
        }
        let options: Vec<(String, String)> = self.phone_contacts.iter().map(|(id, c)| (format!("phone:{id}"), c.name.clone())).chain([("phone:".to_string(), "Hang up".to_string())]).collect();
        self.sound("telephone_buttonPush");
        self.emit(WorldEvent::Choice { text: "Who do you want to call?".into(), options });
        true
    }

    /// Today's special at the shop selling `DISH_OF_THE_DAY`.
    pub fn todays_dish(&self) -> Option<String> {
        for (id, data) in &self.shops {
            let Some(pos) = data.items.iter().position(|e| e.item_id.trim() == "DISH_OF_THE_DAY") else { continue };
            let mut d = data.clone();
            d.items.truncate(pos + 1);
            let mut rng = self.day_rng(3);
            let mail = self.farmer.mail_received.clone();
            let seen = self.farmer.events_seen.clone();
            let mut ctx = crate::gsq::QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng, world: Some(self) };
            let stock = crate::shop::build_stock(id, &d, &self.items, &mut ctx);
            return stock.entries.last().map(|e| self.items.display_name(&e.item));
        }
        None
    }

    pub(crate) fn call(&mut self, id: &str) {
        let Some(c) = self.phone_contacts.get(id).cloned() else { return };
        self.sound("phone");
        let text = match c.kind.as_str() {
            "ToolUpgrade" => match &self.farmer.tool_upgrade {
                Some((tool, 0)) => format!("{}: Your {} is ready! Come pick it up.", c.name, self.items.display_name(&crate::item::Item::Tool(tool.clone()))),
                Some((tool, d)) => format!("{}: Your {} will be ready in {d} day{}.", c.name, self.items.display_name(&crate::item::Item::Tool(tool.clone())), if *d == 1 { "" } else { "s" }),
                None => format!("{}: Nothing on the anvil for you right now. Bring a tool by if you want it upgraded.", c.name),
            },
            "Construction" => match self.buildings.iter().find(|b| b.days_left > 0) {
                Some(b) => format!("{}: Your {} will be done in {} day{}.", c.name, self.buildings_data.get(&b.kind).map(|d| d.name.clone()).unwrap_or(b.kind.clone()), b.days_left, if b.days_left == 1 { "" } else { "s" }),
                None => format!("{}: No construction going on. Stop by if you have a project in mind!", c.name),
            },
            "DishOfTheDay" => match self.todays_dish() {
                Some(d) => format!("{}: Today's special is {d}.", c.name),
                None => format!("{}: No special today, sorry.", c.name),
            },
            _ => format!("{}: {}", c.name, self.items.expand_text(&c.message)),
        };
        self.emit(WorldEvent::Message(text));
    }
}
