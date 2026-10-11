use valley_content::ContentManager;
use valley_sim::evaluation::{candles_for, EVALUATED};
use valley_sim::farmer::{Skill, XP_LEVELS};
use valley_sim::item::Item;
use valley_sim::time::Season;
use valley_sim::world::{NewGame, World, WorldEvent};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

fn to_year_three(w: &mut World) {
    w.clock.year = 2;
    w.clock.season = Season::Winter;
    w.clock.day = 28;
    w.end_day(false);
    assert_eq!((w.clock.year, w.clock.season, w.clock.day), (3, Season::Spring, 1));
}

fn boost(w: &mut World) {
    w.farmer.total_earned = 1_200_000;
    for s in [Skill::Farming, Skill::Mining, Skill::Foraging, Skill::Fishing, Skill::Combat] {
        w.farmer.skills.gain(s, XP_LEVELS[9]);
    }
    for i in 0..10 {
        w.farmer.friendships.entry(format!("Friend{i}")).or_default().points = 2500;
    }
    w.farmer.mail_received.push("ccIsComplete".into());
    w.farmer.deepest_mine_level = 120;
    if let Some(p) = w.pet.as_mut() {
        p.friendship = 1000;
    }
}

#[test]
fn candle_thresholds() {
    assert_eq!([0, 3, 4, 7, 8, 11, 12, 21].map(candles_for), [1, 1, 2, 2, 3, 3, 4, 4]);
}

#[test]
fn grandpa_evaluates_at_the_start_of_year_three() {
    let mut w = world();
    w.end_day(false);
    assert!(!w.farmer.mail_received.iter().any(|m| m == EVALUATED), "not in year 1");
    to_year_three(&mut w);
    assert!(w.farmer.mail_received.iter().any(|m| m == EVALUATED));
    assert_eq!(w.farmer.grandpa_candles, 1, "a fresh farm scores low");
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("Grandpa"))));
    // Only once.
    w.end_day(false);
    assert!(!w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("Grandpa"))));
}

#[test]
fn a_great_farm_lights_four_candles() {
    let mut w = world();
    boost(&mut w);
    assert!(w.grandpa_score() >= 12, "score {} {:?}", w.grandpa_score(), w.grandpa_score_lines());
    to_year_three(&mut w);
    assert_eq!(w.farmer.grandpa_candles, 4);
}

#[test]
fn a_diamond_on_the_shrine_asks_for_a_new_evaluation() {
    let mut w = world();
    to_year_three(&mut w);
    assert_eq!(w.farmer.grandpa_candles, 1);
    boost(&mut w);
    let shrine = w.compat.shrine.expect("dev farm has a shrine");
    w.warp_farmer("Farm", valley_sim::TilePos::new(shrine.x, shrine.y + 1), Some(valley_sim::Dir::Up));
    w.drain_events();
    // Without a diamond: just a message.
    assert!(w.do_action(Some(shrine)));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("diamond"))));
    w.farmer.inventory.add(Item::object("72", 1));
    assert!(w.do_action(Some(shrine)));
    let choice = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Choice { options, .. } => Some(options),
        _ => None,
    });
    let options = choice.expect("asks to place the diamond");
    w.choose(&options[0].0);
    assert_eq!(w.farmer.inventory.count("72"), 0);
    w.end_day(false);
    assert_eq!(w.farmer.grandpa_candles, 4);
}

#[test]
fn joja_route_funds_areas_instead_of_bundles() {
    let mut w = world();
    w.farmer.money = 100_000;
    w.farmer.inventory.add(Item::object("Dev_Turnip", 5));
    w.drain_events();
    assert!(w.run_action_pub("Port.JojaForm"));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("members only"))));
    assert!(w.run_action_pub("Port.JojaMembership"));
    let opts = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Choice { options, .. } => Some(options),
        _ => None,
    });
    w.choose(&opts.expect("membership offer")[0].0);
    assert!(w.joja_member());
    assert_eq!(w.farmer.money, 95_000);
    assert!(w.donate_to_bundle("Garden/0", 0).is_err(), "bundles are closed");

    assert!(w.run_action_pub("Port.JojaForm"));
    let opts = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::Choice { options, .. } => Some(options),
        _ => None,
    });
    let opts = opts.expect("project list");
    assert_eq!(opts.len(), 2);
    w.choose("joja_project:Workshop");
    assert!(w.has_flag("ccDone_Workshop"));
    assert!(w.has_flag("jojaWorkshop"));
    assert!(!w.has_flag("ccIsComplete"));
    w.choose("joja_project:Garden");
    assert!(w.has_flag("ccIsComplete"));
    assert_eq!(w.farmer.money, 95_000 - 12_000 - 8_000);
    assert!(w.buy_joja_project("Garden").is_err(), "only once");
}

#[test]
fn original_area_names_set_their_well_known_flags() {
    assert_eq!(valley_sim::evaluation::area_flag("Boiler Room"), Some("ccBoilerRoom"));
    assert_eq!(valley_sim::evaluation::area_flag("Pantry"), Some("ccPantry"));
    let projects = valley_sim::evaluation::default_joja_projects();
    assert_eq!(projects.iter().map(|p| p.price).sum::<i32>(), 135_000);
}

#[test]
fn arcade_cabinets_open_games_and_record_scores() {
    let mut w = world();
    w.drain_events();
    assert!(w.run_action_pub("Arcade_Prairie"));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::OpenArcade(g) if g == "ranger")));
    assert!(w.run_action_pub("Arcade_Minecart"));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::OpenArcade(g) if g == "rails")));
    w.arcade_finished("rails", 900, false);
    w.arcade_finished("rails", 500, true);
    assert_eq!(w.farmer.counters["Arcade.rails.Best"], 900);
    assert_eq!(w.farmer.counters["Arcade.rails.Wins"], 1);
    assert!(w.farmer.achievements.iter().any(|a| a == "arcade_rails"));
}
