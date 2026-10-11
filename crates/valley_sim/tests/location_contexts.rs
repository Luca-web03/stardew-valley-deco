use valley_content::{ContentManager, LayeredSource, MemSource};
use valley_sim::time::Weather;
use valley_sim::world::{NewGame, World};

#[test]
fn context_weather_conditions_decide_tomorrow() {
    let mut extra = MemSource::new("test");
    extra.insert_str(
        "Data/LocationContexts.json",
        r#"{ "Default": { "AllowRainTotem": false, "WeatherConditions": [
            { "Id": "odd", "Condition": "DAY_OF_MONTH odd", "Weather": "Rain" },
            { "Id": "rest", "Weather": "Sun" }
        ] } }"#,
    );
    let mut l = LayeredSource::default();
    l.push(Box::new(extra));
    l.push(Box::new(valley_content::dev::build()));
    let mut cm = ContentManager::new(Box::new(l));
    let mut w = World::new(&mut cm, NewGame::default());
    for _ in 0..6 {
        w.end_day(false);
        // Tomorrow's weather follows tomorrow's date.
        let tomorrow_odd = (w.clock.day % 28 + 1) % 2 == 1;
        let expect = if tomorrow_odd { Weather::Rain } else { Weather::Sun };
        assert_eq!(w.weather_tomorrow, expect, "day {}", w.clock.day);
    }
    assert!(!w.rain_totem_allowed());
}
