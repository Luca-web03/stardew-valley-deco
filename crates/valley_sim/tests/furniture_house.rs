use valley_content::ContentManager;
use valley_sim::item::Item;
use valley_sim::save::SaveData;
use valley_sim::world::{NewGame, World};
use valley_sim::{Dir, TilePos};

fn world() -> (ContentManager, World) {
    let mut cm = ContentManager::dev_only();
    let w = World::new(&mut cm, NewGame::default());
    (cm, w)
}

#[test]
fn furniture_places_blocks_and_picks_up() {
    let (_, mut w) = world();
    w.farmer.set_tile(TilePos::new(4, 4));
    w.farmer.facing = Dir::Down;
    w.farmer.inventory.slots[11] = Some(Item::Furniture { id: "Dev_Table".into() });
    w.farmer.current_slot = 11;
    assert!(w.do_action(None));
    assert_eq!(w.location().state.furniture.len(), 1);
    // A 2x1 table blocks both tiles.
    assert!(!valley_sim::collision::tile_passable(w.location(), TilePos::new(4, 5)));
    assert!(!valley_sim::collision::tile_passable(w.location(), TilePos::new(5, 5)));
    // Rugs don't block.
    w.farmer.inventory.slots[11] = Some(Item::Furniture { id: "Dev_Rug".into() });
    w.farmer.set_tile(TilePos::new(2, 6));
    w.farmer.facing = Dir::Down;
    assert!(w.do_action(None));
    assert!(valley_sim::collision::tile_passable(w.location(), TilePos::new(3, 7)));
    // Outdoors is refused.
    w.warp_farmer("Farm", TilePos::new(12, 18), Some(Dir::Down));
    w.farmer.inventory.slots[11] = Some(Item::Furniture { id: "Dev_Lamp".into() });
    w.do_action(None);
    assert!(w.locations["Farm"].state.furniture.is_empty());
    // Pick the table back up with empty hands.
    w.warp_farmer("FarmHouse", TilePos::new(4, 4), Some(Dir::Down));
    w.farmer.set_tile(TilePos::new(4, 4));
    w.farmer.facing = Dir::Down;
    w.farmer.current_slot = 30;
    assert!(w.do_action(None));
    assert!(w.farmer.inventory.slots.iter().flatten().any(|i| matches!(i, Item::Furniture { id } if id == "Dev_Table")));
}

#[test]
fn house_upgrade_swaps_map_and_survives_save() {
    let (mut cm, mut w) = world();
    assert!(w.start_house_upgrade().is_err());
    w.farmer.money = 20_000;
    w.farmer.inventory.add(Item::object("388", 450));
    w.start_house_upgrade().unwrap();
    assert_eq!(w.farmer.money, 10_000);
    let width0 = w.locations["FarmHouse"].width();
    w.end_day(false);
    w.end_day(false);
    assert_eq!(w.farmer.house_level, 0);
    w.end_day(false);
    assert_eq!(w.farmer.house_level, 1);
    assert!(w.locations["FarmHouse"].width() > width0);
    let r = SaveData::from_json(&SaveData::capture(&w).to_json()).unwrap().restore(&mut cm);
    assert_eq!(r.farmer.house_level, 1);
    assert_eq!(r.locations["FarmHouse"].width(), w.locations["FarmHouse"].width());
}
