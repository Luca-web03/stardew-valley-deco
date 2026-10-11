//! Location contexts (`Data/LocationContexts`, public format subset): the
//! `Default` context's `WeatherConditions` decide tomorrow's weather (the
//! first entry whose `Condition` passes, checked against tomorrow's date),
//! and `AllowRainTotem` gates rain totems. A location's context is its map
//! property `LocationContext` (else `Default`). Without the asset the
//! engine's own seasonal weather chances apply.

use std::collections::BTreeMap;

use engine::Rng;
use serde::Deserialize;

use crate::gsq::{QueryCtx, QueryWorld};
use crate::time::Weather;
use crate::world::World;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct WeatherCondition {
    pub id: Option<String>,
    pub condition: Option<String>,
    pub weather: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct LocationContextData {
    pub season_override: Option<String>,
    pub default_music: Option<String>,
    pub weather_conditions: Vec<WeatherCondition>,
    pub copy_weather_from_location: Option<String>,
    pub allow_rain_totem: bool,
    pub max_pass_out_cost: i32,
}

impl Default for LocationContextData {
    fn default() -> Self {
        Self { season_override: None, default_music: None, weather_conditions: Vec::new(), copy_weather_from_location: None, allow_rain_totem: true, max_pass_out_cost: -1 }
    }
}

pub fn load(cm: &mut valley_content::ContentManager) -> BTreeMap<String, LocationContextData> {
    cm.load_dict("Data/LocationContexts")
}

pub fn parse_weather(s: &str) -> Option<Weather> {
    Some(match s.to_ascii_lowercase().as_str() {
        "sun" | "festival" | "wedding" => Weather::Sun,
        "rain" => Weather::Rain,
        "storm" => Weather::Storm,
        "snow" => Weather::Snow,
        "wind" => Weather::Wind,
        "greenrain" => Weather::GreenRain,
        _ => return None,
    })
}

impl World {
    /// The context of a location (map property `LocationContext`).
    pub fn location_context(&self, location: &str) -> String {
        self.locations.get(location).and_then(|l| l.map.prop("LocationContext")).map(|s| s.to_string()).unwrap_or_else(|| "Default".into())
    }

    /// Tomorrow's weather from the `Default` context's conditions, if the
    /// data defines any.
    pub(crate) fn data_weather_tomorrow(&self, rng: &mut Rng) -> Option<Weather> {
        let ctx_data = self.location_contexts.get("Default")?;
        if ctx_data.weather_conditions.is_empty() {
            return None;
        }
        let mut tomorrow = self.clock.clone();
        tomorrow.next_day();
        let mail = self.farmer.mail_received.clone();
        let seen = self.farmer.events_seen.clone();
        for c in &ctx_data.weather_conditions {
            let mut ctx = QueryCtx { clock: &tomorrow, weather: self.weather, mail: &mail, events_seen: &seen, location: "Farm", rng, world: Some(self) };
            if crate::gsq::check_with(c.condition.as_deref(), &mut ctx, QueryWorld::default()) {
                return parse_weather(&c.weather);
            }
        }
        None
    }

    /// Can a rain totem be used here?
    pub fn rain_totem_allowed(&self) -> bool {
        let c = self.location_context(&self.current);
        self.location_contexts.get(&c).map_or(true, |d| d.allow_rain_totem)
    }
}
