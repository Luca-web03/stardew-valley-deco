use valley_content::ContentManager;
use valley_sim::collision::tile_passable;
use valley_sim::item::Item;
use valley_sim::objects::ObjKind;
use valley_sim::save::SaveData;
use valley_sim::volcano::{volcano_floor, VOLCANO_FLOORS};
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

#[test]
fn shells_count_and_repair_the_bridge() {
    let (mut cm, mut w) = world();
    let bridge = TilePos::new(12, 8);
    assert!(!tile_passable(&w.locations["Dev_Isle"], bridge), "broken bridge blocks");
    // Collectibles go to a counter, not the inventory.
    w.give_item(Item::object("Dev_SunShell", 3));
    assert_eq!(w.counter("SunShell"), 3);
    assert_eq!(w.farmer.inventory.count("Dev_SunShell"), 0);
    w.warp_farmer("Dev_Isle", TilePos::new(11, 8), Some(Dir::Up));
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(11, 7))));
    assert!(!tile_passable(&w.locations["Dev_Isle"], bridge), "not enough shells yet");
    w.give_item(Item::object("Dev_SunShell", 2));
    assert!(w.do_action(Some(TilePos::new(11, 7))));
    assert_eq!(w.counter("SunShell"), 0);
    assert!(tile_passable(&w.locations["Dev_Isle"], bridge), "repaired");
    // Stays repaired after a save and load (maps come from content again).
    let json = SaveData::capture(&w).to_json();
    let r = SaveData::from_json(&json).unwrap().restore(&mut cm);
    assert!(tile_passable(&r.locations["Dev_Isle"], bridge));
    assert_eq!(r.counter("SunShell"), 0);
}

#[test]
fn boat_from_town_reaches_the_isle() {
    let (_, mut w) = world();
    w.warp_farmer("Town", TilePos::new(18, 24), Some(Dir::Down));
    w.clock.time = 900;
    w.drain_events();
    assert!(w.do_action(Some(TilePos::new(18, 25))));
    let trip = w.drain_events().into_iter().find_map(|e| match e {
        valley_sim::world::WorldEvent::TravelPrompt { location, tile, fare } => Some((location, tile, fare)),
        _ => None,
    });
    let (loc, tile, fare) = trip.expect("boat offered");
    assert!(w.travel(&loc, tile, fare));
    assert_eq!(w.current, "Dev_Isle");
}

#[test]
fn volcano_floors_lava_and_forge() {
    let (_, mut w) = world();
    w.warp_farmer("Dev_Isle", TilePos::new(20, 4), Some(Dir::Up));
    assert!(w.do_action(Some(TilePos::new(20, 3))));
    assert_eq!(volcano_floor(&w.current), Some(1));
    // Lava exists and burns.
    let lava = {
        let loc = w.location();
        (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).find(|p| loc.tile_prop("Back", *p, "Type").is_some_and(|v| v == "Lava"))
    };
    let lava = lava.expect("lava pools");
    w.farmer.set_tile(lava);
    assert!(w.on_lava());
    let hp = w.farmer.health;
    w.update(&valley_sim::input::PlayerInput::default());
    assert!(w.farmer.health < hp);
    // Walk the ladders down to the forge floor.
    for n in 2..=VOLCANO_FLOORS {
        let down = w.location().state.objects.iter().find(|(_, o)| matches!(o.kind, ObjKind::Ladder { down: true })).map(|(p, _)| *p).expect("way down");
        w.farmer.set_tile(TilePos::new(down.x, down.y + 1));
        w.monsters.clear();
        assert!(w.do_action(Some(down)));
        assert_eq!(volcano_floor(&w.current), Some(n));
    }
    let forge = {
        let loc = w.location();
        (0..loc.height()).flat_map(|y| (0..loc.width()).map(move |x| TilePos::new(x, y))).find(|p| loc.action_at(*p).is_some_and(|a| a == "Port.Forge"))
    };
    assert!(forge.is_some(), "forge on the last floor");
    // The forge opens its menu.
    w.drain_events();
    assert!(w.run_action_pub("Port.Forge"));
    assert!(w.drain_events().iter().any(|e| matches!(e, valley_sim::world::WorldEvent::OpenForge)));
    // The top ladder leads back out.
    w.enter_volcano(1);
    let up = w.location().state.objects.iter().find(|(_, o)| matches!(o.kind, ObjKind::Ladder { down: false })).map(|(p, _)| *p).unwrap();
    w.farmer.set_tile(TilePos::new(up.x, up.y + 1));
    w.monsters.clear();
    assert!(w.do_action(Some(up)));
    assert_eq!(w.current, "Dev_Isle");
    assert!(w.locations.keys().all(|k| volcano_floor(k).is_none()));
}
