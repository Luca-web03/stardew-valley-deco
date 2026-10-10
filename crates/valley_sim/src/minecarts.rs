//! Minecart networks (`Data/Minecarts`, public format subset) and the TV
//! channels (weather, fortune, and the cooking show from
//! `Data/TV/CookingChannel`).

use std::collections::BTreeMap;

use serde::Deserialize;
use valley_content::data::Point;

use crate::time::Weather;
use crate::world::{World, WorldEvent};
use crate::{Dir, TilePos};

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MinecartDestination {
    pub id: String,
    pub display_name: String,
    pub condition: Option<String>,
    pub price: i32,
    pub target_location: String,
    pub target_tile: Point,
    pub target_direction: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MinecartNetwork {
    pub unlock_condition: Option<String>,
    pub locked_message: Option<String>,
    pub choose_destination_message: Option<String>,
    pub destinations: Vec<MinecartDestination>,
}

/// One choice offered by a minecart: (destination id, label).
pub type MinecartChoice = (String, String);

/// TV channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Weather,
    Fortune,
    Cooking,
}

impl World {
    fn gsq_ok(&self, condition: Option<&str>) -> bool {
        let mut rng = self.day_rng(57);
        let mail = self.farmer.mail_received.clone();
        let seen = self.farmer.events_seen.clone();
        let mut ctx = crate::gsq::QueryCtx { clock: &self.clock, weather: self.weather, mail: &mail, events_seen: &seen, location: &self.current, rng: &mut rng };
        crate::gsq::check(condition, &mut ctx)
    }

    /// `MinecartTransport [network] [exclude destination]`.
    pub(crate) fn open_minecart(&mut self, network: &str, exclude: Option<&str>) -> bool {
        let Some(net) = self.minecarts.get(network).cloned() else { return false };
        if !self.gsq_ok(net.unlock_condition.as_deref()) {
            let msg = net.locked_message.map(|m| self.items.expand_text(&m)).unwrap_or_else(|| "The minecarts aren't running yet.".into());
            self.emit(WorldEvent::Message(msg));
            return true;
        }
        let choices: Vec<MinecartChoice> = net
            .destinations
            .iter()
            .filter(|d| Some(d.id.as_str()) != exclude && d.target_location != self.current && self.gsq_ok(d.condition.as_deref()))
            .map(|d| {
                let name = self.items.expand_text(&d.display_name);
                let name = if name.is_empty() { d.target_location.clone() } else { name };
                (d.id.clone(), if d.price > 0 { format!("{name} ({}g)", d.price) } else { name })
            })
            .collect();
        let prompt = net.choose_destination_message.map(|m| self.items.expand_text(&m)).unwrap_or_else(|| "Choose destination:".into());
        self.emit(WorldEvent::MinecartMenu { network: network.to_string(), prompt, choices });
        true
    }

    /// Ride to a destination of `network`.
    pub fn ride_minecart(&mut self, network: &str, destination: &str) -> bool {
        let Some(d) = self.minecarts.get(network).and_then(|n| n.destinations.iter().find(|d| d.id == destination)).cloned() else { return false };
        if self.farmer.money < d.price {
            self.emit(WorldEvent::Message(format!("You need {}g.", d.price)));
            return false;
        }
        if !self.locations.contains_key(&d.target_location) {
            return false;
        }
        self.farmer.money -= d.price;
        let facing = match d.target_direction.as_deref() {
            Some("up") => Dir::Up,
            Some("left") => Dir::Left,
            Some("right") => Dir::Right,
            _ => Dir::Down,
        };
        self.sound("minecartLoop");
        self.warp_farmer(&d.target_location, TilePos::new(d.target_tile.x, d.target_tile.y), Some(facing));
        true
    }

    /// Cooking show episode airing today (Sundays new, Wednesdays rerun of
    /// the same week's episode): (episode id, recipe name, text).
    pub fn cooking_episode(&self) -> Option<(String, String, String)> {
        if !matches!(self.clock.weekday(), "Sun" | "Wed") || self.cooking_channel.is_empty() {
            return None;
        }
        let mut ids: Vec<&String> = self.cooking_channel.keys().collect();
        ids.sort_by_key(|k| (k.parse::<i64>().unwrap_or(i64::MAX), (*k).clone()));
        // Count weeks from the most recent Sunday (Wednesday reruns it).
        let since_sunday = (self.clock.weekday_index() + 1) % 7;
        let last_sunday = self.clock.days_played() - since_sunday;
        let week = ((last_sunday - 1).max(0) / 7) as usize;
        let id = ids[week % ids.len()].clone();
        let raw = &self.cooking_channel[&id];
        let (recipe, text) = raw.split_once('/').unwrap_or((raw.as_str(), ""));
        Some((id, recipe.to_string(), self.items.expand_text(text)))
    }

    /// Channels showing today.
    pub fn tv_channels(&self) -> Vec<Channel> {
        let mut c = vec![Channel::Weather, Channel::Fortune];
        if self.cooking_episode().is_some() {
            c.push(Channel::Cooking);
        }
        c
    }

    /// The TV: pick a channel.
    pub fn watch_tv(&mut self) {
        self.sound("tvClick");
        let channels = self.tv_channels();
        self.emit(WorldEvent::TvMenu(channels));
    }

    pub fn watch_channel(&mut self, channel: Channel) {
        let text = match channel {
            Channel::Weather => {
                let forecast = match self.weather_tomorrow {
                    Weather::Sun => "Tomorrow looks clear and sunny.",
                    Weather::Rain => "Expect rain tomorrow. Your crops will thank you.",
                    Weather::Storm => "A thunderstorm is on the way tomorrow. Stay safe!",
                    Weather::Snow => "Snow is in the forecast for tomorrow.",
                    Weather::Wind => "It'll be a breezy day tomorrow.",
                };
                format!("Weather report: {forecast}")
            }
            Channel::Fortune => {
                let luck = match self.daily_luck {
                    l if l > 0.07 => "The spirits are very happy today. Great luck!",
                    l if l > 0.02 => "The spirits are in good humor today.",
                    l if l >= -0.02 => "The spirits feel neutral today.",
                    l if l >= -0.07 => "The spirits are somewhat annoyed today.",
                    _ => "The spirits are very displeased today. Be careful!",
                };
                format!("Fortune corner: {luck}")
            }
            Channel::Cooking => {
                let Some((_, recipe, text)) = self.cooking_episode() else { return };
                let mut msg = text;
                if self.cooking.contains_key(&recipe) {
                    if self.farmer.cooking_recipes.contains_key(&recipe) {
                        msg.push_str("\n\n(You already know this recipe.)");
                    } else {
                        self.farmer.cooking_recipes.insert(recipe.clone(), 0);
                        msg.push_str(&format!("\n\nYou learned how to make {recipe}!"));
                    }
                }
                msg
            }
        };
        self.emit(WorldEvent::Message(text));
    }
}

/// `Data/Minecarts` loader.
pub fn load(cm: &mut valley_content::ContentManager) -> BTreeMap<String, MinecartNetwork> {
    cm.load_json("Data/Minecarts").unwrap_or_default()
}
