use valley_content::ContentManager;
use valley_sim::minecarts::Channel;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn events(w: &mut World) -> Vec<WorldEvent> {
    w.drain_events()
}

#[test]
fn minecarts_are_locked_until_the_workshop_is_done() {
    let mut w = world();
    w.warp_farmer("Town", TilePos::new(7, 4), Some(Dir::Up));
    events(&mut w);
    assert!(w.do_action(Some(TilePos::new(7, 3))));
    assert!(events(&mut w).iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("out of order"))));
    w.farmer.mail_received.push("ccDone_Workshop".into());
    assert!(w.do_action(Some(TilePos::new(7, 3))));
    let menu = events(&mut w).into_iter().find_map(|e| match e {
        WorldEvent::MinecartMenu { network, choices, .. } => Some((network, choices)),
        _ => None,
    });
    let (network, choices) = menu.expect("destination menu");
    assert_eq!(network, "Default");
    let ids: Vec<&str> = choices.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["Farm", "Dunes"], "the current stop is left out");
    assert!(choices[1].1.contains("50g"));
    // Ride to the dunes (costs 50g) and arrive beside that minecart.
    let money = w.farmer.money;
    assert!(w.ride_minecart("Default", "Dunes"));
    assert_eq!((w.current.as_str(), w.farmer.tile()), ("Dev_Dunes", TilePos::new(14, 16)));
    assert_eq!(w.farmer.money, money - 50);
    w.farmer.money = 0;
    assert!(!w.ride_minecart("Default", "Dunes"), "can't afford it");
    assert!(w.ride_minecart("Default", "Farm"));
    assert_eq!(w.current, "Farm");
    assert!(valley_sim::collision::tile_passable(&w.locations["Farm"], w.farmer.tile()));
}

#[test]
fn tv_has_weather_and_fortune_daily_and_a_cooking_show_on_sundays() {
    let mut w = world();
    assert_eq!(w.clock.weekday(), "Mon");
    assert_eq!(w.tv_channels(), vec![Channel::Weather, Channel::Fortune]);
    w.watch_tv();
    assert!(events(&mut w).iter().any(|e| matches!(e, WorldEvent::TvMenu(c) if c.len() == 2)));
    w.watch_channel(Channel::Weather);
    assert!(events(&mut w).iter().any(|e| matches!(e, WorldEvent::Message(m) if m.starts_with("Weather report"))));
    while w.clock.weekday() != "Sun" {
        w.end_day(false);
    }
    assert!(w.tv_channels().contains(&Channel::Cooking));
    let (_, recipe, _) = w.cooking_episode().unwrap();
    assert_eq!(recipe, "Berry Tart");
    assert!(!w.farmer.cooking_recipes.contains_key("Berry Tart"), "only the show teaches it");
    w.watch_channel(Channel::Cooking);
    assert!(w.farmer.cooking_recipes.contains_key("Berry Tart"));
    // Wednesday is a rerun of the same episode; Monday has no show.
    w.end_day(false);
    assert!(w.cooking_episode().is_none());
    while w.clock.weekday() != "Wed" {
        w.end_day(false);
    }
    assert_eq!(w.cooking_episode().unwrap().1, "Berry Tart");
    while w.clock.weekday() != "Sun" {
        w.end_day(false);
    }
    assert_eq!(w.cooking_episode().unwrap().1, "Garden Salad", "the next week has the next episode");
}
