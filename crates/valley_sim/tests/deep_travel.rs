use valley_content::ContentManager;
use valley_sim::mines::{DEEP_START, MAX_LEVEL, SHAFT_DROP};
use valley_sim::objects::{ObjKind, PlacedObject};
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn deep_cavern_starts_past_the_mines_and_is_harder() {
    let mut w = world();
    w.enter_mine_level(10);
    let shallow: i32 = w.monsters.iter().map(|m| m.info.health).max().unwrap_or(0);
    w.enter_mine_level(DEEP_START);
    assert_eq!(w.mine_level(), Some(DEEP_START));
    assert!(w.location().display_name.starts_with("Deep Cavern"));
    let deep: i32 = w.monsters.iter().map(|m| m.info.health).max().unwrap_or(0);
    assert!(deep > shallow * 2, "deep {deep} vs shallow {shallow}");
    // The regular mines end at MAX_LEVEL: no ladder spawns there.
    assert!(MAX_LEVEL < DEEP_START);
}

#[test]
fn shaft_drops_several_floors_and_hurts() {
    let mut w = world();
    w.enter_mine_level(DEEP_START + 2);
    let t = w.farmer.tile();
    let below = TilePos::new(t.x, t.y + 1);
    w.location_mut().state.objects.insert(below, PlacedObject::new("Port_Shaft", false, ObjKind::Shaft));
    w.farmer.facing = Dir::Down;
    w.drain_events();
    assert!(w.do_action(Some(below)));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::ShaftPrompt)));
    let hp = w.farmer.health;
    w.jump_down_shaft();
    let lvl = w.mine_level().unwrap();
    let fell = lvl - (DEEP_START + 2);
    assert!((SHAFT_DROP.0..=SHAFT_DROP.1).contains(&fell), "fell {fell}");
    assert!(w.farmer.health < hp && w.farmer.health >= 1);
}

#[test]
fn mine_action_accepts_a_start_floor() {
    let mut w = world();
    w.warp_farmer("Dev_Dunes", TilePos::new(8, 4), Some(Dir::Up));
    assert!(w.do_action(Some(TilePos::new(8, 3))));
    assert_eq!(w.mine_level(), Some(DEEP_START));
    // The up ladder returns to where we came in.
    let up = w.location().state.objects.iter().find(|(_, o)| matches!(o.kind, ObjKind::Ladder { down: false })).map(|(p, _)| *p).unwrap();
    w.farmer.set_tile(TilePos::new(up.x, up.y + 1));
    assert!(w.do_action(Some(up)));
    assert_eq!(w.current, "Dev_Dunes");
}

#[test]
fn travel_costs_the_fare_takes_time_and_respects_hours() {
    let mut w = world();
    w.warp_farmer("Town", TilePos::new(37, 18), Some(Dir::Up));
    w.drain_events();
    // Too early: the service starts at 9am.
    assert!(w.do_action(Some(TilePos::new(37, 17))));
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Message(m) if m.contains("9:00"))));
    w.clock.time = 1000;
    assert!(w.do_action(Some(TilePos::new(37, 17))));
    let prompt = w.drain_events().into_iter().find_map(|e| match e {
        WorldEvent::TravelPrompt { location, tile, fare } => Some((location, tile, fare)),
        _ => None,
    });
    let (loc, tile, fare) = prompt.expect("travel offered");
    assert_eq!((loc.as_str(), fare), ("Dev_Dunes", 300));
    let money = w.farmer.money;
    assert!(w.travel(&loc, tile, fare));
    assert_eq!(w.current, "Dev_Dunes");
    assert_eq!(w.farmer.money, money - 300);
    assert_eq!(w.clock.time, 1030);
    // Broke: no trip.
    w.farmer.money = 10;
    assert!(!w.travel("Town", TilePos::new(36, 18), 300));
    assert_eq!(w.current, "Dev_Dunes");
    assert!(w.travel("Town", TilePos::new(36, 18), 0), "the ride back is free");
}
