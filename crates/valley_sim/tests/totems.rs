use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::world::{NewGame, World, WorldEvent};
use valley_sim::{Dir, TilePos};

fn world() -> World {
    let mut cm = ContentManager::dev_only();
    World::new(&mut cm, NewGame::default())
}

#[test]
fn warp_items_teleport_and_are_used_up() {
    let mut w = world();
    w.warp_farmer("Town", TilePos::new(20, 20), Some(Dir::Down));
    w.farmer.inventory.slots[7] = Some(Item::object("Dev_HomingStone", 2));
    w.farmer.current_slot = 7;
    w.drain_events();
    assert!(w.do_action(None));
    assert_eq!(w.current, "Farm");
    let entry = w.locations["Farm"].map_prop_tile("FarmHouseEntry").unwrap();
    assert_eq!(w.farmer.tile(), TilePos::new(entry.x, entry.y + 2));
    assert_eq!(w.farmer.inventory.count("Dev_HomingStone"), 1);
    assert!(w.drain_events().iter().any(|e| matches!(e, WorldEvent::Flash)));
    // Explicit coordinates in Port.WarpTo.
    w.farmer.inventory.slots[7] = Some(Item::object("Dev_DuneCharm", 1));
    assert!(w.do_action(None));
    assert_eq!((w.current.as_str(), w.farmer.tile()), ("Dev_Dunes", TilePos::new(10, 14)));
    assert!(w.farmer.inventory.get(7).is_none());
    // Well-known totem ids fall back to the location's arrival tile; missing
    // locations aren't consumed.
    assert_eq!(w.warp_totem_target("688").map(|(l, _)| l), Some("Farm".to_string()));
    assert!(w.warp_totem_target("Dev_Turnip").is_none());
}

#[test]
fn obelisk_building_runs_its_default_action() {
    let mut w = world();
    w.farmer.money = 50_000;
    w.farmer.inventory.add(Item::object("390", 300));
    w.farmer.inventory.add(Item::object("Dev_GoldBar", 5));
    {
        let farm = w.locations.get_mut("Farm").unwrap();
        for y in 16..22 {
            for x in 8..14 {
                farm.state.objects.remove(&TilePos::new(x, y));
                farm.state.terrain.remove(&TilePos::new(x, y));
            }
        }
    }
    w.place_building("Dev_Obelisk", TilePos::new(10, 17)).unwrap();
    w.end_day(false);
    w.end_day(false);
    assert_eq!(w.buildings[0].days_left, 0);
    w.warp_farmer("Farm", TilePos::new(10, 20), Some(Dir::Up));
    assert!(w.do_action(Some(TilePos::new(10, 19))));
    assert_eq!((w.current.as_str(), w.farmer.tile()), ("Dev_Dunes", TilePos::new(10, 14)));
}

#[test]
fn homing_stones_can_be_crafted() {
    let w = world();
    let r = &w.recipes["Homing Stone"];
    assert_eq!(r.output(), Item::object("Dev_HomingStone", 3));
}
